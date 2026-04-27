<script lang="ts">
	type Room = { id: string; name: string };

	type Props = {
		rooms: Room[];
		selectedRoomId: string | null;
		onSelect: (roomId: string) => void;
	};

	let { rooms, selectedRoomId, onSelect }: Props = $props();
</script>

<div data-testid="room-list">
	{#if rooms.length === 0}
		<p data-testid="no-rooms-placeholder">No rooms yet</p>
	{:else}
		{#each rooms as room (room.id)}
			<button
				data-testid="room-item"
				class="room-item {selectedRoomId === room.id ? 'active' : ''}"
				onclick={() => onSelect(room.id)}
			>
				{room.name}
			</button>
		{/each}
	{/if}
</div>
