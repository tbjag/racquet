import type { WebSocketClient } from './ws';

const RTC_CONFIG: RTCConfiguration = {
	iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
};

export class WebRTCManager {
	private localStream: MediaStream | null = null;
	private peers: Map<string, RTCPeerConnection> = new Map();
	private pendingCandidates: Map<string, RTCIceCandidateInit[]> = new Map();
	private ws: WebSocketClient;
	private roomId: string;
	private onRemoteStream: (userId: string, username: string, stream: MediaStream) => void;
	private onPeerDisconnected: (userId: string) => void;

	constructor(
		ws: WebSocketClient,
		roomId: string,
		onRemoteStream: (userId: string, username: string, stream: MediaStream) => void,
		onPeerDisconnected: (userId: string) => void
	) {
		this.ws = ws;
		this.roomId = roomId;
		this.onRemoteStream = onRemoteStream;
		this.onPeerDisconnected = onPeerDisconnected;
	}

	async joinCall(): Promise<MediaStream | null> {
		// Try audio+video, fall back to audio-only, then no media
		// (Windows often gives exclusive camera access to one browser)
		try {
			this.localStream = await navigator.mediaDevices.getUserMedia({
				audio: true,
				video: true
			});
		} catch {
			try {
				console.warn('Could not access camera, falling back to audio-only');
				this.localStream = await navigator.mediaDevices.getUserMedia({
					audio: true,
					video: false
				});
			} catch {
				console.warn('Could not access any media devices, joining without media');
				this.localStream = null;
			}
		}
		return this.localStream;
	}

	getLocalStream(): MediaStream | null {
		return this.localStream;
	}

	async handlePeersInRoom(users: Array<{ userId: string; username: string }>): Promise<void> {
		for (const user of users) {
			await this.createPeerAndOffer(user.userId, user.username);
		}
	}

	private async createPeerAndOffer(targetUserId: string, targetUsername: string): Promise<void> {
		const pc = this.createPeerConnection(targetUserId, targetUsername);
		this.peers.set(targetUserId, pc);

		const offer = await pc.createOffer();
		await pc.setLocalDescription(offer);
		this.ws.sendOffer(this.roomId, targetUserId, pc.localDescription!.toJSON());
	}

	async handleOffer(
		fromUserId: string,
		fromUsername: string,
		payload: RTCSessionDescriptionInit
	): Promise<void> {
		const pc = this.createPeerConnection(fromUserId, fromUsername);
		this.peers.set(fromUserId, pc);

		await pc.setRemoteDescription(new RTCSessionDescription(payload));
		this.flushPendingCandidates(fromUserId, pc);

		const answer = await pc.createAnswer();
		await pc.setLocalDescription(answer);
		this.ws.sendAnswer(this.roomId, fromUserId, pc.localDescription!.toJSON());
	}

	async handleAnswer(fromUserId: string, payload: RTCSessionDescriptionInit): Promise<void> {
		const pc = this.peers.get(fromUserId);
		if (!pc) return;

		await pc.setRemoteDescription(new RTCSessionDescription(payload));
		this.flushPendingCandidates(fromUserId, pc);
	}

	async handleIceCandidate(fromUserId: string, payload: RTCIceCandidateInit): Promise<void> {
		const pc = this.peers.get(fromUserId);
		if (pc && pc.remoteDescription) {
			await pc.addIceCandidate(new RTCIceCandidate(payload));
		} else {
			// Buffer candidate until remote description is set
			if (!this.pendingCandidates.has(fromUserId)) {
				this.pendingCandidates.set(fromUserId, []);
			}
			this.pendingCandidates.get(fromUserId)!.push(payload);
		}
	}

	removePeer(userId: string): void {
		const pc = this.peers.get(userId);
		if (pc) {
			pc.close();
			this.peers.delete(userId);
		}
		this.pendingCandidates.delete(userId);
		this.onPeerDisconnected(userId);
	}

	leaveCall(): void {
		// Notify each peer that we're leaving
		for (const [userId] of this.peers) {
			this.ws.sendCallLeave(this.roomId, userId);
		}
		for (const [, pc] of this.peers) {
			pc.close();
		}
		this.peers.clear();
		this.pendingCandidates.clear();

		if (this.localStream) {
			for (const track of this.localStream.getTracks()) {
				track.stop();
			}
			this.localStream = null;
		}
	}

	private createPeerConnection(remoteUserId: string, remoteUsername: string): RTCPeerConnection {
		const pc = new RTCPeerConnection(RTC_CONFIG);

		// Add local tracks
		if (this.localStream) {
			for (const track of this.localStream.getTracks()) {
				pc.addTrack(track, this.localStream);
			}
		}

		// Send ICE candidates to remote peer
		pc.onicecandidate = (event) => {
			if (event.candidate) {
				this.ws.sendIceCandidate(this.roomId, remoteUserId, event.candidate.toJSON());
			}
		};

		// Handle incoming remote tracks
		pc.ontrack = (event) => {
			if (event.streams[0]) {
				this.onRemoteStream(remoteUserId, remoteUsername, event.streams[0]);
			}
		};

		// Handle connection state changes
		pc.onconnectionstatechange = () => {
			if (
				pc.connectionState === 'failed' ||
				pc.connectionState === 'disconnected' ||
				pc.connectionState === 'closed'
			) {
				this.peers.delete(remoteUserId);
				this.pendingCandidates.delete(remoteUserId);
				this.onPeerDisconnected(remoteUserId);
			}
		};

		return pc;
	}

	private flushPendingCandidates(userId: string, pc: RTCPeerConnection): void {
		const candidates = this.pendingCandidates.get(userId);
		if (candidates) {
			for (const candidate of candidates) {
				pc.addIceCandidate(new RTCIceCandidate(candidate));
			}
			this.pendingCandidates.delete(userId);
		}
	}
}
