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
		disabled={submitting || !name.trim()}
	>
		{submitting ? 'Creating…' : 'Create'}
	</button>
</form>
