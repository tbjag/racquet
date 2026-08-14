<script lang="ts">
	type Props = {
		onSubmit: (name: string) => Promise<boolean>;
	};

	let { onSubmit }: Props = $props();

	let name = $state('');
	let submitting = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		const trimmed = name.trim();
		if (!trimmed || submitting) return;
		submitting = true;
		try {
			const ok = await onSubmit(trimmed);
			if (ok) name = '';
		} finally {
			submitting = false;
		}
	}
</script>

<form onsubmit={handleSubmit} class="create-room-form">
	<input
		data-testid="room-name-input"
		type="text"
		placeholder="Room name"
		bind:value={name}
		disabled={submitting}
	/>
	<button
		data-testid="room-submit-button"
		type="submit"
		class="btn primary"
		disabled={submitting || !name.trim()}
	>
		{submitting ? 'Creating…' : 'Create'}
	</button>
</form>

<style>
	.create-room-form {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--bg-sunken);
	}

	.create-room-form input {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--bg);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
	}

	.create-room-form input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	/* Button styling comes from .btn.primary in app.css. */
</style>
