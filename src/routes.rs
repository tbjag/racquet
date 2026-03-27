use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::auth::{self, AuthUser};
use crate::errors::AppError;
use crate::models;
use crate::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    // Validate input
    if req.username.is_empty() || req.username.len() < 3 || req.username.len() > 20 {
        return Err(AppError::BadRequest("username must be 3-20 characters".to_string()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".to_string()));
    }

    // Check for duplicate username
    if models::find_user_by_username(&state.db, &req.username)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("username already taken".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = auth::hash_password(&req.password)?;

    models::insert_user(&state.db, &id, &req.username, &password_hash).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "username": req.username,
        })),
    ))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user = models::find_user_by_username(&state.db, &req.username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("invalid credentials".to_string()))?;

    let valid = auth::verify_password(&req.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::Unauthorized("invalid credentials".to_string()));
    }

    let token = auth::create_token(&user.id, &user.username, &state.jwt_secret)?;

    Ok(Json(serde_json::json!({ "token": token })))
}

pub async fn list_rooms(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let rooms = models::list_rooms(&state.db).await?;
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
    // Check for duplicate room name
    let existing = models::list_rooms(&state.db).await?;
    if existing.iter().any(|r| r.name == req.name) {
        return Err(AppError::Conflict("room name already taken".to_string()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    models::insert_room(&state.db, &id, &req.name, &user.user_id).await?;

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

    Ok(Json(serde_json::json!(messages)))
}
