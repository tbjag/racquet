<script lang="ts">
	type Props = {
		onSend: (text: string) => void;
	};

	let { onSend }: Props = $props();

	const MAX = 4000;
	const COUNTER_AT = 3600;

	let value = $state('');

	function submit() {
		const trimmed = value.trim();
		if (!trimmed || trimmed.length > MAX) return;
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
	<div class="center-col row">
		<input
			data-testid="message-input"
			type="text"
			placeholder="Type a message..."
			maxlength={MAX}
			bind:value
			onkeydown={handleKeyDown}
		/>
		{#if value.length >= COUNTER_AT}
			<span
				data-testid="message-length-counter"
				class="counter"
				class:over={value.length >= MAX}
			>
				{value.length}/{MAX}
			</span>
		{/if}
	</div>
</form>

<style>
	.message-form {
		padding: var(--space-3) var(--space-4);
		border-top: 1px solid var(--border);
		background: var(--bg);
	}

	.row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}

	.counter {
		flex-shrink: 0;
		font-size: 0.75rem;
		font-variant-numeric: tabular-nums;
		color: var(--text-muted);
	}

	.counter.over {
		color: var(--danger);
		font-weight: 600;
	}

	.message-form input {
		flex: 1;
		min-width: 0;
		padding: var(--space-3);
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
		font-size: 0.95rem;
	}

	/* The offset shadow above outranks the global :focus-visible ring, so restore it here. */
	.message-form input:focus-visible {
		outline: none;
		box-shadow: var(--focus-ring);
	}

	.message-form input::placeholder {
		color: var(--text-muted);
	}
</style>
