import { connection } from '$lib/stores/connection.svelte';

type MessageHandler = (msg: any) => void;

function wsBase(): string {
	const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${proto}//${window.location.host}`;
}

const INITIAL_BACKOFF_MS = 250;
const MAX_BACKOFF_MS = 10_000;
const AUTH_FAIL_AFTER_ATTEMPTS = 3; // give up if we never reach OPEN this many times in a row

export class WebSocketClient {
	private ws: WebSocket | null = null;
	private handlers: MessageHandler[] = [];
	private token: string | null = null;
	private currentRoomId: string | null = null;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private failedAttempts = 0;
	private hasOpenedOnce = false;
	private deliberatelyClosed = false;
	private onAuthFailureCallback: (() => void) | null = null;

	connect(token: string): void {
		this.token = token;
		this.deliberatelyClosed = false;
		this.openSocket();
	}

	setToken(token: string): void {
		this.token = token;
		// Force a reconnect with the new token.
		this.cancelReconnect();
		if (this.ws) this.ws.close();
		this.openSocket();
	}

	onAuthFailure(cb: () => void): void {
		this.onAuthFailureCallback = cb;
	}

	private openSocket(): void {
		if (!this.token) return;
		this.cancelReconnect();
		this.hasOpenedOnce = false;
		connection.set(this.failedAttempts === 0 ? 'connecting' : 'reconnecting');

		const ws = new WebSocket(`${wsBase()}/ws?token=${this.token}`);
		this.ws = ws;

		ws.onopen = () => {
			this.failedAttempts = 0;
			this.hasOpenedOnce = true;
			connection.set('open');
			if (this.currentRoomId) {
				this.send({ type: 'join_room', room_id: this.currentRoomId });
			}
		};

		ws.onmessage = (event) => {
			try {
				const msg = JSON.parse(event.data);
				for (const handler of this.handlers) handler(msg);
			} catch {
				// Ignore non-JSON messages
			}
		};

		ws.onerror = () => {};

		ws.onclose = () => {
			if (this.ws !== ws) return; // stale socket
			this.ws = null;
			if (this.deliberatelyClosed) {
				connection.set('closed');
				return;
			}
			if (!this.hasOpenedOnce) {
				this.failedAttempts++;
				if (this.failedAttempts >= AUTH_FAIL_AFTER_ATTEMPTS) {
					connection.set('auth_failed');
					this.onAuthFailureCallback?.();
					return;
				}
			} else {
				// Once we've opened at least once, retry indefinitely with backoff.
				this.failedAttempts++;
			}
			this.scheduleReconnect();
		};
	}

	private scheduleReconnect(): void {
		const base = Math.min(
			INITIAL_BACKOFF_MS * 2 ** Math.min(this.failedAttempts - 1, 8),
			MAX_BACKOFF_MS
		);
		const jitter = base * (0.8 + Math.random() * 0.4); // ±20%
		const delay = Math.round(jitter);
		const nextRetryAt = Date.now() + delay;
		connection.set('reconnecting', nextRetryAt);
		this.reconnectTimer = setTimeout(() => {
			this.reconnectTimer = null;
			this.openSocket();
		}, delay);
	}

	private cancelReconnect(): void {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
	}

	onMessage(handler: MessageHandler): () => void {
		this.handlers.push(handler);
		return () => {
			this.handlers = this.handlers.filter((h) => h !== handler);
		};
	}

	joinRoom(roomId: string): void {
		this.currentRoomId = roomId;
		this.send({ type: 'join_room', room_id: roomId });
	}

	leaveRoom(roomId: string): void {
		if (this.currentRoomId === roomId) this.currentRoomId = null;
		this.send({ type: 'leave_room', room_id: roomId });
	}

	sendMessage(roomId: string, content: string): void {
		this.send({ type: 'send_message', room_id: roomId, content });
	}

	sendOffer(roomId: string, targetUserId: string, payload: RTCSessionDescriptionInit): void {
		this.send({ type: 'offer', room_id: roomId, target_user_id: targetUserId, payload });
	}

	sendAnswer(roomId: string, targetUserId: string, payload: RTCSessionDescriptionInit): void {
		this.send({ type: 'answer', room_id: roomId, target_user_id: targetUserId, payload });
	}

	sendIceCandidate(roomId: string, targetUserId: string, payload: RTCIceCandidateInit): void {
		this.send({ type: 'ice_candidate', room_id: roomId, target_user_id: targetUserId, payload });
	}

	sendCallLeave(roomId: string, targetUserId: string): void {
		this.send({ type: 'call_leave', room_id: roomId, target_user_id: targetUserId, payload: {} });
	}

	sendScreenShareStart(roomId: string, streamId: string): void {
		this.send({
			type: 'screen_share_start',
			room_id: roomId,
			payload: { stream_id: streamId }
		});
	}

	sendScreenShareStop(roomId: string): void {
		this.send({ type: 'screen_share_stop', room_id: roomId, payload: {} });
	}

	private send(data: any): void {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(data));
		}
	}

	disconnect(): void {
		this.deliberatelyClosed = true;
		this.cancelReconnect();
		this.ws?.close();
		this.ws = null;
		this.currentRoomId = null;
		connection.set('closed');
	}
}
