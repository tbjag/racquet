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
    screen_sharers: RwLock<HashMap<RoomId, UserId>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            rooms: RwLock::new(HashMap::new()),
            screen_sharers: RwLock::new(HashMap::new()),
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

        // Collect existing users list before notifying (excludes the joining user)
        let existing_users: Vec<serde_json::Value> = room
            .iter()
            .map(|(id, user)| {
                serde_json::json!({ "user_id": id, "username": user.username })
            })
            .collect();

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

        // Send room_users list to the joining user
        let room_users_msg = serde_json::json!({
            "type": "room_users",
            "room_id": room_id,
            "users": existing_users,
        });
        let _ = sender.send(axum::extract::ws::Message::Text(room_users_msg.to_string().into()));

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

        // Release any active screen shares this user held in these rooms.
        let mut released_in: Vec<String> = Vec::new();
        {
            let mut sharers = self.screen_sharers.write().await;
            for room_id in &room_ids {
                if sharers.get(room_id).map(|s| s.as_str()) == Some(user_id) {
                    sharers.remove(room_id);
                    released_in.push(room_id.clone());
                }
            }
        }

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

                    if released_in.contains(room_id) {
                        let stop = serde_json::json!({
                            "type": "screen_share_stopped",
                            "room_id": room_id,
                            "user_id": user_id,
                        });
                        let stop_msg = axum::extract::ws::Message::Text(stop.to_string().into());
                        for user in room.values() {
                            let _ = user.sender.send(stop_msg.clone());
                        }
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

    pub async fn send_to_user(&self, room_id: &str, target_user_id: &str, message: &str) -> bool {
        let rooms = self.rooms.read().await;
        if let Some(room) = rooms.get(room_id) {
            if let Some(user) = room.get(target_user_id) {
                let msg = axum::extract::ws::Message::Text(message.to_string().into());
                let _ = user.sender.send(msg);
                return true;
            }
        }
        false
    }

    /// Returns true when the share was acquired (or re-asserted by the same user).
    /// Returns false if a different user is currently sharing in this room.
    pub async fn try_acquire_screen_share(&self, room_id: &str, user_id: &str) -> bool {
        let mut sharers = self.screen_sharers.write().await;
        match sharers.get(room_id) {
            Some(existing) if existing != user_id => false,
            _ => {
                sharers.insert(room_id.to_string(), user_id.to_string());
                true
            }
        }
    }

    /// Returns true if `user_id` was the active sharer and was released.
    pub async fn release_screen_share(&self, room_id: &str, user_id: &str) -> bool {
        let mut sharers = self.screen_sharers.write().await;
        if sharers.get(room_id).map(|s| s.as_str()) == Some(user_id) {
            sharers.remove(room_id);
            true
        } else {
            false
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

        cm.join_room("room-1", "user-1", "alice", tx1).await;
        cm.join_room("room-2", "user-1", "alice", tx2).await;

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
        // Drain room_users sent to alice
        let _ = rx1.recv().await;
        cm.join_room("room-1", "user-2", "bob", tx2).await;
        // Drain room_users sent to bob
        let _ = rx2.recv().await;
        // Drain user_joined notification that bob's join sent to alice
        let _ = rx1.recv().await;

        let msg = r#"{"type":"new_message","content":"hello"}"#;
        cm.broadcast_to_room("room-1", msg).await;

        let received1 = rx1.recv().await.expect("user-1 should receive message");
        let received2 = rx2.recv().await.expect("user-2 should receive message");

        assert_eq!(received1.into_text().unwrap(), msg);
        assert_eq!(received2.into_text().unwrap(), msg);
    }

    #[tokio::test]
    async fn test_send_to_user_delivers_to_target() {
        let cm = ConnectionManager::new();
        let (tx1, mut rx1) = make_user_channel();
        let (tx2, mut rx2) = make_user_channel();

        cm.join_room("room-1", "user-1", "alice", tx1).await;
        // Drain room_users sent to alice
        let _ = rx1.recv().await;
        cm.join_room("room-1", "user-2", "bob", tx2).await;
        // Drain room_users sent to bob
        let _ = rx2.recv().await;
        // Drain user_joined notification that bob's join sent to alice
        let _ = rx1.recv().await;

        let msg = r#"{"type":"offer","payload":"test"}"#;
        let sent = cm.send_to_user("room-1", "user-2", msg).await;
        assert!(sent, "should return true when user exists");

        let received = rx2.recv().await.expect("user-2 should receive the message");
        assert_eq!(received.into_text().unwrap(), msg);

        // user-1 should NOT receive it
        let nothing = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            rx1.recv(),
        ).await;
        assert!(nothing.is_err(), "user-1 should not receive a targeted message for user-2");
    }

    #[tokio::test]
    async fn test_send_to_user_returns_false_for_missing_user() {
        let cm = ConnectionManager::new();
        let (tx1, _rx1) = make_user_channel();
        cm.join_room("room-1", "user-1", "alice", tx1).await;

        let sent = cm.send_to_user("room-1", "user-99", "msg").await;
        assert!(!sent, "should return false when target user not in room");
    }

    #[tokio::test]
    async fn test_send_to_user_returns_false_for_missing_room() {
        let cm = ConnectionManager::new();
        let sent = cm.send_to_user("nonexistent-room", "user-1", "msg").await;
        assert!(!sent, "should return false when room doesn't exist");
    }

    #[tokio::test]
    async fn test_join_room_sends_user_joined() {
        let cm = ConnectionManager::new();
        let (tx1, mut rx1) = make_user_channel();
        let (tx2, _rx2) = make_user_channel();

        cm.join_room("room-1", "user-1", "alice", tx1).await;
        // Drain room_users sent to alice (empty list)
        let _ = rx1.recv().await;

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
