import type { WebSocketClient } from './ws';

const RTC_CONFIG: RTCConfiguration = {
	iceServers: [{ urls: 'stun:stun.l.google.com:19302' }]
};

export class WebRTCManager {
	private localStream: MediaStream | null = null;
	private screenStream: MediaStream | null = null;
	private peers: Map<string, RTCPeerConnection> = new Map();
	private pendingCandidates: Map<string, RTCIceCandidateInit[]> = new Map();
	// Stream IDs the receiver has been told are screen shares (via screen_share_started).
	// Populated before the renegotiation offer arrives so ontrack can classify them.
	private remoteScreenStreamIds: Set<string> = new Set();
	private ws: WebSocketClient;
	private roomId: string;
	private onRemoteStream: (userId: string, username: string, stream: MediaStream) => void;
	private onPeerDisconnected: (userId: string) => void;
	private onRemoteScreenStream?: (userId: string, username: string, stream: MediaStream) => void;
	private onScreenShareEnded?: () => void;

	constructor(
		ws: WebSocketClient,
		roomId: string,
		onRemoteStream: (userId: string, username: string, stream: MediaStream) => void,
		onPeerDisconnected: (userId: string) => void,
		onRemoteScreenStream?: (userId: string, username: string, stream: MediaStream) => void,
		onScreenShareEnded?: () => void
	) {
		this.ws = ws;
		this.roomId = roomId;
		this.onRemoteStream = onRemoteStream;
		this.onPeerDisconnected = onPeerDisconnected;
		this.onRemoteScreenStream = onRemoteScreenStream;
		this.onScreenShareEnded = onScreenShareEnded;
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

	getScreenStream(): MediaStream | null {
		return this.screenStream;
	}

	registerRemoteScreenStreamId(streamId: string): void {
		this.remoteScreenStreamIds.add(streamId);
	}

	unregisterRemoteScreenStreamId(streamId: string): void {
		this.remoteScreenStreamIds.delete(streamId);
	}

	/// Acquires a display-media stream but does not yet add it to peers.
	/// Caller should send screen_share_start (so receivers know the stream id)
	/// then call broadcastScreenStream() to renegotiate.
	///
	/// `mode` tunes the encoder for the content type:
	///   - 'motion' (default): 30 fps ideal, contentHint='motion' — for videos / gameplay.
	///   - 'detail': 5–15 fps, contentHint='detail' — for code / docs (sharper text,
	///     bitrate spent on per-frame quality not temporal smoothness).
	async acquireScreenStream(mode: 'motion' | 'detail' = 'motion'): Promise<MediaStream | null> {
		const frameRate = mode === 'detail' ? { ideal: 5, max: 15 } : { ideal: 30 };
		try {
			this.screenStream = await navigator.mediaDevices.getDisplayMedia({
				video: { frameRate },
				audio: true
			});
		} catch {
			this.screenStream = null;
			return null;
		}

		const videoTrack = this.screenStream.getVideoTracks()[0];
		if (videoTrack) {
			videoTrack.contentHint = mode;
			// User-initiated stop (browser's "Stop sharing" bar) ends the video track.
			videoTrack.onended = () => {
				if (this.onScreenShareEnded) this.onScreenShareEnded();
			};
		}

		return this.screenStream;
	}

	/// Live-update the encoder hint on the active screen track. No renegotiation
	/// needed — receivers don't see anything change at the signaling layer.
	setScreenShareMode(mode: 'motion' | 'detail'): void {
		if (!this.screenStream) return;
		const videoTrack = this.screenStream.getVideoTracks()[0];
		if (videoTrack) videoTrack.contentHint = mode;
	}

	async broadcastScreenStream(): Promise<void> {
		if (!this.screenStream) return;
		for (const [userId, pc] of this.peers) {
			for (const track of this.screenStream.getTracks()) {
				pc.addTrack(track, this.screenStream);
			}
			await this.renegotiate(userId, pc);
		}
	}

	async stopScreenShare(): Promise<void> {
		if (!this.screenStream) return;

		const screenTracks = this.screenStream.getTracks();
		for (const [userId, pc] of this.peers) {
			for (const sender of pc.getSenders()) {
				if (sender.track && screenTracks.includes(sender.track)) {
					pc.removeTrack(sender);
				}
			}
			await this.renegotiate(userId, pc);
		}

		for (const track of screenTracks) {
			track.stop();
		}
		this.screenStream = null;
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

	private async renegotiate(targetUserId: string, pc: RTCPeerConnection): Promise<void> {
		const offer = await pc.createOffer();
		await pc.setLocalDescription(offer);
		this.ws.sendOffer(this.roomId, targetUserId, pc.localDescription!.toJSON());
	}

	async handleOffer(
		fromUserId: string,
		fromUsername: string,
		payload: RTCSessionDescriptionInit
	): Promise<void> {
		// Reuse the existing peer connection if one already exists — re-offers
		// from renegotiation (e.g. starting a screen share) arrive on the same
		// fromUserId and must not blow away the active camera/audio tracks.
		let pc = this.peers.get(fromUserId);
		if (!pc) {
			pc = this.createPeerConnection(fromUserId, fromUsername);
			this.peers.set(fromUserId, pc);
		}

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
		this.remoteScreenStreamIds.clear();

		if (this.localStream) {
			for (const track of this.localStream.getTracks()) {
				track.stop();
			}
			this.localStream = null;
		}
		if (this.screenStream) {
			for (const track of this.screenStream.getTracks()) {
				track.stop();
			}
			this.screenStream = null;
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
		// Late-joining peers should also receive the screen if a share is active
		if (this.screenStream) {
			for (const track of this.screenStream.getTracks()) {
				pc.addTrack(track, this.screenStream);
			}
		}

		// Send ICE candidates to remote peer
		pc.onicecandidate = (event) => {
			if (event.candidate) {
				this.ws.sendIceCandidate(this.roomId, remoteUserId, event.candidate.toJSON());
			}
		};

		// Handle incoming remote tracks — classify as screen vs camera by stream id.
		pc.ontrack = (event) => {
			const stream = event.streams[0];
			if (!stream) return;
			if (this.remoteScreenStreamIds.has(stream.id)) {
				if (this.onRemoteScreenStream) {
					this.onRemoteScreenStream(remoteUserId, remoteUsername, stream);
				}
			} else {
				this.onRemoteStream(remoteUserId, remoteUsername, stream);
			}
		};

		// Handle terminal connection states only. "disconnected" is transient
		// (can recover) and renegotiation may flap through it briefly.
		pc.onconnectionstatechange = () => {
			if (pc.connectionState === 'failed' || pc.connectionState === 'closed') {
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
