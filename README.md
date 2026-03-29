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
| Frontend | SvelteKit 2 + Svelte 5 + TypeScript (Vite 7) |
| Integration tests | Playwright (Chromium) |
| Audio/Video | Browser WebRTC API (no Rust dependency) |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) (for the frontend)
- SQLite is handled automatically via sqlx -- no manual install needed

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | `sqlite:racquet.db?mode=rwc` | SQLite connection string |
| `JWT_SECRET` | `racquet-dev-secret-do-not-use-in-prod` | Secret for signing JWTs |
| `PORT` | `3000` | Server port |

### Running the App

```bash
# Terminal 1: Start the backend (port 3000)
cargo run

# Terminal 2: Start the frontend (port 5173)
cd frontend && npm install && npm run dev
```

Open `http://localhost:5173` in your browser. Register an account, create a room, and start chatting.

For multi-user testing, open a second browser (e.g. Chrome + Edge), register a second user, and join the same room. Messages appear in real time.

### Running Tests

**Backend tests (38 total: 13 unit + 25 integration):**

```bash
cargo test
```

Each backend integration test spawns an isolated server with its own temp database.

**Frontend integration tests (25 Playwright tests):**

```bash
cd frontend && npx playwright test
```

Playwright auto-starts both the backend and frontend servers. Tests cover auth flows, room management, real-time chat, and WebRTC calls (including two-user P2P call tests).

```bash
# See the browser while tests run
npx playwright test --headed

# Interactive Playwright UI
npx playwright test --ui
```

## Project Structure

```
src/
  main.rs           # Entry point -- loads config, creates DB pool, starts server
  lib.rs            # AppState definition, router setup, CORS config
  config.rs         # Configuration from environment variables
  db.rs             # SQLite connection pool creation (WAL mode, foreign keys)
  models.rs         # Data models (User, Room, Message) and DB query functions
  auth.rs           # Password hashing (Argon2), JWT creation/verification, AuthUser extractor
  errors.rs         # AppError enum -> HTTP status code mapping
  routes.rs         # REST handlers: register, login, rooms CRUD, message history
  ws.rs             # WebSocket handler: join/leave rooms, chat messages, signaling relay
  connection.rs     # In-memory connection manager: rooms, broadcast, targeted send

frontend/
  src/
    lib/
      api.ts        # REST API client (register, login, rooms, messages)
      auth.ts       # JWT token management (localStorage)
      ws.ts         # WebSocket client (connect, join/leave rooms, send messages, signaling)
      webrtc.ts     # WebRTC peer connection manager (P2P audio/video)
    routes/
      +layout.svelte  # Auth guard -- redirects to /login if no token
      +layout.ts      # SSR disabled (client-side SPA)
      +page.svelte    # Main app: room sidebar + chat area
      login/+page.svelte
      register/+page.svelte
  tests/
    helpers.ts      # Test utilities (register/login via API, setup authenticated user)
    auth.spec.ts    # Auth flow tests (7 tests)
    rooms.spec.ts   # Room list and creation tests (5 tests)
    chat.spec.ts    # Messaging tests including real-time two-user test (6 tests)
    webrtc.spec.ts  # WebRTC call tests including two-user P2P test (7 tests)
  playwright.config.ts

tests/
  integration.rs    # Backend integration tests (spawns real server, tests API + WebSocket)

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
- `offer` -- WebRTC SDP offer (relayed to target peer)
- `answer` -- WebRTC SDP answer (relayed to target peer)
- `ice_candidate` -- WebRTC ICE candidate (relayed to target peer)
- `call_leave` -- notify a peer you're leaving the call

**Server -> Client:**
- `user_joined` -- a user joined the room
- `user_left` -- a user left or disconnected
- `new_message` -- new chat message
- `room_users` -- list of users in the room (sent on join)
- `offer` / `answer` / `ice_candidate` / `call_leave` -- relayed signaling (includes `from_user_id`)

## Database Schema

Three tables with foreign key relationships:

- **users** -- id, username (unique), password_hash, created_at
- **rooms** -- id, name (unique), created_by (FK to users), created_at
- **messages** -- id, room_id (FK to rooms), user_id (FK to users), content, created_at

Migrations run automatically on server startup.

## Deployment Notes (TODO)

Notes for cloud deployment on Digital Ocean with Caddy reverse proxy.

### HTTPS is required

Browsers block `getUserMedia` (camera/mic) on non-localhost origins without HTTPS. Caddy handles TLS automatically via Let's Encrypt.

### Caddy reverse proxy setup

Add a subdomain (e.g. `racquet.yourdomain.com`) with a Caddy config block that:
- Proxies `/api/*` and `/ws` to the Axum server (`localhost:3000`)
- WebSocket proxying needs explicit config in Caddy for the `/ws` path
- Optionally serve the built frontend static files from Caddy, or let Axum serve them (Phase 4)

### STUN/TURN

Google's free STUN server (`stun:stun.l.google.com:19302`) is already configured and sufficient for most connections. Users behind strict NATs (mobile carriers, some corporate networks) will fail to connect without a TURN relay. Coturn is the standard self-hosted option and can run on the same droplet. Start without TURN and add it only if someone can't connect.

### SQLite sufficiency

SQLite is fine for 2-10 users. Write volume (chat messages) is trivial at this scale. WAL mode is already enabled via sqlx, which handles concurrent reads well. If contention ever becomes an issue, that's the signal to consider Postgres -- but it shouldn't at this scale.

### Old message cleanup

No auto-cleanup exists yet. Options:
- **Cron job**: `DELETE FROM messages WHERE created_at < datetime('now', '-30 days')` against the SQLite DB
- **In-app**: a Rust background task using `tokio::spawn` with `tokio::time::interval`
- Run `VACUUM` after bulk deletes if disk space matters (SQLite doesn't reclaim space automatically)

### Security hardening for production

- **JWT secret**: must be a strong random value from an environment variable (not the dev default)
- **CORS**: lock down to your domain (currently permissive for dev)
- **Rate limiting**: not critical at this scale but trivial to add with `tower` middleware

### Google SSO (optional)

Can add Google OAuth 2.0 / OpenID Connect alongside existing password auth:
1. Create OAuth 2.0 credentials in Google Cloud Console (free)
2. Add a backend endpoint (`/api/auth/google`) that exchanges the auth code with Google for an ID token
3. Frontend: "Sign in with Google" button that redirects to Google's OAuth URL
4. DB: add `google_id` column to `users` table, make password optional for Google users
5. The JWT issued by the server stays the same, so WebSocket auth, rooms, chat, and WebRTC all work unchanged
6. OAuth redirect URI must be HTTPS (Caddy handles this)
