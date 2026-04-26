<script lang="ts">
	import { toasts, dismiss } from '$lib/stores/toast.svelte';
</script>

<div class="toast-host" aria-live="polite" aria-atomic="true">
	{#each toasts as toast (toast.id)}
		<button
			type="button"
			class="toast toast-{toast.kind}"
			data-testid={`toast-${toast.kind}`}
			onclick={() => dismiss(toast.id)}
		>
			{toast.message}
		</button>
	{/each}
</div>

<style>
	.toast-host {
		position: fixed;
		bottom: var(--space-4);
		right: var(--space-4);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		z-index: 200;
		max-width: 380px;
	}

	.toast {
		text-align: left;
		padding: var(--space-3) var(--space-4);
		border-radius: var(--radius-md);
		border: 1px solid var(--border);
		background: var(--bg-elevated);
		color: var(--text);
		font: inherit;
		cursor: pointer;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
	}

	.toast-error {
		background: var(--danger-bg);
		border-color: var(--danger);
		color: var(--danger);
	}

	.toast-success {
		border-color: var(--success);
		color: var(--success);
	}
</style>
