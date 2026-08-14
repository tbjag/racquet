<script lang="ts">
	type Room = { id: string; name: string };

	type Props = {
		rooms: Room[];
		selectedRoomId: string | null;
		onSelect: (roomId: string) => void;
	};

	let { rooms, selectedRoomId, onSelect }: Props = $props();
</script>

<div data-testid="room-list" class="room-list">
	{#if rooms.length === 0}
		<p data-testid="no-rooms-placeholder" class="empty">No rooms yet</p>
	{:else}
		{#each rooms as room (room.id)}
			<button
				data-testid="room-item"
				class="room-item {selectedRoomId === room.id ? 'active' : ''}"
				onclick={() => onSelect(room.id)}
			>
				<span class="hash" aria-hidden="true">#</span>
				<span class="name">{room.name}</span>
			</button>
		{/each}
	{/if}
</div>

<style>
	.room-list {
		display: flex;
		flex-direction: column;
		padding: var(--space-2);
		gap: 2px;
	}

	.empty {
		margin: 0;
		padding: var(--space-3) var(--space-2);
		color: var(--text-muted);
		font-size: 0.85rem;
	}

	.room-item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: transparent;
		color: var(--text-muted);
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		font-size: 0.9rem;
		text-align: left;
	}

	.room-item:hover {
		background: var(--bg-hover);
		color: var(--text);
	}

	.room-item.active {
		background: var(--bg-elevated);
		border-color: var(--border);
		color: var(--text);
		font-weight: 600;
	}

	.hash {
		color: var(--text-muted);
		font-weight: 400;
	}

	.room-item.active .hash {
		color: var(--accent);
	}

	.name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
