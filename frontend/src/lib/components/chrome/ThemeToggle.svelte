<script lang="ts">
	import { theme, cycleMode } from '$lib/stores/theme.svelte';

	const labels = { light: 'Light', dark: 'Dark', system: 'System', retro: 'Retro' } as const;
</script>

<button
	data-testid="theme-toggle"
	data-theme-mode={theme.mode}
	class="btn"
	onclick={cycleMode}
	title="Toggle theme (light → dark → system → retro)"
	aria-label="Toggle theme"
>
	Theme: {labels[theme.mode]}
</button>
