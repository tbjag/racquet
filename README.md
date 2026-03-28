# Racquet

A lightweight Discord alternative for small friend groups (2-10 people), built in Rust. Features text chat, audio, and video.

## Architecture

```
[Browser A]                         [Browser B]
  WebRTC audio/video (P2P)  <--->  WebRTC audio/video (P2P)
  Svelte + TypeScript UI            Svelte + TypeScript UI
       |  ^                              |  ^
  WS   |  | signaling              WS   |  | signaling
       v  |                              v  |
     +----------------------------------------------+
     |            Rust Axum Server                   |
     |  - WebSocket hub (tokio-tungstenite)          |
     |  - REST API (axum)                            |
     |  - Chat history (SQLite via sqlx)             |
     |  - Room & user management                     |
     |  - Serves /static frontend files              |
     +----------------------------------------------+
```

Audio and video flow directly browser-to-browser (P2P mesh). The server only relays small signaling messages to coordinate connections and handles text chat.

## Tech Stack

| Layer | Choice |
|---|---|
| Async runtime | Tokio |
| HTTP + REST | Axum |
| WebSockets | tokio-tungstenite |
| Database | SQLite (sqlx) |
| Auth | JWT (jsonwebtoken) + Argon2 password hashing |
| Frontend | Svelte + TypeScript (Vite) |
| Audio/Video | Browser WebRTC API (no Rust dependency) |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- SQLite is handled automatically via sqlx -- no manual install needed

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite:racquet.db?mode=rwc` | SQLite connection string |
| `JWT_SECRET` | `racquet-dev-secret-do-not-use-in-prod` | Secret for signing JWTs |
| `PORT` | `3000` | Server port |

### Running the Server

```bash
# Build and run (database is created and migrated automatically)
cargo run

# With custom config
DATABASE_URL="sqlite:my.db?mode=rwc" JWT_SECRET="my-secret" PORT=8080 cargo run

# Auto-restart on file changes (requires cargo-watch)
cargo watch -x run
```

The server will be available at `http://localhost:3000`.

### Running Tests

```bash
# Run all tests (unit + integration)
cargo test

# Run with output visible
cargo test -- --nocapture

# Integration tests only (run single-threaded to avoid port conflicts)
cargo test --test integration -- --test-threads=1

# A specific test
cargo test test_register_success -- --nocapture
```

**Test coverage:**
- 9 unit tests (password hashing, JWT lifecycle, connection manager)
- 19 integration tests (auth, rooms, messages, WebSocket)

## Project Structure

```
src/
  main.rs           # Entry point -- loads config, creates DB pool, starts server
  lib.rs            # AppState definition and router setup (all route bindings)
  config.rs         # Configuration from environment variables
  db.rs             # SQLite connection pool creation (WAL mode, foreign keys)
  models.rs         # Data models (User, Room, Message) and DB query functions
  auth.rs           # Password hashing (Argon2), JWT creation/verification, AuthUser extractor
  errors.rs         # AppError enum -> HTTP status code mapping
  routes.rs         # REST handlers: register, login, rooms CRUD, message history
  ws.rs             # WebSocket handler: join/leave rooms, send/broadcast messages
  connection.rs     # In-memory connection manager tracking users per room

tests/
  integration.rs    # Integration tests (spawns real server, tests API + WebSocket)

migrations/
  20260326000001_initial.sql  # Schema: users, rooms, messages tables + indexes
```

## API

### REST Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/api/register` | No | Register a new user |
| `POST` | `/api/login` | No | Login, returns JWT |
| `GET` | `/api/rooms` | Yes | List all rooms |
| `POST` | `/api/rooms` | Yes | Create a room |
| `GET` | `/api/rooms/{room_id}/messages` | Yes | Fetch message history (cursor pagination) |

### WebSocket

Connect to `/ws?token=<JWT>`. Messages are JSON with a `type` field:

**Client -> Server:**
- `join_room` -- join a room
- `leave_room` -- leave a room
- `send_message` -- send a chat message (persisted to DB and broadcast)

**Server -> Client:**
- `user_joined` -- a user joined the room
- `user_left` -- a user left or disconnected
- `new_message` -- new chat message

## Manual API Testing (Postman / Bruno)

Start the server with `cargo run`, then use the following requests against `http://localhost:3000`.

### 1. Register a user

```
POST /api/register
Content-Type: application/json

{
  "username": "testuser",
  "password": "password123"
}
```

Returns `201` with `{ "id": "...", "username": "testuser" }`.

### 2. Log in

```
POST /api/login
Content-Type: application/json

{
  "username": "testuser",
  "password": "password123"
}
```

Returns `200` with `{ "token": "eyJ..." }`. Copy this token — it's your bearer token for all authenticated requests (expires in 24h).

### 3. Create a room

```
POST /api/rooms
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "general"
}
```

Returns `201` with the room object. Copy the `id` for subsequent requests.

### 4. List rooms

```
GET /api/rooms
Authorization: Bearer <token>
```

### 5. Get messages in a room

```
GET /api/rooms/<room_id>/messages
Authorization: Bearer <token>
```

Optional query params: `?limit=20` or `?before=<message_id>` for cursor pagination.

### 6. WebSocket

Connect to:

```
ws://localhost:3000/ws?token=<token>
```

Once connected, send JSON messages:

**Join a room:**
```json
{ "type": "join_room", "room_id": "<room_id>" }
```

**Send a message:**
```json
{ "type": "send_message", "room_id": "<room_id>", "content": "hello world" }
```

**Leave a room:**
```json
{ "type": "leave_room", "room_id": "<room_id>" }
```

The server broadcasts `user_joined`, `new_message`, and `user_left` events to all users in the room.

### Multi-user testing

Register a second user, get a second token, and open a second WebSocket connection. Have both join the same room to see each other's join notifications and messages in real time.

## Database Schema

Three tables with foreign key relationships:

- **users** -- id, username (unique), password_hash, created_at
- **rooms** -- id, name (unique), created_by (FK to users), created_at
- **messages** -- id, room_id (FK to rooms), user_id (FK to users), content, created_at

Migrations run automatically on server startup.
