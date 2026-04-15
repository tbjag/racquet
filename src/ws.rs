use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::auth;
use crate::errors::AppError;
use crate::models;
use crate::AppState;

#[derive(Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    let token = query
        .token
        .ok_or_else(|| AppError::Unauthorized("missing token".to_string()))?;

    let claims = auth::verify_token(&token, &state.jwt_secret)?;
    tracing::info!(user_id = %claims.sub, username = %claims.username, "websocket upgrading");

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, claims)))
}

async fn handle_socket(socket: WebSocket, state: AppState, claims: auth::Claims) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let user_id = claims.sub;
    let username = claims.username;
    tracing::info!(user_id = %user_id, username = %username, "websocket connected");

    // Write task: forward messages from channel to WebSocket
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Read task: read from WebSocket and dispatch
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                handle_text_message(&text, &user_id, &username, &state, &tx).await;
            }
            Message::Close(_) => {
                tracing::debug!(user_id = %user_id, "received close frame");
                break;
            }
            _ => {}
        }
    }

    // Cleanup: disconnect user from all rooms
    tracing::info!(user_id = %user_id, username = %username, "websocket disconnected");
    state.cm.disconnect(&user_id).await;

    // Abort the write task
    write_task.abort();
}

async fn handle_text_message(
    text: &str,
    user_id: &str,
    username: &str,
    state: &AppState,
    sender: &mpsc::UnboundedSender<Message>,
) {
    let parsed: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            tracing::debug!("received invalid JSON from client");
            let _ = sender.send(Message::Text(
                serde_json::json!({ "type": "error", "message": "invalid JSON" })
                    .to_string()
                    .into(),
            ));
            return;
        }
    };

    let msg_type = parsed["type"].as_str().unwrap_or("");
    tracing::debug!(msg_type = %msg_type, user_id = %user_id, "received ws message");

    match msg_type {
        "join_room" => {
            if let Some(room_id) = parsed["room_id"].as_str() {
                state
                    .cm
                    .join_room(room_id, user_id, username, sender.clone())
                    .await;

                // Send room_users to the joining user
                let users = state.cm.get_room_users(room_id).await;
                let user_list: Vec<_> = users
                    .iter()
                    .filter(|(id, _)| id != user_id)
                    .map(|(id, name)| serde_json::json!({"user_id": id, "username": name}))
                    .collect();
                let response = serde_json::json!({
                    "type": "room_users",
                    "room_id": room_id,
                    "users": user_list,
                });
                let _ = sender.send(Message::Text(response.to_string().into()));
            }
        }
        "leave_room" => {
            if let Some(room_id) = parsed["room_id"].as_str() {
                state.cm.leave_room(room_id, user_id).await;
            }
        }
        "send_message" => {
            let room_id = match parsed["room_id"].as_str() {
                Some(id) => id,
                None => return,
            };
            let content = match parsed["content"].as_str() {
                Some(c) => c,
                None => return,
            };

            let msg_id = uuid::Uuid::new_v4().to_string();

            // Persist to database
            if let Err(e) = models::insert_message(&state.db, &msg_id, room_id, user_id, content).await {
                tracing::error!(error = %e, room_id = %room_id, user_id = %user_id, "failed to persist message");
                let _ = sender.send(Message::Text(
                    serde_json::json!({ "type": "error", "message": e.to_string() })
                        .to_string()
                        .into(),
                ));
                return;
            }

            // Fetch the created_at from DB for consistency
            let created_at = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

            let broadcast = serde_json::json!({
                "type": "new_message",
                "id": msg_id,
                "room_id": room_id,
                "user_id": user_id,
                "username": username,
                "content": content,
                "created_at": created_at,
            });

            state
                .cm
                .broadcast_to_room(room_id, &broadcast.to_string())
                .await;
        }
        "offer" | "answer" | "ice_candidate" | "call_leave" => {
            let room_id = match parsed["room_id"].as_str() {
                Some(id) => id,
                None => {
                    let _ = sender.send(Message::Text(
                        serde_json::json!({ "type": "error", "message": "missing room_id" }).to_string().into(),
                    ));
                    return;
                }
            };
            let target_user_id = match parsed["target_user_id"].as_str() {
                Some(id) => id,
                None => {
                    let _ = sender.send(Message::Text(
                        serde_json::json!({ "type": "error", "message": "missing target_user_id" }).to_string().into(),
                    ));
                    return;
                }
            };
            let payload = &parsed["payload"];
            if payload.is_null() {
                let _ = sender.send(Message::Text(
                    serde_json::json!({ "type": "error", "message": "missing payload" }).to_string().into(),
                ));
                return;
            }

            let relay = serde_json::json!({
                "type": msg_type,
                "room_id": room_id,
                "from_user_id": user_id,
                "from_username": username,
                "payload": payload,
            });

            let sent = state.cm.send_to_user(room_id, target_user_id, &relay.to_string()).await;
            if !sent {
                let _ = sender.send(Message::Text(
                    serde_json::json!({
                        "type": "error",
                        "message": format!("user {} not found in room", target_user_id)
                    }).to_string().into(),
                ));
            }
        }
        _ => {
            tracing::debug!(msg_type = %msg_type, "unknown message type received");
            let _ = sender.send(Message::Text(
                serde_json::json!({ "type": "error", "message": format!("unknown message type: {msg_type}") })
                    .to_string()
                    .into(),
            ));
        }
    }
}
