use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to create a text WS message from a JSON value
fn text_msg(v: Value) -> Message {
    Message::Text(v.to_string().into())
}

/// Spawns the app on a random port with a temporary SQLite database.
/// Returns the base URL (e.g., "http://127.0.0.1:12345").
async fn spawn_app() -> String {
    let db_url = format!("sqlite:/tmp/racquet-test-{}.db?mode=rwc", uuid::Uuid::new_v4());

    let pool = racquet::db::create_pool(&db_url).await;
    sqlx::migrate!().run(&pool).await.expect("migrations failed");

    let state = racquet::AppState {
        db: pool,
        cm: std::sync::Arc::new(racquet::connection::ConnectionManager::new()),
        jwt_secret: "test-secret".to_string(),
    };

    let app = racquet::build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

/// Helper: register a user and return the response body
async fn register_user(base: &str, username: &str, password: &str) -> reqwest::Response {
    reqwest::Client::new()
        .post(format!("{base}/api/register"))
        .json(&json!({ "username": username, "password": password }))
        .send()
        .await
        .expect("register request should succeed")
}

/// Helper: login and return the JWT token string
async fn login_user(base: &str, username: &str, password: &str) -> String {
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/login"))
        .json(&json!({ "username": username, "password": password }))
        .send()
        .await
        .expect("login request should succeed");

    let body: Value = resp.json().await.expect("login response should be JSON");
    body["token"].as_str().expect("token field should exist").to_string()
}

/// Helper: register a user and return their user ID
async fn register_and_get_id(base: &str, username: &str, password: &str) -> String {
    let resp = register_user(base, username, password).await;
    let body: Value = resp.json().await.expect("register response should be JSON");
    body["id"].as_str().expect("id field should exist").to_string()
}

/// Helper: drain all pending WebSocket messages with a short timeout
async fn drain_ws<S>(ws: &mut S)
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while tokio::time::timeout(std::time::Duration::from_millis(100), ws.next())
        .await
        .is_ok()
    {}
}

/// Helper: receive the next text WS message, parsed as JSON, with a timeout
async fn recv_json<S>(ws: &mut S) -> Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next())
        .await
        .expect("should receive within timeout")
        .expect("stream should not end")
        .expect("should be a valid message");
    serde_json::from_str(&msg.into_text().unwrap()).unwrap()
}

/// Helper: create a room and return the response body as JSON
async fn create_room(base: &str, token: &str, name: &str) -> Value {
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/rooms"))
        .bearer_auth(token)
        .json(&json!({ "name": name }))
        .send()
        .await
        .expect("create room request should succeed");

    resp.json().await.expect("create room response should be JSON")
}

// ============================================================
// Auth tests
// ============================================================

#[tokio::test]
async fn test_register_success() {
    let base = spawn_app().await;

    let resp = register_user(&base, "alice", "password123").await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "alice");
    assert!(body["id"].as_str().is_some(), "response should include an id");
}

