type MessageHandler = (msg: any) => void;

function wsBase(): string {
	const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	return `${proto}//${window.location.host}`;
}

export class WebSocketClient {
	private ws: WebSocket | null = null;
	private handlers: MessageHandler[] = [];

	connect(token: string): void {
		if (this.ws) this.ws.close();

		this.ws = new WebSocket(`${wsBase()}/ws?token=${token}`);

		this.ws.onmessage = (event) => {
			try {
				const msg = JSON.parse(event.data);
				for (const handler of this.handlers) {
					handler(msg);
				}
			} catch {
				// Ignore non-JSON messages
			}
		};

		this.ws.onerror = () => {};
		this.ws.onclose = () => {};
	}

	onMessage(handler: MessageHandler): () => void {
		this.handlers.push(handler);
		return () => {
			this.handlers = this.handlers.filter((h) => h !== handler);
		};
	}

	joinRoom(roomId: string): void {
		this.send({ type: 'join_room', room_id: roomId });
	}

	leaveRoom(roomId: string): void {
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
		this.ws?.close();
		this.ws = null;
	}
}
