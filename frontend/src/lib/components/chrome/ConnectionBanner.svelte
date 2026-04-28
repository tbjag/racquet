<script lang="ts">
	import { connection } from '$lib/stores/connection.svelte';

	const visible = $derived(connection.status !== 'open' && connection.status !== 'closed');

	const text = $derived(
		connection.status === 'connecting'
			? 'Connecting…'
			: connection.status === 'reconnecting'
				? 'Reconnecting…'
				: connection.status === 'auth_failed'
					? 'Sign-in expired — please log in again'
					: ''
	);

	const tone = $derived(connection.status === 'auth_failed' ? 'error' : 'info');
</script>

{#if visible}
	<div data-testid="connection-banner" class="banner" data-tone={tone} role="status">
		{text}
	</div>
{/if}

<style>
	.banner {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 150;
		padding: var(--space-2) var(--space-4);
		font-size: 0.85rem;
		font-weight: 500;
		text-align: center;
		background: var(--accent);
		color: var(--accent-text);
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
		animation: banner-in 200ms ease-out;
	}

	.banner[data-tone='error'] {
		background: var(--danger);
	}

	@keyframes banner-in {
		from {
			transform: translateY(-100%);
		}
		to {
			transform: translateY(0);
		}
	}
</style>
