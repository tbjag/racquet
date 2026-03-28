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
| Auth | `jsonwebtoken` (JWT) |
| Frontend framework | Svelte + TypeScript |
| Frontend build tool | Vite |
| Audio / Video | Browser WebRTC API (no Rust dependency) |
| STUN (local dev) | Google's free public STUN server |

## Dev Environment

- **OS**: Windows with WSL2 (Ubuntu)
- **Code editing and compilation**: inside WSL
- **Testing**: Windows browser (Chrome or Edge) at `http://localhost:3000`
- **State**: Phase 1 complete — backend server with auth, rooms, messages, and WebSocket hub is fully implemented and tested

WSL2 and Windows share `localhost`, so any port Axum binds to in WSL is immediately accessible from the Windows browser with no extra config.

### What runs where

| Process | Where it runs |
|---|---|
| `cargo run` (Axum server) | WSL |
| `npm run dev` (Vite dev server) | WSL |
| Browser / app UI | Windows (Chrome or Edge) |
| Mic / camera access | Windows browser — WSL never touches hardware |

### Day-to-day dev workflow

1. In WSL terminal 1: `cargo watch -x run` — auto-restarts Axum on Rust file changes
2. In WSL terminal 2: `npm run dev` inside the frontend folder — Vite hot-reloads on JS/TS/Svelte changes
3. Open `http://localhost:3000` in Windows browser
4. For multi-user testing: use Chrome + Edge simultaneously, logged in as different users

During development, Vite proxies `/api` and `/ws` requests to Axum so you don't deal with CORS. In production, Axum serves the compiled `/dist` output from `npm run build` directly as static files.

### Audio/video testing locally

- Two browser tabs share the same mic — awkward for audio testing
- **Two different browsers (Chrome + Edge)** is the recommended approach for testing calls
- WebRTC requires HTTPS for mic/camera in production, but `localhost` is exempt — plain HTTP works in dev

## Implementation Phases

### Phase 1 — Server Foundation ✅
- Axum server setup, `/api` REST routes, `/ws` WebSocket endpoint
- User registration and login with JWT
- SQLite schema: `users`, `rooms`, `messages`
- WebSocket connection manager (track connected users per room)

### Phase 2 — Chat
- Real-time message broadcast via WebSocket to all users in a room (server-side already implemented in Phase 1)
- REST endpoint to fetch message history on join (server-side already implemented in Phase 1)
- Frontend: room list, message input, scrollable chat history (not yet started — no frontend code exists in repo)

### Phase 3 — WebRTC Signaling
- Add signaling message types to the WebSocket protocol: `offer`, `answer`, `ice-candidate`
- Server relays signaling messages between peers (does not interpret them)
- Frontend: `RTCPeerConnection` + `getUserMedia` for P2P audio/video

### Phase 4 — Polish and Windows Build
- Frontend compiled to static files, served by Axum (single binary + static folder)
- Cross-compile from WSL: `cargo build --target x86_64-pc-windows-gnu`
- If C dependencies are added later, use `cross` (Docker-based) instead of raw cross-compilation

## Known Pitfalls

- **Do not use `webrtc-rs`** — previously caused build and testing issues in WSL. All WebRTC is handled by the browser.
- **WSL has no mic/camera access** — always test audio/video in a Windows browser, never from WSL directly.
- **P2P mesh for group calls** — at 5+ simultaneous video streams, client bandwidth becomes heavy. Acceptable for this scale but worth monitoring.
- **STUN/TURN**: P2P works on a local network without these. For users on different networks, a STUN server is required (use `stun:stun.l.google.com:19302`). Users behind strict NATs may also need a TURN relay.
- **Cross-compilation from WSL**: `x86_64-pc-windows-gnu` works for a pure Rust binary. If native C dependencies are introduced, switch to `cross`.