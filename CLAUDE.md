# CLAUDE.md — Project Context

## What This Is

A lightweight Discord alternative for a small friend group (2–10 people), built in Rust.
Features: text chat, audio, and video. Nothing more.

## Architecture

This is a **client-server** application:

- **Rust Axum server** — central backend that handles signaling, chat, and serves the frontend as static files
- **TypeScript/Svelte frontend** — runs in the browser, handles all audio/video via browser WebRTC APIs
- **No media processing in Rust** — the server only relays WebRTC signaling messages (SDP offers/answers, ICE candidates), never touches audio or video

### Why this shape

`webrtc-rs` was attempted previously and caused build/testing problems in WSL. The solution is to offload all WebRTC work to the browser's native WebRTC API, which has no Rust dependencies and works out of the box on Windows.

### Diagram

```
[Browser: User A]                    [Browser: User B]
  WebRTC audio/video (P2P)  <------>  WebRTC audio/video (P2P)
  Svelte + TypeScript UI               Svelte + TypeScript UI
       |  ^                                 |  ^
  WS   |  | signaling signals          WS  |  | signaling signals
       v  |                                 v  |
     +-----------------------------------------------+
     |           Rust Axum Server                    |
     |  - WebSocket hub (tokio-tungstenite)          |
     |  - REST API (axum)                            |
     |  - Chat history (SQLite via sqlx)             |
     |  - Room & user management                     |
     |  - Serves /static frontend files              |
     +-----------------------------------------------+
```

Audio and video flow directly browser-to-browser (P2P mesh). The server only passes small signaling messages to coordinate connections.

## Tech Stack

| Layer | Choice |
|---|---|
| Async runtime | `tokio` |
| HTTP + REST | `axum` |
| WebSockets | `tokio-tungstenite` |
| Database | `sqlx` + SQLite |
| Auth | Google OAuth + `jsonwebtoken` (JWT sessions) |
| Frontend framework | SvelteKit 2 + Svelte 5 (runes mode) + TypeScript |
| Frontend build tool | Vite 7 |
| Integration tests | Playwright (Chromium) |
| Audio / Video | Browser WebRTC API (no Rust dependency) |
| STUN (local dev) | Google's free public STUN server |

## Dev Environment

- **OS**: Windows with WSL2 (Ubuntu)
- **Code editing and compilation**: inside WSL
- **Testing**: Windows browser (Chrome or Edge) at `http://localhost:5173` (Vite dev server)
- **Integration tests**: Playwright (Chromium headless) — runs against real backend + frontend
- **State**: Phase 4 complete — backend server + Svelte frontend with Google OAuth, rooms, real-time chat, and WebRTC signaling

WSL2 and Windows share `localhost`, so any port Axum binds to in WSL is immediately accessible from the Windows browser with no extra config.

### What runs where

| Process | Where it runs |
|---|---|
| `cargo run` (Axum server) | WSL |
| `npm run dev` (Vite dev server) | WSL |
| Browser / app UI | Windows (Chrome or Edge) |
| Mic / camera access | Windows browser — WSL never touches hardware |

### Day-to-day dev workflow

1. Copy `.env.example` to `.env` and fill in `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`
2. Add allowed emails to `allowed_emails.txt` (one per line)
3. In WSL terminal 1: `cargo run` (or `cargo watch -x run`) — starts Axum on port 3000 (auto-loads `.env` via `dotenvy`)
4. In WSL terminal 2: `cd frontend && npm run dev` — Vite dev server on port 5173
5. Open `http://localhost:5173` in Windows browser
6. For multi-user testing: use Chrome + Edge simultaneously, logged in as different Google accounts

The frontend calls the backend directly at `localhost:3000` (configured in `frontend/src/lib/api.ts` and `frontend/src/lib/ws.ts`). CORS is enabled on the backend via `tower-http`. In production, Axum serves the compiled `/dist` output from `npm run build` directly as static files.

### Running integration tests

```bash
cd frontend && npx playwright test
```

