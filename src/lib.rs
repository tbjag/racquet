pub mod auth;
pub mod config;
pub mod connection;
pub mod db;
pub mod errors;
pub mod models;
pub mod routes;
pub mod ws;

use std::sync::Arc;
use connection::ConnectionManager;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub cm: Arc<ConnectionManager>,
    pub jwt_secret: String,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub frontend_url: String,
    pub allowed_emails: Vec<String>,
    pub test_mode: bool,
}

pub fn build_router(state: AppState) -> axum::Router {
    use axum::routing::{get, post};
    use tower_http::cors::{Any, CorsLayer};
    use tower_http::trace::TraceLayer;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut router = axum::Router::new()
        .route("/api/auth/google", get(routes::google_auth_redirect))
        .route(
            "/api/auth/google/callback",
            get(routes::google_auth_callback),
        )
        .route("/api/profile", get(routes::get_profile).put(routes::update_profile))
        .route("/api/rooms", get(routes::list_rooms).post(routes::create_room))
        .route("/api/rooms/{room_id}/messages", get(routes::get_messages))
        .route("/ws", get(ws::ws_handler));

    if state.test_mode {
        router = router.route("/api/auth/test-login", post(routes::test_login));
    }

    router
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}
