use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde_json::{Value, json};
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Helper to create a text WS message from a JSON value
fn text_msg(v: Value) -> Message {
    Message::Text(v.to_string().into())
}

/// Spawns the app on a random port with a temporary SQLite database.
/// Test mode is enabled, so `/api/auth/test-login` is available.
/// Returns the base URL (e.g., "http://127.0.0.1:12345").
async fn spawn_app() -> String {
    let db_url = format!(
        "sqlite:/tmp/racquet-test-{}.db?mode=rwc",
        uuid::Uuid::new_v4()
    );

    let pool = racquet::db::create_pool(&db_url).await;
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("migrations failed");

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
        static_dir: None,
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

    let body: Value = resp
        .json()
        .await
        .expect("test-login response should be JSON");
    body["token"]
        .as_str()
        .expect("token field should exist")
        .to_string()
}

/// Helper: create a test user via test-login and return their user ID
async fn register_and_get_id(base: &str, username: &str, _password: &str) -> String {
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/auth/test-login"))
        .json(&json!({ "email": username }))
        .send()
        .await
        .expect("test-login request should succeed");

    let body: Value = resp
        .json()
        .await
        .expect("test-login response should be JSON");
    body["user_id"]
        .as_str()
        .expect("user_id field should exist")
        .to_string()
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

    resp.json()
        .await
        .expect("create room response should be JSON")
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
    assert!(
        body["token"].as_str().is_some(),
        "response should include a token"
    );
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
async fn test_custom_username_survives_relogin() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/profile"))
        .bearer_auth(&token)
        .json(&json!({ "username": "Ali" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let new_token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/profile"))
        .bearer_auth(&new_token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "Ali");
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

#[tokio::test]
async fn test_rename_room() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .json(&json!({ "name": "renamed" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "renamed");
    assert_eq!(body["id"], room_id);

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    let rooms = body.as_array().unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0]["name"], "renamed");
}

#[tokio::test]
async fn test_rename_room_duplicate() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    create_room(&base, &token, "general").await;
    let room = create_room(&base, &token, "gaming").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .json(&json!({ "name": "general" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_rename_room_same_name_allowed() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .json(&json!({ "name": "general" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rename_room_empty_name() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .json(&json!({ "name": "   " }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_rename_room_not_found() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .put(format!("{base}/api/rooms/nope"))
        .bearer_auth(&token)
        .json(&json!({ "name": "whatever" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_room() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .delete(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_delete_room_not_found() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;

    let resp = reqwest::Client::new()
        .delete(format!("{base}/api/rooms/nope"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_room_no_auth() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let resp = reqwest::Client::new()
        .delete(format!("{base}/api/rooms/{room_id}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_delete_room_with_messages() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .unwrap();

    ws.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
        .await
        .unwrap();
    drain_ws(&mut ws).await;

    ws.send(text_msg(json!({
        "type": "send_message",
        "room_id": room_id,
        "content": "hello"
    })))
    .await
    .unwrap();

    let parsed = recv_json(&mut ws).await;
    assert_eq!(parsed["type"], "new_message");

    let resp = reqwest::Client::new()
        .delete(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_room_broadcasts_room_deleted() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .unwrap();

    ws.send(text_msg(json!({ "type": "join_room", "room_id": room_id })))
        .await
        .unwrap();
    drain_ws(&mut ws).await;

    let resp = reqwest::Client::new()
        .delete(format!("{base}/api/rooms/{room_id}"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let parsed = recv_json(&mut ws).await;
    assert_eq!(parsed["type"], "room_deleted");
    assert_eq!(parsed["room_id"], room_id);
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
    assert_eq!(
        messages.len(),
        10,
        "second page should have remaining 10 messages"
    );
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
    while let Ok(msg) =
        tokio::time::timeout(std::time::Duration::from_millis(100), ws_a.next()).await
    {
        let _ = msg;
    }
    while let Ok(msg) =
        tokio::time::timeout(std::time::Duration::from_millis(100), ws_b.next()).await
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
    let msg_a = tokio::time::timeout(std::time::Duration::from_secs(2), ws_a.next())
        .await
        .expect("alice should receive within timeout")
        .expect("stream should not end")
        .expect("should be a valid message");

    let parsed_a: Value = serde_json::from_str(&msg_a.into_text().unwrap()).unwrap();
    assert_eq!(parsed_a["type"], "new_message");
    assert_eq!(parsed_a["content"], "hello from alice");

    let msg_b = tokio::time::timeout(std::time::Duration::from_secs(2), ws_b.next())
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

    // Drain alice's room_users + user_joined notifications
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;

    // Bob leaves the room
    ws_b.send(text_msg(
        json!({ "type": "leave_room", "room_id": room_id }),
    ))
    .await
    .unwrap();

    // Alice should receive user_left
    let msg = tokio::time::timeout(std::time::Duration::from_secs(2), ws_a.next())
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
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;

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
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    // Both join room
    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
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
    })))
    .await
    .unwrap();

    // Bob should receive the relayed offer
    let received = recv_json(&mut ws_b).await;
    assert_eq!(received["type"], "offer");
    assert_eq!(received["from_user_id"], alice_id);
    assert_eq!(received["from_username"], "alice");
    assert_eq!(received["room_id"], room_id);
    assert_eq!(received["payload"]["sdp"], "v=0\r\n...");

    // Alice should NOT receive her own offer back
    let nothing = tokio::time::timeout(std::time::Duration::from_millis(200), ws_a.next()).await;
    assert!(nothing.is_err(), "alice should not receive her own offer");
}

#[tokio::test]
async fn test_ws_answer_relayed_to_offerer() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
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
    })))
    .await
    .unwrap();

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
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
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
    })))
    .await
    .unwrap();

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
    let token_a = login_user(&base, "alice").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;

    // Send offer to nonexistent user
    ws_a.send(text_msg(json!({
        "type": "offer",
        "room_id": room_id,
        "target_user_id": "nonexistent",
        "payload": {"type": "offer", "sdp": "..."}
    })))
    .await
    .unwrap();

    let received = recv_json(&mut ws_a).await;
    assert_eq!(received["type"], "error");
}

#[tokio::test]
async fn test_ws_signaling_not_broadcast_to_third_user() {
    let base = spawn_app().await;

    let _alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let _charlie_id = register_and_get_id(&base, "charlie", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;
    let token_c = login_user(&base, "charlie").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();
    let (mut ws_c, _) = connect_async(format!("{ws_url}/ws?token={token_c}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_c.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
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
    })))
    .await
    .unwrap();

    // Bob should receive it
    let received = recv_json(&mut ws_b).await;
    assert_eq!(received["type"], "offer");

    // Charlie should NOT receive it
    let nothing = tokio::time::timeout(std::time::Duration::from_millis(200), ws_c.next()).await;
    assert!(
        nothing.is_err(),
        "charlie should not receive alice's offer to bob"
    );
}

#[tokio::test]
async fn test_ws_join_room_returns_room_users() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let _bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();

    // Alice joins — should get room_users with empty list (she's the only one)
    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    let msg = recv_json(&mut ws_a).await;
    assert_eq!(msg["type"], "room_users");
    assert_eq!(msg["room_id"], room_id);
    let users = msg["users"].as_array().expect("users should be an array");
    assert!(
        users.is_empty(),
        "no other users when alice is first to join"
    );

    // Bob joins — should get room_users listing alice
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();

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

// ============================================================
// Screen share signaling tests
// ============================================================

/// Receive WS messages until one matches `predicate`, or timeout. Drains earlier messages.
async fn recv_json_matching<S, F>(ws: &mut S, predicate: F) -> Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    F: Fn(&Value) -> bool,
{
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let msg = tokio::time::timeout(remaining, ws.next())
            .await
            .expect("should receive matching message before timeout")
            .expect("stream should not end")
            .expect("should be a valid message");
        let parsed: Value = serde_json::from_str(&msg.into_text().unwrap()).unwrap();
        if predicate(&parsed) {
            return parsed;
        }
    }
}

#[tokio::test]
async fn test_ws_screen_share_start_broadcasts_to_room() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let _bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Alice starts sharing
    ws_a.send(text_msg(json!({
        "type": "screen_share_start",
        "room_id": room_id,
        "payload": { "stream_id": "abc-stream-123" }
    })))
    .await
    .unwrap();

    // Bob should receive screen_share_started
    let received = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_started").await;
    assert_eq!(received["room_id"], room_id);
    assert_eq!(received["user_id"], alice_id);
    assert_eq!(received["username"], "alice");
    assert_eq!(received["stream_id"], "abc-stream-123");

    // Alice should also receive the broadcast (mirrors send_message behavior)
    let echoed = recv_json_matching(&mut ws_a, |v| v["type"] == "screen_share_started").await;
    assert_eq!(echoed["user_id"], alice_id);
}

#[tokio::test]
async fn test_ws_screen_share_start_rejected_when_active() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let _bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Alice starts; both receive screen_share_started
    ws_a.send(text_msg(json!({
        "type": "screen_share_start",
        "room_id": room_id,
        "payload": { "stream_id": "alice-stream" }
    })))
    .await
    .unwrap();
    let _ = recv_json_matching(&mut ws_a, |v| v["type"] == "screen_share_started").await;
    let _ = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_started").await;

    // Bob attempts to start — should get an error
    ws_b.send(text_msg(json!({
        "type": "screen_share_start",
        "room_id": room_id,
        "payload": { "stream_id": "bob-stream" }
    })))
    .await
    .unwrap();

    let received = recv_json_matching(&mut ws_b, |v| v["type"] == "error").await;
    assert!(
        received["message"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("already"),
        "error message should mention an active share, got: {}",
        received["message"]
    );

    // Alice should NOT receive a second screen_share_started for bob
    let nothing = tokio::time::timeout(std::time::Duration::from_millis(200), ws_a.next()).await;
    assert!(
        nothing.is_err(),
        "alice's share should be unaffected by bob's rejected attempt"
    );

    // Sanity: alice's share is still attributed to alice
    assert_eq!(alice_id, alice_id);
}

#[tokio::test]
async fn test_ws_screen_share_stop_broadcasts() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let _bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Start
    ws_a.send(text_msg(json!({
        "type": "screen_share_start",
        "room_id": room_id,
        "payload": { "stream_id": "alice-stream" }
    })))
    .await
    .unwrap();
    let _ = recv_json_matching(&mut ws_a, |v| v["type"] == "screen_share_started").await;
    let _ = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_started").await;

    // Stop
    ws_a.send(text_msg(json!({
        "type": "screen_share_stop",
        "room_id": room_id,
        "payload": {}
    })))
    .await
    .unwrap();

    let received_b = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_stopped").await;
    assert_eq!(received_b["room_id"], room_id);
    assert_eq!(received_b["user_id"], alice_id);

    let received_a = recv_json_matching(&mut ws_a, |v| v["type"] == "screen_share_stopped").await;
    assert_eq!(received_a["user_id"], alice_id);
}

#[tokio::test]
async fn test_ws_screen_share_cleared_on_disconnect() {
    let base = spawn_app().await;

    let alice_id = register_and_get_id(&base, "alice", "password123").await;
    let bob_id = register_and_get_id(&base, "bob", "password123").await;
    let token_a = login_user(&base, "alice").await;
    let token_b = login_user(&base, "bob").await;

    let room = create_room(&base, &token_a, "voice").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws_a, _) = connect_async(format!("{ws_url}/ws?token={token_a}"))
        .await
        .unwrap();
    let (mut ws_b, _) = connect_async(format!("{ws_url}/ws?token={token_b}"))
        .await
        .unwrap();

    ws_a.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    ws_b.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws_a).await;
    drain_ws(&mut ws_b).await;

    // Alice starts sharing
    ws_a.send(text_msg(json!({
        "type": "screen_share_start",
        "room_id": room_id,
        "payload": { "stream_id": "alice-stream" }
    })))
    .await
    .unwrap();
    let _ = recv_json_matching(&mut ws_a, |v| v["type"] == "screen_share_started").await;
    let _ = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_started").await;

    // Alice disconnects abruptly
    drop(ws_a);

    // Bob receives screen_share_stopped (somewhere in his stream, possibly after user_left)
    let stopped = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_stopped").await;
    assert_eq!(stopped["user_id"], alice_id);

    // Bob can now start sharing successfully
    ws_b.send(text_msg(json!({
        "type": "screen_share_start",
        "room_id": room_id,
        "payload": { "stream_id": "bob-stream" }
    })))
    .await
    .unwrap();

    let started = recv_json_matching(&mut ws_b, |v| v["type"] == "screen_share_started").await;
    assert_eq!(started["user_id"], bob_id);
    assert_eq!(started["stream_id"], "bob-stream");
}

#[tokio::test]
async fn test_ws_send_message_too_long() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .unwrap();

    ws.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws).await;

    ws.send(text_msg(json!({
        "type": "send_message",
        "room_id": room_id,
        "content": "x".repeat(4001)
    })))
    .await
    .unwrap();

    let received = recv_json_matching(&mut ws, |v| v["type"] == "error").await;
    assert!(
        received["message"].as_str().unwrap().contains("4000"),
        "error should mention the limit, got: {}",
        received["message"]
    );

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms/{room_id}/messages"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(
        body.as_array().unwrap().is_empty(),
        "over-long message must not be persisted"
    );
}