#[tokio::test]
async fn test_register_duplicate_username() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let resp = register_user(&base, "alice", "differentpassword").await;

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_register_invalid_input() {
    let base = spawn_app().await;

    // Empty username
    let resp = register_user(&base, "", "password123").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Password too short (< 8 chars)
    let resp = register_user(&base, "bob", "short").await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_login_success() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/login"))
        .json(&json!({ "username": "alice", "password": "password123" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().is_some(), "response should include a token");
}

#[tokio::test]
async fn test_login_wrong_password() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/login"))
        .json(&json!({ "username": "alice", "password": "wrongpassword" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_login_nonexistent_user() {
    let base = spawn_app().await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/login"))
        .json(&json!({ "username": "nobody", "password": "password123" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// ============================================================
// Room tests
// ============================================================

#[tokio::test]
async fn test_create_room() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/rooms"))
        .bearer_auth(&token)
        .json(&json!({ "name": "general" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "general");
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
async fn test_create_room_no_auth() {
    let base = spawn_app().await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/rooms"))
        .json(&json!({ "name": "general" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_duplicate_room() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;

    create_room(&base, &token, "general").await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/rooms"))
        .bearer_auth(&token)
        .json(&json!({ "name": "general" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_list_rooms() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;

    create_room(&base, &token, "general").await;
    create_room(&base, &token, "random").await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    let rooms = body.as_array().expect("response should be an array");
    assert_eq!(rooms.len(), 2);
}

#[tokio::test]
async fn test_list_rooms_empty() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    let rooms = body.as_array().expect("response should be an array");
    assert!(rooms.is_empty());
}

// ============================================================
// Message tests
// ============================================================

#[tokio::test]
async fn test_get_messages_empty() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms/{room_id}/messages"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    let messages = body.as_array().expect("response should be an array");
    assert!(messages.is_empty());
}

#[tokio::test]
async fn test_get_messages_pagination() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    // Send 60 messages via WebSocket
    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .expect("ws connect should succeed");

    // Join the room
    ws.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Drain room_users message
    let _ = ws.next().await;

    // Send 60 messages
    for i in 0..60 {
        ws.send(text_msg(json!({
            "type": "send_message",
            "room_id": room_id,
            "content": format!("message {i}")
        })))
        .await
        .unwrap();

        // Consume the broadcast echo for each message
        let _ = ws.next().await;
    }

    // Fetch first page (default limit 50)
    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms/{room_id}/messages"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let messages = body.as_array().expect("response should be an array");
    assert_eq!(messages.len(), 50, "default limit should be 50");

    // Use the last message's id as cursor for next page
    let last_id = messages.last().unwrap()["id"].as_str().unwrap();
    let resp = reqwest::Client::new()
        .get(format!(
            "{base}/api/rooms/{room_id}/messages?before={last_id}&limit=50"
        ))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let messages = body.as_array().expect("response should be an array");
    assert_eq!(messages.len(), 10, "second page should have remaining 10 messages");
}

// ============================================================
// WebSocket tests
// ============================================================

#[tokio::test]
async fn test_ws_connect_with_valid_token() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;

    let ws_url = base.replace("http://", "ws://");
    let result = connect_async(format!("{ws_url}/ws?token={token}")).await;

    assert!(result.is_ok(), "should connect with valid token");
}

#[tokio::test]
async fn test_ws_connect_without_token() {
    let base = spawn_app().await;

    let ws_url = base.replace("http://", "ws://");
    let result = connect_async(format!("{ws_url}/ws")).await;

    assert!(result.is_err(), "should fail without token");
}

#[tokio::test]
async fn test_ws_join_and_receive_message() {
    let base = spawn_app().await;

    // Register two users
    register_user(&base, "alice", "password123").await;
    register_user(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");

    // Connect both users
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    // Both join the room
    ws_a.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    ws_b.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Drain join notifications (user_joined messages)
    // Alice receives: bob's user_joined
    // Bob receives: nothing extra (alice joined before bob)
    // Give a moment for messages to propagate
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    while let Ok(msg) = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        ws_a.next(),
    )
    .await
    {
        // drain
        let _ = msg;
    }
    while let Ok(msg) = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        ws_b.next(),
    )
    .await
    {
        let _ = msg;
    }

    // Alice sends a message
    ws_a.send(text_msg(json!({
        "type": "send_message",
        "room_id": room_id,
        "content": "hello from alice"
    })))
    .await
    .unwrap();

    // Both should receive the new_message
    let msg_a = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        ws_a.next(),
    )
    .await
    .expect("alice should receive within timeout")
    .expect("stream should not end")
    .expect("should be a valid message");

    let parsed_a: Value = serde_json::from_str(&msg_a.into_text().unwrap()).unwrap();
    assert_eq!(parsed_a["type"], "new_message");
    assert_eq!(parsed_a["content"], "hello from alice");

    let msg_b = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        ws_b.next(),
    )
    .await
    .expect("bob should receive within timeout")
    .expect("stream should not end")
    .expect("should be a valid message");

    let parsed_b: Value = serde_json::from_str(&msg_b.into_text().unwrap()).unwrap();
    assert_eq!(parsed_b["type"], "new_message");
    assert_eq!(parsed_b["content"], "hello from alice");
}

#[tokio::test]
async fn test_ws_user_joined_notification() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    register_user(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");

    // Alice connects and joins the room
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    ws_a.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Drain alice's room_users message
    let _ = recv_json(&mut ws_a).await;

    // Bob connects and joins the room
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Alice should receive a user_joined notification for bob
    let parsed = recv_json(&mut ws_a).await;
    assert_eq!(parsed["type"], "user_joined");
    assert_eq!(parsed["username"], "bob");
    assert_eq!(parsed["room_id"], room_id);
}

#[tokio::test]
async fn test_ws_user_left_notification() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    register_user(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");

    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    // Both join
    ws_a.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();
    ws_b.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Drain alice's room_users + user_joined notifications
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;

    // Bob leaves the room
    ws_b.send(text_msg(json!({ "type": "leave_room", "room_id": room_id })))
    .await
    .unwrap();

    // Alice should receive user_left
    let msg = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        ws_a.next(),
    )
    .await
    .expect("should receive within timeout")
    .expect("stream should not end")
    .expect("should be valid");

    let parsed: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
    assert_eq!(parsed["type"], "user_left");
    assert_eq!(parsed["username"], "bob");
    assert_eq!(parsed["room_id"], room_id);
}

#[tokio::test]
async fn test_ws_message_persisted() {
    let base = spawn_app().await;

    register_user(&base, "alice", "password123").await;
    let token = login_user(&base, "alice", "password123").await;

    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .unwrap();

    // Join room and send a message
    ws.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Drain room_users message
    let _ = recv_json(&mut ws).await;

    ws.send(text_msg(json!({
        "type": "send_message",
        "room_id": room_id,
        "content": "persisted message"
    })))
    .await
    .unwrap();

    // Wait for the echo
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        ws.next(),
    )
    .await;

    // Now fetch via REST API
    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms/{room_id}/messages"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    let messages = body.as_array().expect("should be an array");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], "persisted message");
    assert_eq!(messages[0]["username"], "alice");
}

// ============================================================
// Signaling relay tests
// ============================================================

#[tokio::test]
async fn test_ws_offer_relayed_to_target() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}")).await.unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}")).await.unwrap();

    // Both join room
    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Alice sends offer to bob
    let sdp_payload = json!({"type": "offer", "sdp": "v=0\r\n..."});
    ws_a.send(text_msg(json!({
        "type": "offer",
        "room_id": room_id,
        "target_user_id": bob_id,
        "payload": sdp_payload
    }))).await.unwrap();

    // Bob should receive the relayed offer
    let received = recv_json(&mut ws_b).await;
    assert_eq!(received["type"], "offer");
    assert_eq!(received["from_user_id"], alice_id);
    assert_eq!(received["from_username"], "alice");
    assert_eq!(received["room_id"], room_id);
    assert_eq!(received["payload"]["sdp"], "v=0\r\n...");

    // Alice should NOT receive her own offer back
    let nothing = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        ws_a.next(),
    ).await;
    assert!(nothing.is_err(), "alice should not receive her own offer");
}

#[tokio::test]
async fn test_ws_answer_relayed_to_offerer() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}")).await.unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}")).await.unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Bob sends answer to alice
    let sdp_payload = json!({"type": "answer", "sdp": "v=0\r\nanswer..."});
    ws_b.send(text_msg(json!({
        "type": "answer",
        "room_id": room_id,
        "target_user_id": alice_id,
        "payload": sdp_payload
    }))).await.unwrap();

    let received = recv_json(&mut ws_a).await;
    assert_eq!(received["type"], "answer");
    assert_eq!(received["from_user_id"], bob_id);
    assert_eq!(received["from_username"], "bob");
    assert_eq!(received["payload"]["sdp"], "v=0\r\nanswer...");
}

