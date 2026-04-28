# CLAUDE.md — Project Context

## What This Is

A lightweight Discord alternative for a small friend group (2–10 people), built in Rust.
Features: text chat, audio, video, and screen share. Nothing more.

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
cd frontend && ./node_modules/.bin/playwright test
```

Use the local binary, not `npx playwright` — npx silently downloads a newer global Playwright that mismatches the local install and then every spec fails with "Playwright Test did not expect test.describe() to be called here". See the resolved-blocker note in Phase 7 for details.

Playwright auto-starts both the backend (port 3000) and frontend (port 5173) via `webServer` config. Tests use a separate SQLite DB at `/tmp/racquet-e2e.db`. If servers are already running, they are reused.

- 25 tests across 4 files: `auth.spec.ts`, `rooms.spec.ts`, `chat.spec.ts`, `webrtc.spec.ts`
- Tests use random usernames/room names so they don't depend on a clean DB
- For headed mode (see the browser): `./node_modules/.bin/playwright test --headed`
- For the interactive UI: `./node_modules/.bin/playwright test --ui`

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
- Live deployment: `https://racquet.tbjag.com`, served from `/opt/racquet/` on the droplet (binary, `dist/`, `.env`, `allowed_emails.txt`, `racquet.db`). Cloudflare DNS-only (grey cloud) so Caddy gets its own Let's Encrypt cert via HTTP-01. Pushes via `deploy/deploy.sh` from WSL (cross-compiles static musl binary, rsyncs, restarts systemd unit).

### Phase 6 — Screen Share ✅
- One sharer at a time per room, enforced server-side via `active_screen_sharer` map in `ConnectionManager`. Concurrent attempts get an `error` reply; the slot auto-clears on disconnect with a `screen_share_stopped` broadcast.
- Two new WS message types from client → server: `screen_share_start` (payload `{ stream_id }`) and `screen_share_stop`. Server broadcasts `screen_share_started` / `screen_share_stopped` to the whole room.
- Sender keeps the camera stream running; screen tracks are added as a separate stream and broadcast via WebRTC renegotiation (addTrack + new offer over the existing signaling path). Audio capture (`getDisplayMedia({ audio: true })`) is included when the user opts in via the browser picker.
- Receivers classify incoming streams as screen vs camera by matching the `stream_id` from the `screen_share_started` event against `event.streams[0].id` in `pc.ontrack`. Critical ordering: the UI sends `screen_share_start` *before* renegotiating, so receivers have the id registered when the new tracks arrive.
- Renegotiation exposed two pre-existing bugs that are now fixed: `WebRTCManager.handleOffer` reuses the existing `RTCPeerConnection` for re-offers (was creating a new one each time), and `onconnectionstatechange` only treats `failed`/`closed` as terminal (`disconnected` flickers during renegotiation).
- UI: `Share Screen` / `Stop Sharing` button next to mute/video; large focused `screen-share-tile` above the camera tiles when any user is sharing; B's button is disabled while A is sharing. `data-testid` selectors `screen-share-button`, `screen-share-tile`.
- 4 new Rust integration tests + 5 new Playwright tests. Total now 29 Rust + 32 Playwright.

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
- **`reqwest` must use `rustls-tls`, not `native-tls`** — the deploy build cross-compiles to `x86_64-unknown-linux-musl`, and `native-tls` pulls in `openssl-sys` which fails because Ubuntu has no prebuilt OpenSSL for musl. The dependency is declared as `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }` — do not add features without keeping `default-features = false`, or you'll silently re-enable native-tls and break the deploy build.
- **Frontend URLs must be same-origin (empty string base)** — there is one `API_BASE` in `frontend/src/lib/api.ts` (correctly `''`). Don't introduce a second `API_BASE` constant in a route file with a hardcoded `http://localhost:3000`; in prod this resolves to a literal `localhost:3000` link in the user's browser. Vite's dev proxy makes the bug invisible until you hit production.
- **Renegotiation ordering for screen share** — the sender must send `screen_share_start` (with the stream id) over the WebSocket *before* calling `WebRTCManager.broadcastScreenStream()` (which sends the renegotiation offer). The receiver's `pc.ontrack` fires synchronously inside `setRemoteDescription`, and it classifies incoming streams as screen-vs-camera by checking the stream id against `remoteScreenStreamIds`. If the offer arrives before the `screen_share_started` notify, the screen stream gets misrouted to the camera tile (and the camera entry gets overwritten because `remoteStreams` is keyed by user id). The split `acquireScreenStream` / `broadcastScreenStream` API in `webrtc.ts` exists specifically to enforce this order — don't collapse it back into one method.
- **Screen share tests in headless Chromium** — `getDisplayMedia` doesn't have a fake source under the existing Playwright launch flags. `frontend/tests/webrtc.spec.ts` stubs `navigator.mediaDevices.getDisplayMedia` via `page.addInitScript` to delegate to `getUserMedia`, so the tests exercise UI + signaling end-to-end. If you ever want a real screen-capture test, add `--auto-select-desktop-capture-source=Entire` to the Chromium launch args.

## Phase 7 — Frontend refactor (in progress)

Working plan: `/home/tbjag/.claude/plans/ok-i-now-want-atomic-moth.md`. Goal: turn the unstyled, monolithic `+page.svelte` into a themed, componentized desktop client with visible feedback for in-flight work, errors, and connection state. Desktop-only, light + dark themes (default = follow system), Svelte scoped `<style>` blocks, no Tailwind. Committing to main as work progresses (no PRs).

### Completed commits

