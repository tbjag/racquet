<script lang="ts">
	import ThemeToggle from '$lib/components/chrome/ThemeToggle.svelte';

	type Props = {
		displayName: string;
		onSave: (newName: string) => Promise<boolean>;
		onLogout: () => void;
	};

	let { displayName, onSave, onLogout }: Props = $props();

	let editing = $state(false);
	let editValue = $state('');

	function startEdit() {
		editValue = displayName;
		editing = true;
	}

	async function handleSave(e: Event) {
		e.preventDefault();
		const trimmed = editValue.trim();
		if (!trimmed) return;
		const ok = await onSave(trimmed);
		if (ok) editing = false;
	}
</script>

<div data-testid="user-profile" class="user-profile">
	{#if editing}
		<form onsubmit={handleSave} class="edit-name-form">
			<input
				data-testid="display-name-input"
				type="text"
				bind:value={editValue}
				maxlength={30}
			/>
			<div class="edit-actions">
				<button data-testid="save-name-button" type="submit" class="btn primary">Save</button>
				<button type="button" class="btn" onclick={() => (editing = false)}>Cancel</button>
			</div>
		</form>
	{:else}
		<div class="profile-row">
			<span data-testid="display-name" class="display-name">{displayName}</span>
			<button data-testid="edit-name-button" class="btn ghost" onclick={startEdit}>Edit</button>
		</div>
	{/if}
	<div class="footer-row">
		<ThemeToggle />
		<button data-testid="logout-button" class="btn ghost logout" onclick={onLogout}>Logout</button>
	</div>
</div>

<style>
	.user-profile {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
	}

	.profile-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.display-name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 600;
		color: var(--text);
	}

	.edit-name-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}

	.edit-name-form input {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--bg);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
	}

	.edit-actions {
		display: flex;
		gap: var(--space-2);
	}

	.footer-row {
		display: flex;
		gap: var(--space-2);
		align-items: center;
	}

	/* Button styling comes from .btn in app.css. */
	.logout {
		font-size: 0.8rem;
	}
</style>
