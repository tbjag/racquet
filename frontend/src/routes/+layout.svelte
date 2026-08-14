<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { getToken } from '$lib/auth';
	import favicon from '$lib/assets/favicon.svg';
	import { theme } from '$lib/stores/theme.svelte';
	import ToastHost from '$lib/components/chrome/ToastHost.svelte';
	import '@fontsource-variable/inter';
	import '../app.css';

	let { children } = $props();

	const publicPaths = ['/login', '/auth/callback'];

	onMount(() => {
		const token = getToken();
		const currentPath = window.location.pathname;
		if (!token && !publicPaths.includes(currentPath)) {
			goto('/login');
		}
	});

	$effect(() => {
		if (typeof document !== 'undefined') {
			document.documentElement.dataset.theme = theme.effective;
		}
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}

<ToastHost />
