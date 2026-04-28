<script lang="ts">
	type Props = {
		onSend: (text: string) => void;
	};

	let { onSend }: Props = $props();

	let value = $state('');

	function submit() {
		const trimmed = value.trim();
		if (!trimmed) return;
		onSend(trimmed);
		value = '';
	}

	function handleSubmit(e: Event) {
		e.preventDefault();
		submit();
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			submit();
		}
	}
</script>

<form onsubmit={handleSubmit} class="message-form">
	<input
		data-testid="message-input"
		type="text"
		placeholder="Type a message..."
		bind:value
		onkeydown={handleKeyDown}
	/>
</form>

<style>
	.message-form {
		padding: var(--space-3) var(--space-4);
		border-top: 1px solid var(--border);
	}

	.message-form input {
		width: 100%;
		padding: var(--space-2) var(--space-3);
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
	}
</style>
