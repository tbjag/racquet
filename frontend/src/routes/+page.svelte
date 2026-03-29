<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { getToken } from '$lib/auth';
	import { getRooms, createRoom, getMessages } from '$lib/api';
	import { WebSocketClient } from '$lib/ws';
	import { WebRTCManager } from '$lib/webrtc';

	type Room = { id: string; name: string; created_by: string; created_at: string };
	type Message = {
		id: string;
		room_id: string;
		user_id: string;
		username: string;
		content: string;
		created_at: string;
	};

	let token = $state<string | null>(null);
	let rooms = $state<Room[]>([]);
	let selectedRoomId = $state<string | null>(null);
	let messages = $state<Message[]>([]);
	let messageInput = $state('');
	let showCreateForm = $state(false);
	let newRoomName = $state('');
	let ws: WebSocketClient | null = null;
	let currentRoomId: string | null = null;

	// WebRTC state
	let inCall = $state(false);
	let localStream = $state<MediaStream | null>(null);
	let remoteStreams = $state<Map<string, { username: string; stream: MediaStream }>>(new Map());
	let webrtcManager: WebRTCManager | null = null;
	let audioMuted = $state(false);
	let videoMuted = $state(false);
	let roomUsers = $state<Map<string, string>>(new Map()); // userId -> username

	onMount(async () => {
		token = getToken();
		if (!token) {
			goto('/login');
			return;
		}

		ws = new WebSocketClient();
		ws.connect(token);
		ws.onMessage(handleWsMessage);

		await loadRooms();
	});

	onDestroy(() => {
		ws?.disconnect();
	});

	function handleWsMessage(msg: any) {
		if (msg.type === 'new_message' && msg.room_id === selectedRoomId) {
			messages = [...messages, msg];
		} else if (msg.type === 'room_users' && msg.room_id === selectedRoomId) {
			const newUsers = new Map<string, string>();
			for (const u of msg.users) {
				newUsers.set(u.user_id, u.username);
			}
			roomUsers = newUsers;
		} else if (msg.type === 'user_joined' && msg.room_id === selectedRoomId) {
			roomUsers = new Map([...roomUsers, [msg.user_id, msg.username]]);
		} else if (msg.type === 'user_left' && msg.room_id === selectedRoomId) {
			const updated = new Map(roomUsers);
			updated.delete(msg.user_id);
			roomUsers = updated;
			// Clean up peer connection if in call
			if (inCall && webrtcManager) {
				webrtcManager.removePeer(msg.user_id);
			}
		} else if (msg.type === 'offer') {
			webrtcManager?.handleOffer(msg.from_user_id, msg.from_username, msg.payload);
		} else if (msg.type === 'answer') {
			webrtcManager?.handleAnswer(msg.from_user_id, msg.payload);
		} else if (msg.type === 'ice_candidate') {
			webrtcManager?.handleIceCandidate(msg.from_user_id, msg.payload);
		} else if (msg.type === 'call_leave') {
			if (inCall && webrtcManager) {
				webrtcManager.removePeer(msg.from_user_id);
			}
		}
	}

	async function loadRooms() {
		if (!token) return;
		rooms = await getRooms(token);
	}

	async function selectRoom(roomId: string) {
		if (!token || !ws) return;

		// Leave current room and clean up call
		if (currentRoomId) {
			if (inCall) leaveCall();
			ws.leaveRoom(currentRoomId);
		}

		selectedRoomId = roomId;
		currentRoomId = roomId;
		roomUsers = new Map();
		ws.joinRoom(roomId);

		const history = await getMessages(token, roomId);
		messages = history.reverse();
	}

	async function handleCreateRoom(e: Event) {
		e.preventDefault();
		if (!token || !newRoomName.trim()) return;

		await createRoom(token, newRoomName.trim());
		newRoomName = '';
		showCreateForm = false;
		await loadRooms();
	}

	function handleSendMessage(e: Event) {
		e.preventDefault();
		if (!ws || !selectedRoomId || !messageInput.trim()) return;

		ws.sendMessage(selectedRoomId, messageInput.trim());
		messageInput = '';
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSendMessage(e);
		}
	}

	async function joinCall() {
		if (!ws || !selectedRoomId) return;

		webrtcManager = new WebRTCManager(
			ws,
			selectedRoomId,
			(userId, username, stream) => {
				remoteStreams = new Map([...remoteStreams, [userId, { username, stream }]]);
			},
			(userId) => {
				const updated = new Map(remoteStreams);
				updated.delete(userId);
				remoteStreams = updated;
			}
		);

		localStream = await webrtcManager.joinCall();
		inCall = true;
		audioMuted = false;
		videoMuted = false;

		// Send offers to all users already in the room
		const peers = Array.from(roomUsers.entries()).map(([userId, username]) => ({
			userId,
			username
		}));
		await webrtcManager.handlePeersInRoom(peers);
	}

	function leaveCall() {
		if (webrtcManager) {
			webrtcManager.leaveCall();
			webrtcManager = null;
		}
		localStream = null;
		remoteStreams = new Map();
		inCall = false;
		audioMuted = false;
		videoMuted = false;
	}

	function toggleMute() {
		if (!localStream) return;
		audioMuted = !audioMuted;
		for (const track of localStream.getAudioTracks()) {
			track.enabled = !audioMuted;
		}
	}

	function toggleVideo() {
		if (!localStream) return;
		videoMuted = !videoMuted;
		for (const track of localStream.getVideoTracks()) {
			track.enabled = !videoMuted;
		}
	}

	function setStream(node: HTMLVideoElement, stream: MediaStream) {
		node.srcObject = stream;
		return {
			update(newStream: MediaStream) {
				node.srcObject = newStream;
			},
			destroy() {
				node.srcObject = null;
			}
		};
	}
