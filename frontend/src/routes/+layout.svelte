<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { getToken } from '$lib/auth';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	const publicPaths = ['/login', '/register'];

	onMount(() => {
		const token = getToken();
		const currentPath = window.location.pathname;
		if (!token && !publicPaths.includes(currentPath)) {
			goto('/login');
		}
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