#[tokio::test]
async fn test_ws_ice_candidate_relayed() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}")).await.unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}")).await.unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Alice sends ICE candidate to bob
    let ice_payload = json!({"candidate": "candidate:1 ...", "sdpMid": "0", "sdpMLineIndex": 0});
    ws_a.send(text_msg(json!({
        "type": "ice_candidate",
        "room_id": room_id,
        "target_user_id": bob_id,
        "payload": ice_payload
    }))).await.unwrap();

    let received = recv_json(&mut ws_b).await;
    assert_eq!(received["type"], "ice_candidate");
    assert_eq!(received["from_user_id"], alice_id);
    assert_eq!(received["payload"]["candidate"], "candidate:1 ...");
    assert_eq!(received["payload"]["sdpMid"], "0");
}

#[tokio::test]
async fn test_ws_signaling_to_missing_user_returns_error() {
    let base = spawn_app().await;

    register_and_get_id(&base, "alice", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}")).await.unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;

    // Send offer to nonexistent user
    ws_a.send(text_msg(json!({
        "type": "offer",
        "room_id": room_id,
        "target_user_id": "nonexistent",
        "payload": {"type": "offer", "sdp": "..."}
    }))).await.unwrap();

    let received = recv_json(&mut ws_a).await;
    assert_eq!(received["type"], "error");
}

#[tokio::test]
async fn test_ws_signaling_not_broadcast_to_third_user() {
    let base = spawn_app().await;

    let _alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let _charlie_id = register_and_get_id(&base, "charlie", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;
    let token_c = login_user(&base, "charlie", "password123").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}")).await.unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}")).await.unwrap();
    let (mut ws_c, _) = connect_async(format!("{ws_url}/ws?token={token_c}")).await.unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    ws_c.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;
    drain_ws(&mut ws_c).await;

    // Alice sends offer to bob only
    ws_a.send(text_msg(json!({
        "type": "offer",
        "room_id": room_id,
        "target_user_id": bob_id,
        "payload": {"type": "offer", "sdp": "..."}
    }))).await.unwrap();

    // Bob should receive it
    let received = recv_json(&mut ws_b).await;
    assert_eq!(received["type"], "offer");

    // Charlie should NOT receive it
    let nothing = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        ws_c.next(),
    ).await;
    assert!(nothing.is_err(), "charlie should not receive alice's offer to bob");
}

#[tokio::test]
async fn test_ws_join_room_returns_room_users() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let _bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice", "password123").await;
    let token_b = login_user(&base, "bob", "password123").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}")).await.unwrap();

    // Alice joins — should get room_users with empty list (she's the only one)
    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();
    let msg = recv_json(&mut ws_a).await;
    assert_eq!(msg["type"], "room_users");
    assert_eq!(msg["room_id"], room_id);
    let users = msg["users"].as_array().expect("users should be an array");
    assert!(users.is_empty(), "no other users when alice is first to join");

    // Bob joins — should get room_users listing alice
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}")).await.unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id}))).await.unwrap();

    // Drain alice's user_joined notification for bob
    let _ = recv_json(&mut ws_a).await;

    let msg = recv_json(&mut ws_b).await;
    assert_eq!(msg["type"], "room_users");
    assert_eq!(msg["room_id"], room_id);
    let users = msg["users"].as_array().expect("users should be an array");
    assert_eq!(users.len(), 1);
    assert_eq!(users[0]["user_id"], alice_id);
    assert_eq!(users[0]["username"], "alice");
}
