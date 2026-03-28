use std::collections::HashMap;
use tokio::sync::{mpsc, RwLock};

pub type UserId = String;
pub type RoomId = String;

pub struct ConnectedUser {
    pub username: String,
    pub sender: mpsc::UnboundedSender<axum::extract::ws::Message>,
}

pub struct ConnectionManager {
    rooms: RwLock<HashMap<RoomId, HashMap<UserId, ConnectedUser>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
        }
    }

    pub async fn join_room(
        &self,
        room_id: &str,
        user_id: &str,
        username: &str,
        sender: mpsc::UnboundedSender<axum::extract::ws::Message>,
    ) {
        let mut rooms = self.rooms.write().await;
        let room = rooms.entry(room_id.to_string()).or_insert_with(HashMap::new);

        // Notify existing users in the room
        let notification = serde_json::json!({
            "type": "user_joined",
            "room_id": room_id,
            "user_id": user_id,
            "username": username,
        });
        let msg = axum::extract::ws::Message::Text(notification.to_string().into());
        for existing_user in room.values() {
            let _ = existing_user.sender.send(msg.clone());
        }

        room.insert(
            user_id.to_string(),
            ConnectedUser {
                username: username.to_string(),
                sender,
            },
        );
        tracing::info!(room_id = %room_id, user_id = %user_id, username = %username, "user joined room");
    }

    pub async fn leave_room(&self, room_id: &str, user_id: &str) {
        let mut rooms = self.rooms.write().await;
        let username = if let Some(room) = rooms.get_mut(room_id) {
            if let Some(user) = room.remove(user_id) {
                Some(user.username)
            } else {
                None
            }
        } else {
            None
        };

        // Notify remaining users
        if let Some(ref username) = username {
            tracing::info!(room_id = %room_id, user_id = %user_id, username = %username, "user left room");
        }
        if let Some(username) = username {
            if let Some(room) = rooms.get(room_id) {
                let notification = serde_json::json!({
                    "type": "user_left",
                    "room_id": room_id,
                    "user_id": user_id,
                    "username": username,
                });
                let msg = axum::extract::ws::Message::Text(notification.to_string().into());
                for user in room.values() {
                    let _ = user.sender.send(msg.clone());
                }
            }
        }
    }

    pub async fn disconnect(&self, user_id: &str) {
        let mut rooms = self.rooms.write().await;
        let room_ids: Vec<String> = rooms
            .iter()
            .filter(|(_, users)| users.contains_key(user_id))
            .map(|(room_id, _)| room_id.clone())
            .collect();

        tracing::info!(user_id = %user_id, room_count = room_ids.len(), "disconnecting user from all rooms");

        for room_id in &room_ids {
            let username = if let Some(room) = rooms.get_mut(room_id) {
                room.remove(user_id).map(|u| u.username)
            } else {
                None
            };

            if let Some(username) = username {
                if let Some(room) = rooms.get(room_id) {
                    let notification = serde_json::json!({
                        "type": "user_left",
                        "room_id": room_id,
                        "user_id": user_id,
                        "username": username,
                    });
                    let msg = axum::extract::ws::Message::Text(notification.to_string().into());
                    for user in room.values() {
                        let _ = user.sender.send(msg.clone());
                    }
                }
            }
        }
    }

    pub async fn broadcast_to_room(&self, room_id: &str, message: &str) {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(room_id) {
            tracing::debug!(room_id = %room_id, recipients = room.len(), "broadcasting to room");
            let msg = axum::extract::ws::Message::Text(message.to_string().into());
            for user in room.values() {
                let _ = user.sender.send(msg.clone());
            }
        }
    }

    pub async fn get_room_users(&self, room_id: &str) -> Vec<(UserId, String)> {
        let rooms = self.rooms.read().await;
        rooms
            .get(room_id)
            .map(|room| {
                room.iter()
                    .map(|(id, user)| (id.clone(), user.username.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a connected user's sender/receiver pair
    fn make_user_channel() -> (mpsc::UnboundedSender<axum::extract::ws::Message>, mpsc::UnboundedReceiver<axum::extract::ws::Message>) {
        mpsc::unbounded_channel()
    }

    #[tokio::test]
    async fn test_join_room() {
        let cm = ConnectionManager::new();
        let (tx, _rx) = make_user_channel();

        cm.join_room("room-1", "user-1", "alice", tx).await;

        let users = cm.get_room_users("room-1").await;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].0, "user-1");
        assert_eq!(users[0].1, "alice");
    }

    #[tokio::test]
    async fn test_leave_room() {
        let cm = ConnectionManager::new();
        let (tx, _rx) = make_user_channel();

        cm.join_room("room-1", "user-1", "alice", tx).await;
        cm.leave_room("room-1", "user-1").await;

        let users = cm.get_room_users("room-1").await;
        assert!(users.is_empty(), "user should be removed after leaving");
    }

    #[tokio::test]
    async fn test_disconnect_removes_from_all_rooms() {
        let cm = ConnectionManager::new();
        let (tx1, _rx1) = make_user_channel();
        let (tx2, _rx2) = make_user_channel();

        // User joins two different rooms (needs separate senders)
        cm.join_room("room-1", "user-1", "alice", tx1).await;
        cm.join_room("room-2", "user-1", "alice", tx2).await;

        // Disconnect should remove from both
        cm.disconnect("user-1").await;

        let users_room1 = cm.get_room_users("room-1").await;
        let users_room2 = cm.get_room_users("room-2").await;
        assert!(users_room1.is_empty(), "user should be removed from room-1");
        assert!(users_room2.is_empty(), "user should be removed from room-2");
    }

    #[tokio::test]
    async fn test_broadcast_to_room() {
        let cm = ConnectionManager::new();
        let (tx1, mut rx1) = make_user_channel();
        let (tx2, mut rx2) = make_user_channel();

        cm.join_room("room-1", "user-1", "alice", tx1).await;
        cm.join_room("room-1", "user-2", "bob", tx2).await;

        // Drain the user_joined notification that bob's join sent to alice
        let _ = rx1.recv().await;

        let msg = r#"{"type":"new_message","content":"hello"}"#;
        cm.broadcast_to_room("room-1", msg).await;

        // Both users should receive the message
        let received1 = rx1.recv().await.expect("user-1 should receive message");
        let received2 = rx2.recv().await.expect("user-2 should receive message");

        assert_eq!(received1.into_text().unwrap(), msg);
        assert_eq!(received2.into_text().unwrap(), msg);
    }

    #[tokio::test]
    async fn test_join_room_sends_user_joined() {
        let cm = ConnectionManager::new();
        let (tx1, mut rx1) = make_user_channel();
        let (tx2, _rx2) = make_user_channel();

        // Alice joins first
        cm.join_room("room-1", "user-1", "alice", tx1).await;

        // Bob joins — alice should receive a user_joined notification
        cm.join_room("room-1", "user-2", "bob", tx2).await;

        let received = rx1.recv().await.expect("alice should receive user_joined notification");
        let text = received.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("should be valid JSON");

        assert_eq!(parsed["type"], "user_joined");
        assert_eq!(parsed["user_id"], "user-2");
        assert_eq!(parsed["username"], "bob");
        assert_eq!(parsed["room_id"], "room-1");
    }
}
