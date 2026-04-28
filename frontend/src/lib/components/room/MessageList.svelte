<script lang="ts">
	type Message = {
		id: string;
		username: string;
		content: string;
	};

	type Props = {
		messages: Message[];
	};

	let { messages }: Props = $props();

	const STICK_THRESHOLD_PX = 80;

	let listEl: HTMLDivElement | undefined = $state();
	let stickToBottom = $state(true);

	function distanceFromBottom(el: HTMLElement) {
		return el.scrollHeight - el.scrollTop - el.clientHeight;
	}

	function handleScroll() {
		if (!listEl) return;
		stickToBottom = distanceFromBottom(listEl) < STICK_THRESHOLD_PX;
	}

	// When messages change, scroll to bottom only if the user was already near it.
	$effect(() => {
		// Take a dependency on messages length so this effect re-runs on append.
		messages.length;
		if (!listEl) return;
		if (stickToBottom) {
			// Wait for the new node to be in the DOM before measuring.
			queueMicrotask(() => {
				if (listEl) listEl.scrollTop = listEl.scrollHeight;
			});
		}
	});
</script>

<div
	data-testid="message-list"
	class="message-list"
	bind:this={listEl}
	onscroll={handleScroll}
>
	{#if messages.length === 0}
		<p data-testid="no-messages-placeholder">No messages yet</p>
	{:else}
		{#each messages as msg (msg.id)}
			<div data-testid="message-item" class="message-item">
				<strong>{msg.username}</strong>: {msg.content}
			</div>
		{/each}
	{/if}
</div>

<style>
	.message-list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--space-3) var(--space-4);
	}

	.message-item {
		padding: var(--space-1) 0;
	}
</style>
