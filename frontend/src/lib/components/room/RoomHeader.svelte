<script lang="ts">
	type Props = {
		roomName: string;
	};

	let { roomName }: Props = $props();
</script>

<header data-testid="room-header" class="room-header">
	<span class="hash" aria-hidden="true">#</span>
	<h2>{roomName}</h2>
</header>

<style>
	.room-header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--bg);
		flex-shrink: 0;
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
	}
</style>
