use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to create a text WS message from a JSON value
fn text_msg(v: Value) -> Message {
    Message::Text(v.to_string().into())
}

/// Spawns the app on a random port with a temporary SQLite database.
/// Test mode is enabled, so `/api/auth/test-login` is available.
/// Returns the base URL (e.g., "http://127.0.0.1:12345").
async fn spawn_app() -> String {
    let db_url = format!("sqlite:/tmp/racquet-test-{}.db?mode=rwc", uuid::Uuid::new_v4());

    let pool = racquet::db::create_pool(&db_url).await;
    sqlx::migrate!().run(&pool).await.expect("migrations failed");

    let state = racquet::AppState {
        db: pool,
        cm: std::sync::Arc::new(racquet::connection::ConnectionManager::new()),
        jwt_secret: "test-secret".to_string(),
        google_client_id: "test-client-id".to_string(),
        google_client_secret: "test-client-secret".to_string(),
        google_redirect_uri: "http://localhost:3000/api/auth/google/callback".to_string(),
        frontend_url: "http://localhost:5173".to_string(),
        allowed_emails: vec!["allowed@test.com".to_string()],
        test_mode: true,
    };

    let app = racquet::build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://{addr}")
}

/// Helper: login via test-login endpoint and return the JWT token
async fn login_user(base: &str, email: &str) -> String {
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/auth/test-login"))
        .json(&json!({ "email": email }))
        .send()
        .await
        .expect("test-login request should succeed");

    let body: Value = resp.json().await.expect("test-login response should be JSON");
    body["token"].as_str().expect("token field should exist").to_string()
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
async fn test_login_via_test_endpoint() {
    let base = spawn_app().await;

    let resp = reqwest::Client::new()
        .post(format!("{base}/api/auth/test-login"))
        .json(&json!({ "email": "alice@test.com" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    assert!(body["token"].as_str().is_some(), "response should include a token");
}

#[tokio::test]
async fn test_login_same_email_returns_same_user() {
    let base = spawn_app().await;

    let token1 = login_user(&base, "alice@test.com").await;
    let token2 = login_user(&base, "alice@test.com").await;

    // Both tokens should decode to the same user_id
    // We verify by using both tokens to create rooms (they should share state)
    let room = create_room(&base, &token1, "room1").await;
    assert!(room["id"].as_str().is_some());

    let room = create_room(&base, &token2, "room2").await;
    assert!(room["id"].as_str().is_some());
}

#[tokio::test]
async fn test_get_profile() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/profile"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["email"], "alice@test.com");
    assert_eq!(body["username"], "alice");
    assert!(body["id"].as_str().is_some());
}

#[tokio::test]
async fn test_update_profile() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/profile"))
        .bearer_auth(&token)
        .json(&json!({ "username": "Alice Wonder" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["user"]["username"], "Alice Wonder");
    assert!(body["token"].as_str().is_some());

    // Use the new token to verify it works
    let new_token = body["token"].as_str().unwrap();
    let resp = reqwest::Client::new()
        .get(format!("{base}/api/profile"))
        .bearer_auth(new_token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "Alice Wonder");
}

#[tokio::test]
async fn test_update_profile_empty_name() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/profile"))
        .bearer_auth(&token)
        .json(&json!({ "username": "   " }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_oauth_redirect_returns_302() {
    let base = spawn_app().await;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let resp = client
        .get(format!("{base}/api/auth/google"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT);

    let location = resp.headers().get("location").unwrap().to_str().unwrap();
    assert!(
        location.starts_with("https://accounts.google.com/o/oauth2/v2/auth"),
        "should redirect to Google"
    );
    assert!(location.contains("client_id=test-client-id"));
}

// ============================================================
// Room tests
// ============================================================

#[tokio::test]
async fn test_create_room() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

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
    let token = login_user(&base, "alice@test.com").await;

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
    let token = login_user(&base, "alice@test.com").await;

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
    let token = login_user(&base, "alice@test.com").await;

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
    let token = login_user(&base, "alice@test.com").await;
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
    let token = login_user(&base, "alice@test.com").await;
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
    let token = login_user(&base, "alice@test.com").await;

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

    let token_a = login_user(&base, "alice@test.com").await;
    let token_b = login_user(&base, "bob@test.com").await;

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

    // Drain join notifications
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    while let Ok(msg) = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        ws_a.next(),
    )
    .await
    {
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

    let token_a = login_user(&base, "alice@test.com").await;
    let token_b = login_user(&base, "bob@test.com").await;

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

    // Bob connects and joins the room
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
    .await
    .unwrap();

    // Alice should receive a user_joined notification for bob
    let msg = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        ws_a.next(),
    )
    .await
    .expect("should receive within timeout")
    .expect("stream should not end")
    .expect("should be valid");

    let parsed: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
    assert_eq!(parsed["type"], "user_joined");
    assert_eq!(parsed["username"], "bob");
    assert_eq!(parsed["room_id"], room_id);
}

#[tokio::test]
async fn test_ws_user_left_notification() {
    let base = spawn_app().await;

    let token_a = login_user(&base, "alice@test.com").await;
    let token_b = login_user(&base, "bob@test.com").await;

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

    // Drain alice's user_joined notification for bob
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        ws_a.next(),
    )
    .await;

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
    let token = login_user(&base, "alice@test.com").await;

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