Playwright auto-starts both the backend (port 3000) and frontend (port 5173) via `webServer` config. Tests use a separate SQLite DB at `/tmp/racquet-e2e.db`. If servers are already running, they are reused.

- 17 tests across 3 files: `auth.spec.ts`, `rooms.spec.ts`, `chat.spec.ts`
- Tests use a `POST /api/auth/test-login` endpoint (enabled by `RACQUET_TEST_MODE=true`) to bypass Google OAuth
- Tests use random emails/room names so they don't depend on a clean DB
- For headed mode (see the browser): `npx playwright test --headed`
- For the interactive UI: `npx playwright test --ui`

### Audio/video testing locally

- Two browser tabs share the same mic — awkward for audio testing
- **Two different browsers (Chrome + Edge)** is the recommended approach for testing calls
- WebRTC requires HTTPS for mic/camera in production, but `localhost` is exempt — plain HTTP works in dev

## Implementation Phases

### Phase 1 — Server Foundation ✅
- Axum server setup, `/api` REST routes, `/ws` WebSocket endpoint
- SQLite schema: `users`, `rooms`, `messages`
- WebSocket connection manager (track connected users per room)

### Phase 2 — Chat ✅
- SvelteKit frontend (`frontend/`) with SSR disabled (client-side SPA)
- Room list sidebar with create-room form
- Real-time chat via WebSocket (messages broadcast to all users in a room)
- Message history loaded via REST on room join

### Phase 3 — WebRTC Signaling ✅
- Signaling message types added to WebSocket protocol: `offer`, `answer`, `ice-candidate`
- Server relays signaling messages between peers (does not interpret them)
- Frontend: `RTCPeerConnection` + `getUserMedia` for P2P audio/video

### Phase 4 — Google OAuth + Email Whitelist ✅
- Google OAuth (authorization code flow) replaces username/password auth entirely
- Server-side email whitelist loaded from `allowed_emails.txt` (one email per line, gitignored)
- Login page shows "Sign in with Google" button; `/auth/callback` route stores JWT from redirect
- Users identified by Google email; username derived from Google profile name
- `dotenvy` loads `.env` for `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, etc.
- Test-only `POST /api/auth/test-login` endpoint (gated by `RACQUET_TEST_MODE=true`) for Playwright and Rust integration tests
- 24 Rust tests (8 unit + 16 integration), 17 Playwright tests

### Phase 5 — Polish and Windows Build
- Frontend compiled to static files, served by Axum (single binary + static folder)
- Cross-compile from WSL: `cargo build --target x86_64-pc-windows-gnu`
- If C dependencies are added later, use `cross` (Docker-based) instead of raw cross-compilation

## Production Deployment Notes

- **HTTPS is required** — Google OAuth rejects non-localhost HTTP redirect URIs, and WebRTC mic/camera access requires a secure context
- **Google Cloud Console** — add the production redirect URI (`https://yourdomain.com/api/auth/google/callback`) and JS origin (`https://yourdomain.com`)
- **Environment variables** — set via `.env` file next to the binary, systemd `EnvironmentFile=`, `docker --env-file`, or cloud platform secrets dashboard
- **Update in production**: `GOOGLE_REDIRECT_URI`, `FRONTEND_URL`, and `JWT_SECRET` (use a strong random value)
- **Simplest deployment** — single VPS with the binary, `.env`, `allowed_emails.txt`, and the frontend `dist/` folder

## Known Pitfalls

- **Do not use `webrtc-rs`** — previously caused build and testing issues in WSL. All WebRTC is handled by the browser.
- **WSL has no mic/camera access** — always test audio/video in a Windows browser, never from WSL directly.
- **P2P mesh for group calls** — at 5+ simultaneous video streams, client bandwidth becomes heavy. Acceptable for this scale but worth monitoring.
- **STUN/TURN**: P2P works on a local network without these. For users on different networks, a STUN server is required (use `stun:stun.l.google.com:19302`). Users behind strict NATs may also need a TURN relay.
- **Cross-compilation from WSL**: `x86_64-pc-windows-gnu` works for a pure Rust binary. If native C dependencies are introduced, switch to `cross`.