1. **Theme infrastructure** (`5b7453d`) — `frontend/src/app.css` with CSS custom properties for both `[data-theme='light']` and `[data-theme='dark']`; `frontend/src/lib/stores/theme.svelte.ts` (rune store, tri-state `mode: 'light'|'dark'|'system'` persisted to `localStorage.racquet_theme`); `frontend/src/lib/components/chrome/ThemeToggle.svelte`; FOUC mitigation via inline `<script>` in `frontend/src/app.html` that sets `documentElement.dataset.theme` synchronously on first paint. 5 Playwright tests in `frontend/tests/theme.spec.ts`.
2. **Toast + apiCall** (`fe57957`) — `frontend/src/lib/stores/toast.svelte.ts` (rune array + `pushToast`/`dismiss`); `frontend/src/lib/components/chrome/ToastHost.svelte` (fixed bottom-right, click to dismiss, `aria-live='polite'`); `frontend/src/lib/apiCall.ts` (wraps a promise, toasts on throw, returns `null`). All API calls in `+page.svelte` now go through `apiCall` with human-readable error messages. Error TTL 8000ms, info/success 5000ms. 4 Playwright tests in `frontend/tests/errors.spec.ts`.
3. **Sidebar extraction** — `frontend/src/lib/components/sidebar/{UserProfile,RoomList,CreateRoomForm}.svelte`. `CreateRoomForm` owns its own input state + `submitting` flag and disables both input and button during the in-flight POST (button text flips to `Creating…`). Submit also disabled when name is empty/whitespace. `+page.svelte`'s `handleCreateRoom` and `handleSaveName` were reshaped to take a `name` arg and return `Promise<boolean>` so the children can react to success/failure. All existing `data-testid` selectors preserved for regression coverage. 2 Playwright tests in `frontend/tests/loading.spec.ts` (43 total now).
4. **Chat extraction** — `frontend/src/lib/components/room/{RoomHeader,MessageList,MessageInput}.svelte`. `MessageList` owns the scrollable container, tracks a `stickToBottom` flag from the scroll handler (within 80px of bottom = sticky), and only auto-scrolls on new messages when sticky. `MessageInput` owns its own input state and Enter handling and calls back to the parent via `onSend(text)`. New `RoomHeader` shows the selected room name (`data-testid="room-header"`). The page now has a minimum scoped layout (`.app` flex 100vh, `.chat-area` flex column with `min-height: 0`) so `MessageList` can actually overflow — full styling pass is still commit 8. 1 Playwright test in `frontend/tests/scroll.spec.ts` (44 total now).
5. **Call extraction** — `frontend/src/lib/components/call/{VideoTile,CallControls,CallStage}.svelte`. `VideoTile` is the reusable `<video>` wrapper (owns the `bindStream` action that was inlined in `+page.svelte` as `setStream`). `CallControls` is the button row, takes `inCall`/`audioMuted`/`videoMuted`/`isSharing`/`canShare` flags + four `onToggle*` callbacks. `CallStage` arranges the screen-share-tile + local-video + remote-streams grid and exports a `RemotePeer` type. `+page.svelte`'s `remoteStreams` switched from `Map<string, {...}>` to `RemotePeer[]` so the `{#each}` is straightforward; add/remove use `filter` instead of `new Map(...)` ceremony. Pure refactor — no new tests; webrtc.spec.ts (12) is the regression net. Suite 44/44 green.

### Remaining commits (planned)

6. Member list UI (`MemberList.svelte`) + `frontend/tests/members.spec.ts` — `roomUsers` is already populated, just never rendered.
7. WebSocket reconnect: `lib/stores/connection.ts`, `ws.ts` state machine (closed → connecting → open / reconnecting with exponential backoff + jitter), `setToken()` method, `onAuthFailure` callback, re-join `currentRoomId` on reconnect, `ConnectionBanner.svelte`. Tear down active call on socket loss. No Playwright test (too flaky); manual verification.
8. Visual styling pass — populate scoped `<style>` blocks per component using CSS variables. Order: sidebar → chat → call stage → auth pages. Cap scope.
9. Polish — focus rings, hover/transition states, empty states, scrollbar styling, dark contrast verification.

### Resolved: Playwright test-discovery blocker (real root cause)

`npx playwright test` was downloading a newer Playwright (e.g. 1.59.1) into `~/.npm/_npx/...` and running it against the locally-installed 1.58.2 — that's the "two different versions of @playwright/test" error message taken literally. Symptom is `Playwright Test did not expect test.describe() to be called here` for *every* spec, including unmodified ones. Recovery: invoke the local binary directly — `./node_modules/.bin/playwright test` (run from `frontend/`). The cache-clear that "fixed" it during commit 3 worked only because the cache was wiped *and* npx happened to redownload a matching version on the next run; clearing caches is not a reliable fix. Don't use `npx playwright` in this repo.

### Conventions established this phase

- Stores live in `frontend/src/lib/stores/<name>.svelte.ts` (must use `.svelte.ts` extension to use runes outside components).
- Components grouped by purpose: `lib/components/chrome/` (app-wide UI like toast/banner/theme), `lib/components/sidebar/`, `lib/components/room/`, `lib/components/call/`.
- API failures: wrap callsite in `apiCall(() => fn(), { errorMessage: '...' })` and check the return for `null`. Don't add try/catch in `api.ts` itself.
- Test IDs: keep all existing `data-testid` selectors when extracting components — they're the regression net for the 32 pre-existing Playwright specs.
- New CSS values: always reference variables from `app.css` (`var(--bg)`, `var(--text)`, `var(--space-3)`, etc.). No hardcoded colors or spacing.