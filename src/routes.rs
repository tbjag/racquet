use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::Json;
use serde::Deserialize;

use crate::auth::{self, AuthUser};
use crate::errors::AppError;
use crate::models;
use crate::AppState;

// --- Google OAuth ---

pub async fn google_auth_redirect(State(state): State<AppState>) -> impl IntoResponse {
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
         client_id={}&\
         redirect_uri={}&\
         response_type=code&\
         scope=openid%20email%20profile&\
         access_type=offline",
        urlencoding::encode(&state.google_client_id),
        urlencoding::encode(&state.google_redirect_uri),
    );
    Redirect::temporary(&url)
}

#[derive(Deserialize)]
pub struct GoogleCallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: Option<String>,
    sub: String,
}

pub async fn google_auth_callback(
    State(state): State<AppState>,
    Query(query): Query<GoogleCallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(error) = query.error {
        tracing::warn!(error = %error, "Google OAuth error");
        return Ok(Redirect::temporary(&format!(
            "{}/login?error=oauth_error",
            state.frontend_url
        )));
    }

    let code = query.code.ok_or_else(|| {
        AppError::BadRequest("missing authorization code".to_string())
    })?;

    // Exchange code for access token
    let client = reqwest::Client::new();
    let token_res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", state.google_client_id.as_str()),
            ("client_secret", state.google_client_secret.as_str()),
            ("code", &code),
            ("redirect_uri", state.google_redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("token exchange failed: {e}")))?;

    if !token_res.status().is_success() {
        let body = token_res.text().await.unwrap_or_default();
        tracing::error!(body = %body, "Google token exchange failed");
        return Ok(Redirect::temporary(&format!(
            "{}/login?error=oauth_error",
            state.frontend_url
        )));
    }

    let token_data: GoogleTokenResponse = token_res
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse token response: {e}")))?;

    // Fetch user info
    let userinfo: GoogleUserInfo = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token_data.access_token)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("userinfo request failed: {e}")))?
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("failed to parse userinfo: {e}")))?;

    let email = userinfo.email.to_lowercase();

    // Check whitelist
    if !state.allowed_emails.contains(&email) {
        tracing::warn!(email = %email, "login denied: email not in whitelist");
        return Ok(Redirect::temporary(&format!(
            "{}/login?error=not_allowed",
            state.frontend_url
        )));
    }

    // Create or update user
    let username = userinfo
        .name
        .unwrap_or_else(|| email.split('@').next().unwrap_or("user").to_string());
    let id = uuid::Uuid::new_v4().to_string();
    let user = models::upsert_user_by_email(&state.db, &id, &email, &username, Some(&userinfo.sub))
        .await
        .map_err(|e| AppError::Internal(format!("failed to upsert user: {e}")))?;

    let token = auth::create_token(&user.id, &user.username, &user.email, &state.jwt_secret)?;
    tracing::info!(user_id = %user.id, email = %user.email, "user logged in via Google");

    Ok(Redirect::temporary(&format!(
        "{}/auth/callback?token={}",
        state.frontend_url, token
    )))
}

// --- Test-only login (enabled by RACQUET_TEST_MODE) ---

#[derive(Deserialize)]
pub struct TestLoginRequest {
    email: String,
}

pub async fn test_login(
    State(state): State<AppState>,
    Json(req): Json<TestLoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let email = req.email.to_lowercase();
    let username = email.split('@').next().unwrap_or("user").to_string();
    let id = uuid::Uuid::new_v4().to_string();

    let user = models::upsert_user_by_email(&state.db, &id, &email, &username, None)
        .await
        .map_err(|e| AppError::Internal(format!("failed to upsert user: {e}")))?;

    let token = auth::create_token(&user.id, &user.username, &user.email, &state.jwt_secret)?;
    tracing::info!(user_id = %user.id, email = %email, "test login");

    Ok(Json(serde_json::json!({ "token": token, "user_id": user.id })))
}

// --- Profile ---

pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let db_user = models::find_user_by_id(&state.db, &user.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    Ok(Json(serde_json::json!({
        "id": db_user.id,
        "email": db_user.email,
        "username": db_user.username,
    })))
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    username: String,
}

pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let username = req.username.trim().to_string();
    if username.is_empty() || username.len() > 30 {
        return Err(AppError::BadRequest(
            "display name must be 1-30 characters".to_string(),
        ));
    }

    models::update_username(&state.db, &user.user_id, &username).await?;

    let token = auth::create_token(&user.user_id, &username, &user.email, &state.jwt_secret)?;
    tracing::info!(user_id = %user.user_id, username = %username, "profile updated");

    Ok(Json(serde_json::json!({
        "token": token,
        "user": {
            "id": user.user_id,
            "email": user.email,
            "username": username,
        }
    })))
}

// --- Rooms ---

pub async fn list_rooms(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let rooms = models::list_rooms(&state.db).await?;
    tracing::debug!(count = rooms.len(), "rooms listed");
    Ok(Json(serde_json::json!(rooms)))
}

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    name: String,
}

pub async fn create_room(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateRoomRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let existing = models::list_rooms(&state.db).await?;
    if existing.iter().any(|r| r.name == req.name) {
        return Err(AppError::Conflict("room name already taken".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    models::insert_room(&state.db, &id, &req.name, &user.user_id).await?;
    tracing::info!(room_id = %id, room_name = %req.name, created_by = %user.user_id, "room created");

    let room = models::find_room_by_id(&state.db, &id)
        .await?
        .ok_or_else(|| AppError::Internal("failed to fetch created room".to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": room.id,
            "name": room.name,
            "created_by": room.created_by,
            "created_at": room.created_at,
        })),
    ))
}

#[derive(Deserialize)]
pub struct MessagesQuery {
    before: Option<String>,
    limit: Option<i64>,
}

pub async fn get_messages(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(room_id): Path<String>,
    Query(query): Query<MessagesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let limit = query.limit.unwrap_or(50).min(100).max(1);
    let messages = models::get_messages(
        &state.db,
        &room_id,
        query.before.as_deref(),
        limit,
    )
    .await?;
    tracing::debug!(room_id = %room_id, count = messages.len(), limit, "messages fetched");

    Ok(Json(serde_json::json!(messages)))
}
