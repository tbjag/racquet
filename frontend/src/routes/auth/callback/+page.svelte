<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { setToken } from '$lib/auth';

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const token = params.get('token');

		if (token) {
			setToken(token);
			window.history.replaceState({}, '', '/auth/callback');
			goto('/');
		} else {
			goto('/login');
		}
	});
</script>

<div class="auth-page">
	<p>Signing you in…</p>
</div>

<style>
	.auth-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: var(--bg-sunken);
	}

	p {
		margin: 0;
		color: var(--text-muted);
		font-size: 1rem;
	}
</style>
