<script lang="ts">
	type Props = {
		roomName: string;
	};

	let { roomName }: Props = $props();
</script>

<header data-testid="room-header" class="room-header">
	<h2>{roomName}</h2>
</header>

<style>
	.room-header {
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
	}

	.room-header h2 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
	}
</style>
