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
- **State**: Phase 3 complete — backend server + Svelte frontend with auth, rooms, real-time chat, and WebRTC audio/video calls

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

- 25 tests across 4 files: `auth.spec.ts`, `rooms.spec.ts`, `chat.spec.ts`, `webrtc.spec.ts`
- Tests use random usernames/room names so they don't depend on a clean DB
- For headed mode (see the browser): `npx playwright test --headed`
- For the interactive UI: `npx playwright test --ui`

### Audio/video testing locally

- Two browser tabs share the same mic — awkward for audio testing
- **Two different browsers (Chrome + Edge)** is the recommended approach for testing calls
- WebRTC requires HTTPS for mic/camera in production, but `localhost` is exempt — plain HTTP works in dev
- **Windows exclusive camera access**: only one browser can hold the camera at a time. The second browser falls back to audio-only automatically. Use a virtual webcam (e.g. OBS Virtual Camera) or a second machine for full two-way video testing.

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
- Signaling message types added to WebSocket protocol: `offer`, `answer`, `ice_candidate`, `call_leave`
- Server relays signaling messages to specific peers via `ConnectionManager::send_to_user()` (does not interpret payloads)
- `room_users` message sent on room join for peer discovery
- Frontend `WebRTCManager` (`frontend/src/lib/webrtc.ts`) handles full-mesh P2P connections with ICE candidate buffering
- Call UI: Join/Leave Call, Mute, Video Off buttons; local video preview; remote stream display
- Graceful media fallback: audio+video → audio-only → no media (handles exclusive camera access on Windows)
- 7 Playwright tests for WebRTC (including two-user P2P call e2e test), 6 Rust integration tests for signaling relay, 3 unit tests for `send_to_user`

### Phase 4 — Google OAuth + Email Whitelist ✅
- Google OAuth (authorization code flow) replaces username/password auth entirely
- Server-side email whitelist loaded from `allowed_emails.txt` (one email per line, gitignored)
- Login page shows "Sign in with Google" button; `/auth/callback` route stores JWT from redirect
- Users identified by Google email; username derived from Google profile name
- `dotenvy` loads `.env` for `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET`, etc.
- Test-only `POST /api/auth/test-login` endpoint (gated by `RACQUET_TEST_MODE=true`) for Playwright and Rust integration tests
- 24 Rust tests (8 unit + 16 integration), 17 Playwright tests

### Phase 5 — Single-Origin Prod Deploy ✅
- Frontend built with `@sveltejs/adapter-static` → `frontend/dist/` (SPA fallback on `index.html`)
- Axum serves `dist/` via `tower-http::services::ServeDir` with a `ServeFile` not-found service (SPA client-routing works for direct navigation like `/login`)
- Static dir is opt-in via `STATIC_DIR` env var — unset in dev so Vite owns the frontend on :5173
- Frontend calls use same-origin relative URLs; Vite proxies `/api` and `/ws` to Axum in dev
- CORS removed (same-origin in both dev and prod)
- Release profile: `lto = true`, `codegen-units = 1`, `strip = true`
- Deployment: Linux-only on a Digital Ocean droplet; Caddy terminates TLS and routes `racquet.tbjag.com` → `localhost:3000` (WebSocket upgrade is automatic in Caddy v2). systemd unit at `deploy/racquet.service`.

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