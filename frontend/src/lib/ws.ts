const WS_BASE = 'ws://localhost:3000';

type MessageHandler = (msg: any) => void;

export class WebSocketClient {
	private ws: WebSocket | null = null;
	private handlers: MessageHandler[] = [];

	connect(token: string): void {
		if (this.ws) this.ws.close();

		this.ws = new WebSocket(`${WS_BASE}/ws?token=${token}`);

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
