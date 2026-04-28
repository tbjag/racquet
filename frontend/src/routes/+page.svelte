<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { getToken } from '$lib/auth';
	import { getRooms, createRoom, getMessages, getProfile, updateProfile } from '$lib/api';
	import { setToken, clearToken } from '$lib/auth';
	import { WebSocketClient } from '$lib/ws';
	import { WebRTCManager } from '$lib/webrtc';
	import { apiCall } from '$lib/apiCall';
	import UserProfile from '$lib/components/sidebar/UserProfile.svelte';
	import RoomList from '$lib/components/sidebar/RoomList.svelte';
	import CreateRoomForm from '$lib/components/sidebar/CreateRoomForm.svelte';
	import RoomHeader from '$lib/components/room/RoomHeader.svelte';
	import MessageList from '$lib/components/room/MessageList.svelte';
	import MessageInput from '$lib/components/room/MessageInput.svelte';
	import MemberList from '$lib/components/room/MemberList.svelte';
	import type { Member } from '$lib/components/room/MemberList.svelte';
	import CallControls from '$lib/components/call/CallControls.svelte';
	import CallStage from '$lib/components/call/CallStage.svelte';
	import type { RemotePeer } from '$lib/components/call/CallStage.svelte';

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
	let showCreateForm = $state(false);
	let ws: WebSocketClient | null = null;
	let currentRoomId: string | null = null;
	let displayName = $state('');
	let ownUserId = $state<string>('');

	const selectedRoom = $derived(rooms.find((r) => r.id === selectedRoomId) ?? null);

	const members = $derived<Member[]>(
		selectedRoomId && ownUserId
			? [
					{ id: ownUserId, username: displayName, isSelf: true },
					...Array.from(roomUsers.entries()).map(([id, username]) => ({
						id,
						username,
						isSelf: false
					}))
				]
			: []
	);

	// WebRTC state
	let inCall = $state(false);
	let localStream = $state<MediaStream | null>(null);
	let remoteStreams = $state<RemotePeer[]>([]);
	let webrtcManager: WebRTCManager | null = null;
	let audioMuted = $state(false);
	let videoMuted = $state(false);
	let roomUsers = $state<Map<string, string>>(new Map()); // userId -> username

	// Screen share state
	let localScreenStream = $state<MediaStream | null>(null);
	let activeScreenSharerId = $state<string | null>(null);
	let remoteScreenStream = $state<{ userId: string; username: string; stream: MediaStream } | null>(null);

	onMount(async () => {
		token = getToken();
		if (!token) {
			goto('/login');
			return;
		}

		ws = new WebSocketClient();
		ws.connect(token);
		ws.onMessage(handleWsMessage);

		const profile = await apiCall(() => getProfile(token!), {
			errorMessage: 'Could not load your profile'
		});
		if (profile) {
			displayName = profile.username;
			ownUserId = profile.id;
		}

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
		} else if (msg.type === 'screen_share_started' && msg.room_id === selectedRoomId) {
			activeScreenSharerId = msg.user_id;
			if (msg.user_id !== ownUserId) {
				webrtcManager?.registerRemoteScreenStreamId(msg.stream_id);
			}
		} else if (msg.type === 'screen_share_stopped' && msg.room_id === selectedRoomId) {
			activeScreenSharerId = null;
			if (remoteScreenStream && remoteScreenStream.userId === msg.user_id) {
				webrtcManager?.unregisterRemoteScreenStreamId(remoteScreenStream.stream.id);
				remoteScreenStream = null;
			}
			if (msg.user_id === ownUserId) {
				localScreenStream = null;
			}
		}
	}

	async function loadRooms() {
		if (!token) return;
		const result = await apiCall(() => getRooms(token!), {
			errorMessage: 'Could not load rooms'
		});
		if (result) rooms = result;
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

		const history = await apiCall(() => getMessages(token!, roomId), {
			errorMessage: 'Could not load message history'
		});
		messages = history ? history.reverse() : [];
	}

	async function handleCreateRoom(name: string): Promise<boolean> {
		if (!token) return false;
		const created = await apiCall(() => createRoom(token!, name), {
			errorMessage: 'Could not create room'
		});
		if (!created) return false;
		showCreateForm = false;
		await loadRooms();
		return true;
	}

	function handleSendMessage(text: string) {
		if (!ws || !selectedRoomId) return;
		ws.sendMessage(selectedRoomId, text);
	}

	async function handleSaveName(newName: string): Promise<boolean> {
		if (!token) return false;
		const result = await apiCall(() => updateProfile(token!, newName), {
			errorMessage: 'Could not update name'
		});
		if (!result) return false;
		token = result.token;
		setToken(result.token);
		displayName = result.user.username;

		// Reconnect WebSocket with new token so messages use the new name
		ws?.disconnect();
		ws = new WebSocketClient();
		ws.connect(token);
		ws.onMessage(handleWsMessage);
		if (currentRoomId) {
			ws.joinRoom(currentRoomId);
		}
		return true;
	}

	function handleLogout() {
		clearToken();
		ws?.disconnect();
		goto('/login');
	}

	async function joinCall() {
		if (!ws || !selectedRoomId) return;

		webrtcManager = new WebRTCManager(
			ws,
			selectedRoomId,
			(userId, username, stream) => {
				remoteStreams = [
					...remoteStreams.filter((p) => p.userId !== userId),
					{ userId, username, stream }
				];
			},
			(userId) => {
				remoteStreams = remoteStreams.filter((p) => p.userId !== userId);
			},
			(userId, username, stream) => {
				remoteScreenStream = { userId, username, stream };
			},
			() => {
				// Browser's own "Stop sharing" bar ended the share
				stopScreenShareLocal();
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
		// If we're sharing, tell the server before tearing down
		if (localScreenStream && ws && selectedRoomId) {
			ws.sendScreenShareStop(selectedRoomId);
		}
		if (webrtcManager) {
			webrtcManager.leaveCall();
			webrtcManager = null;
		}
		localStream = null;
		remoteStreams = [];
		localScreenStream = null;
		remoteScreenStream = null;
		inCall = false;
		audioMuted = false;
		videoMuted = false;
	}

	async function toggleScreenShare() {
		if (!webrtcManager || !ws || !selectedRoomId) return;

		if (localScreenStream) {
			await stopScreenShareLocal();
			return;
		}

		const stream = await webrtcManager.acquireScreenStream();
		if (!stream) return; // user cancelled or capture failed
		// Notify peers BEFORE renegotiating so receivers can classify the
		// incoming screen tracks (stream id known) when ontrack fires.
		ws.sendScreenShareStart(selectedRoomId, stream.id);
		localScreenStream = stream;
		await webrtcManager.broadcastScreenStream();
	}

	async function stopScreenShareLocal() {
		if (!webrtcManager || !ws || !selectedRoomId) return;
		ws.sendScreenShareStop(selectedRoomId);
		localScreenStream = null;
		await webrtcManager.stopScreenShare();
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

</script>

<div class="app">
	<aside class="sidebar">
		<UserProfile {displayName} onSave={handleSaveName} onLogout={handleLogout} />

		<div class="sidebar-header">
			<h2>Rooms</h2>
			<button data-testid="create-room-button" onclick={() => (showCreateForm = !showCreateForm)}>+</button>
		</div>

		{#if showCreateForm}
			<CreateRoomForm onSubmit={handleCreateRoom} />
		{/if}

		<RoomList {rooms} {selectedRoomId} onSelect={selectRoom} />
	</aside>

	<main class="content">
		{#if selectedRoomId}
			<div data-testid="chat-area" class="chat-area">
				{#if selectedRoom}
					<RoomHeader roomName={selectedRoom.name} />
				{/if}
				<MemberList {members} />
				<CallControls
					{inCall}
					{audioMuted}
					{videoMuted}
					isSharing={!!localScreenStream}
					canShare={activeScreenSharerId === null || activeScreenSharerId === ownUserId}
					onToggleCall={() => (inCall ? leaveCall() : joinCall())}
					onToggleMute={toggleMute}
					onToggleVideo={toggleVideo}
					onToggleScreenShare={toggleScreenShare}
				/>

				{#if inCall}
					<CallStage
						{localStream}
						{remoteStreams}
						{localScreenStream}
						{remoteScreenStream}
					/>
				{/if}

				<MessageList {messages} />
				<MessageInput onSend={handleSendMessage} />
			</div>
		{:else}
			<p class="select-room-prompt">Select a room to start chatting</p>
		{/if}
	</main>
</div>

<style>
	.app {
		display: flex;
		height: 100vh;
	}

	.sidebar {
		width: 240px;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--border);
		overflow-y: auto;
	}

	.content {
		flex: 1;
		display: flex;
		min-width: 0;
	}

	.chat-area {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		min-width: 0;
	}
</style>