</script>

<div class="app">
	<aside class="sidebar">
		<div class="sidebar-header">
			<h2>Rooms</h2>
			<button data-testid="create-room-button" onclick={() => (showCreateForm = !showCreateForm)}>+</button>
		</div>

		{#if showCreateForm}
			<form onsubmit={handleCreateRoom} class="create-room-form">
				<input
					data-testid="room-name-input"
					type="text"
					placeholder="Room name"
					bind:value={newRoomName}
				/>
				<button data-testid="room-submit-button" type="submit">Create</button>
			</form>
		{/if}

		<div data-testid="room-list">
			{#if rooms.length === 0}
				<p data-testid="no-rooms-placeholder">No rooms yet</p>
			{:else}
				{#each rooms as room}
					<button
						data-testid="room-item"
						class="room-item {selectedRoomId === room.id ? 'active' : ''}"
						onclick={() => selectRoom(room.id)}
					>
						{room.name}
					</button>
				{/each}
			{/if}
		</div>
	</aside>

	<main class="content">
		{#if selectedRoomId}
			<div data-testid="chat-area" class="chat-area">
				<div class="call-controls">
					<button data-testid="call-button" onclick={() => inCall ? leaveCall() : joinCall()}>
						{inCall ? 'Leave Call' : 'Join Call'}
					</button>
					{#if inCall}
						<button data-testid="mute-button" onclick={toggleMute}>
							{audioMuted ? 'Unmute' : 'Mute'}
						</button>
						<button data-testid="video-toggle-button" onclick={toggleVideo}>
							{videoMuted ? 'Video On' : 'Video Off'}
						</button>
					{/if}
				</div>

				{#if inCall && localStream}
					<!-- svelte-ignore a11y_media_has_caption -->
					<video
						data-testid="local-video"
						autoplay
						muted
						playsinline
						use:setStream={localStream}
					></video>
				{/if}

				{#if inCall && remoteStreams.size > 0}
					<div data-testid="remote-streams" class="remote-streams">
						{#each [...remoteStreams.entries()] as [userId, { username, stream }]}
							<!-- svelte-ignore a11y_media_has_caption -->
							<video
								data-testid="remote-stream"
								autoplay
								playsinline
								use:setStream={stream}
							></video>
						{/each}
					</div>
				{/if}

				<div class="message-list">
					{#if messages.length === 0}
						<p data-testid="no-messages-placeholder">No messages yet</p>
					{:else}
						{#each messages as msg}
							<div data-testid="message-item" class="message-item">
								<strong>{msg.username}</strong>: {msg.content}
							</div>
						{/each}
					{/if}
				</div>

				<form onsubmit={handleSendMessage} class="message-form">
					<input
						data-testid="message-input"
						type="text"
						placeholder="Type a message..."
						bind:value={messageInput}
						onkeydown={handleKeyDown}
					/>
				</form>
			</div>
		{:else}
			<p class="select-room-prompt">Select a room to start chatting</p>
		{/if}
	</main>
</div>