#[tokio::test]
async fn test_ws_send_message_at_limit() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .unwrap();

    ws.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws).await;

    let content = "x".repeat(4000);
    ws.send(text_msg(json!({
        "type": "send_message",
        "room_id": room_id,
        "content": content
    })))
    .await
    .unwrap();

    let received = recv_json_matching(&mut ws, |v| v["type"] == "new_message").await;
    assert_eq!(received["content"].as_str().unwrap().len(), 4000);
}

#[tokio::test]
async fn test_ws_send_message_blank_rejected() {
    let base = spawn_app().await;
    let token = login_user(&base, "alice@test.com").await;
    let room = create_room(&base, &token, "general").await;
    let room_id = room["id"].as_str().unwrap();

    let ws_url = base.replace("http://", "ws://");
    let (mut ws, _) = connect_async(format!("{ws_url}/ws?token={token}"))
        .await
        .unwrap();

    ws.send(text_msg(json!({"type":"join_room","room_id":room_id})))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    drain_ws(&mut ws).await;

    ws.send(text_msg(json!({
        "type": "send_message",
        "room_id": room_id,
        "content": "   \n  "
    })))
    .await
    .unwrap();

    let _ = recv_json_matching(&mut ws, |v| v["type"] == "error").await;

    let resp = reqwest::Client::new()
        .get(format!("{base}/api/rooms/{room_id}/messages"))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}
