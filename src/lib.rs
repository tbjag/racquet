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
}

pub fn build_router(state: AppState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/api/register", post(routes::register))
        .route("/api/login", post(routes::login))
        .route("/api/rooms", get(routes::list_rooms).post(routes::create_room))
        .route("/api/rooms/{room_id}/messages", get(routes::get_messages))
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
}
