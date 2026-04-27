<script lang="ts">
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
			<button data-testid="save-name-button" type="submit">Save</button>
			<button type="button" onclick={() => (editing = false)}>Cancel</button>
		</form>
	{:else}
		<span data-testid="display-name">{displayName}</span>
		<button data-testid="edit-name-button" onclick={startEdit}>Edit</button>
	{/if}
	<button data-testid="logout-button" onclick={onLogout}>Logout</button>
</div>
