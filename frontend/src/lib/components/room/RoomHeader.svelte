<script lang="ts">
	type Props = {
		roomName: string;
		onRename: (name: string) => Promise<boolean>;
		onDelete: () => Promise<void>;
	};

	let { roomName, onRename, onDelete }: Props = $props();

	let editing = $state(false);
	let editValue = $state('');
	let confirming = $state(false);
	let busy = $state(false);

	function startEdit() {
		editValue = roomName;
		confirming = false;
		editing = true;
	}

	async function handleSave(e: Event) {
		e.preventDefault();
		const trimmed = editValue.trim();
		if (!trimmed || busy) return;
		busy = true;
		try {
			const ok = await onRename(trimmed);
			if (ok) editing = false;
		} finally {
			busy = false;
		}
	}

	async function handleDelete() {
		if (busy) return;
		busy = true;
		try {
			await onDelete();
		} finally {
			busy = false;
			confirming = false;
		}
	}
</script>

<header data-testid="room-header" class="room-header">
	<div class="center-col inner">
		<span class="hash" aria-hidden="true">#</span>
		{#if editing}
			<form onsubmit={handleSave} class="edit-name-form">
				<input
					data-testid="room-name-edit-input"
					type="text"
					bind:value={editValue}
					maxlength={50}
					disabled={busy}
				/>
				<button
					data-testid="save-room-name-button"
					type="submit"
					class="btn primary"
					disabled={busy || !editValue.trim()}
				>
					Save
				</button>
				<button type="button" class="btn" onclick={() => (editing = false)}>Cancel</button>
			</form>
		{:else}
			<h2>{roomName}</h2>
			<div class="actions">
				{#if confirming}
					<button
						data-testid="confirm-delete-room-button"
						class="btn danger"
						onclick={handleDelete}
						disabled={busy}
					>
						Confirm delete
					</button>
					<button class="btn" onclick={() => (confirming = false)}>Cancel</button>
				{:else}
					<button data-testid="rename-room-button" class="btn ghost" onclick={startEdit}>
						Rename
					</button>
					<button
						data-testid="delete-room-button"
						class="btn ghost"
						onclick={() => (confirming = true)}
					>
						Delete
					</button>
				{/if}
			</div>
		{/if}
	</div>
</header>

<style>
	.room-header {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--bg);
		flex-shrink: 0;
	}

	.inner {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.hash {
		color: var(--text-muted);
		font-size: 1.1rem;
		line-height: 1;
	}

	.room-header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.actions {
		display: flex;
		gap: var(--space-2);
		margin-left: auto;
	}

	.edit-name-form {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: 1;
	}

	.edit-name-form input {
		flex: 1;
		min-width: 0;
		padding: var(--space-2) var(--space-3);
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		font-size: 1rem;
		font-weight: 600;
	}

	.edit-name-form input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
