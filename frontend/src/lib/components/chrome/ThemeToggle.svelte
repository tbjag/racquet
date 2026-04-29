<script lang="ts">
	import { theme, cycleMode } from '$lib/stores/theme.svelte';

	const labels = { light: 'Light', dark: 'Dark', system: 'System', retro: 'Retro' } as const;
</script>

<button
	data-testid="theme-toggle"
	data-theme-mode={theme.mode}
	class="theme-toggle"
	onclick={cycleMode}
	title="Toggle theme (light → dark → system → retro)"
	aria-label="Toggle theme"
>
	Theme: {labels[theme.mode]}
</button>

<style>
	.theme-toggle {
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		padding: var(--space-2) var(--space-3);
		font-size: 0.85rem;
	}

	.theme-toggle:hover {
		background: var(--bg-hover);
		border-color: var(--border-strong);
	}
</style>
