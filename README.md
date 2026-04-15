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

## Deployment

Single-origin deploy on a Linux VPS (tested layout: Digital Ocean droplet + Caddy). Axum serves both the API and the prebuilt Svelte SPA on port 3000; Caddy terminates TLS and routes a subdomain to it.

### 1. Build

```bash
# Frontend -> static bundle at frontend/dist/
cd frontend && npm ci && npm run build && cd ..

# Backend -> target/release/racquet (LTO + strip, small binary)
cargo build --release
```

### 2. Droplet layout

Copy to `/opt/racquet/` on the droplet:

```
/opt/racquet/
  racquet               # from target/release/
  dist/                 # from frontend/dist/
  .env                  # copied from .env.example with prod values
  allowed_emails.txt    # one email per line
```

Create a `racquet` system user that owns `/opt/racquet/`.

### 3. `.env` (prod values)

```env
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REDIRECT_URI=https://racquet.yourdomain.com/api/auth/google/callback
FRONTEND_URL=https://racquet.yourdomain.com
JWT_SECRET=<generate a strong random value>
DATABASE_URL=sqlite:/opt/racquet/racquet.db?mode=rwc
STATIC_DIR=/opt/racquet/dist
```

### 4. systemd

```bash
sudo cp deploy/racquet.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now racquet
sudo systemctl status racquet
```

The unit in `deploy/racquet.service` runs the binary as the `racquet` user, loads `/opt/racquet/.env`, and restarts on failure.

### 5. Caddy

Add a block to your existing Caddyfile (Caddy v2 auto-handles the WebSocket upgrade for `/ws`):

```caddy
racquet.yourdomain.com {
    reverse_proxy localhost:3000
}
```

Then `sudo systemctl reload caddy`.

### 6. Google Cloud Console

In your OAuth 2.0 Client:
- **Authorized redirect URIs**: add `https://racquet.yourdomain.com/api/auth/google/callback`
- **Authorized JavaScript origins**: add `https://racquet.yourdomain.com`

### Going-live checklist

1. Create the `racquet` system user and `/opt/racquet/` directory.
2. Populate `.env` with real Google OAuth credentials and a strong `JWT_SECRET` (`openssl rand -hex 32`).
3. Copy `target/release/racquet` + `frontend/dist/` + `allowed_emails.txt` into `/opt/racquet/`.
4. Install the systemd unit (`deploy/racquet.service`) and `systemctl enable --now racquet`.
5. Add the Caddy block for `racquet.yourdomain.com` and reload Caddy.
6. Update Google Cloud Console redirect URIs + JS origins to the live domain.
7. Add a TURN server (coturn) only if someone actually can't connect over pure STUN.

### STUN/TURN

Google's public STUN server is already configured and is enough for most connections. Users behind strict NATs (mobile carriers, some corporate networks) may need a TURN relay. Add [coturn](https://github.com/coturn/coturn) on the same droplet if someone can't connect.

### Message cleanup

No auto-cleanup exists yet. SQLite is fine at this scale; if the DB ever grows, run:

```sql
DELETE FROM messages WHERE created_at < datetime('now', '-30 days');
VACUUM;
```

via cron.
