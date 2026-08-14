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
	let expanded = $state<Record<string, boolean>>({});
	let overflowing = $state<Record<string, boolean>>({});

	function clampWatch(node: HTMLElement, id: string) {
		const check = () => {
			if (!expanded[id]) overflowing[id] = node.scrollHeight > node.clientHeight + 1;
		};
		const ro = new ResizeObserver(check);
		ro.observe(node);
		return { destroy: () => ro.disconnect() };
	}

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
	<div class="center-col messages">
		{#if messages.length === 0}
			<p data-testid="no-messages-placeholder">No messages yet</p>
		{:else}
			{#each messages as msg (msg.id)}
				<div data-testid="message-item" class="message-item">
					<div class="body" class:clamped={!expanded[msg.id]} use:clampWatch={msg.id}>
						<strong>{msg.username}:</strong>
						{msg.content}
					</div>
					{#if overflowing[msg.id]}
						<button
							data-testid="read-more-button"
							class="read-more"
							onclick={() => (expanded[msg.id] = !expanded[msg.id])}
						>
							{expanded[msg.id] ? 'Show less' : 'Read more'}
						</button>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>

<style>
	.message-list {
		flex: 1;
		min-height: 0;
		overflow-y: auto;
		padding: var(--space-4);
		background: var(--bg);
		color: var(--text);
	}

	.messages {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}

	.message-item {
		padding: var(--space-1) var(--space-2);
		margin: 0 calc(var(--space-2) * -1);
		border-radius: var(--radius-sm);
		line-height: 1.5;
		word-break: break-word;
	}

	.message-item:hover {
		background: var(--bg-hover);
	}

	.message-item strong {
		color: var(--accent);
		font-weight: 600;
	}

	.body.clamped {
		display: -webkit-box;
		-webkit-line-clamp: 8;
		line-clamp: 8;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}

	.read-more {
		margin-top: var(--space-1);
		padding: 0;
		background: none;
		border: none;
		box-shadow: none;
		color: var(--accent);
		font-size: 0.8rem;
		font-weight: 500;
		cursor: pointer;
	}

	.read-more:hover {
		text-decoration: underline;
	}

	[data-testid='no-messages-placeholder'] {
		margin: 0;
		padding: var(--space-3) 0;
		color: var(--text-muted);
		font-size: 0.9rem;
		font-style: italic;
	}
</style>
