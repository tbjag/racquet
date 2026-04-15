<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { getToken } from '$lib/auth';
	import { getRooms, createRoom, getMessages, getProfile, updateProfile } from '$lib/api';
	import { setToken, clearToken } from '$lib/auth';
	import { WebSocketClient } from '$lib/ws';

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
	let displayName = $state('');
	let editingName = $state(false);
	let editNameValue = $state('');

	onMount(async () => {
		token = getToken();
		if (!token) {
			goto('/login');
			return;
		}

		ws = new WebSocketClient();
		ws.connect(token);
		ws.onMessage(handleWsMessage);

		const profile = await getProfile(token);
		displayName = profile.username;

		await loadRooms();
	});

	onDestroy(() => {
		ws?.disconnect();
	});

	function handleWsMessage(msg: any) {
		if (msg.type === 'new_message' && msg.room_id === selectedRoomId) {
			messages = [...messages, msg];
		}
	}

	async function loadRooms() {
		if (!token) return;
		rooms = await getRooms(token);
	}

	async function selectRoom(roomId: string) {
		if (!token || !ws) return;

		if (currentRoomId) {
			ws.leaveRoom(currentRoomId);
		}

		selectedRoomId = roomId;
		currentRoomId = roomId;
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

	async function handleSaveName(e: Event) {
		e.preventDefault();
		if (!token || !editNameValue.trim()) return;

		const result = await updateProfile(token, editNameValue.trim());
		token = result.token;
		setToken(result.token);
		displayName = result.user.username;
		editingName = false;

		// Reconnect WebSocket with new token so messages use the new name
		ws?.disconnect();
		ws = new WebSocketClient();
		ws.connect(token);
		ws.onMessage(handleWsMessage);
		if (currentRoomId) {
			ws.joinRoom(currentRoomId);
		}
	}

	function handleLogout() {
		clearToken();
		ws?.disconnect();
		goto('/login');
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handleSendMessage(e);
		}
	}
</script>

<div class="app">
	<aside class="sidebar">
		<div data-testid="user-profile" class="user-profile">
			{#if editingName}
				<form onsubmit={handleSaveName} class="edit-name-form">
					<input
						data-testid="display-name-input"
						type="text"
						bind:value={editNameValue}
						maxlength={30}
					/>
					<button data-testid="save-name-button" type="submit">Save</button>
					<button type="button" onclick={() => (editingName = false)}>Cancel</button>
				</form>
			{:else}
				<span data-testid="display-name">{displayName}</span>
				<button data-testid="edit-name-button" onclick={() => { editNameValue = displayName; editingName = true; }}>Edit</button>
			{/if}
			<button data-testid="logout-button" onclick={handleLogout}>Logout</button>
		</div>

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
