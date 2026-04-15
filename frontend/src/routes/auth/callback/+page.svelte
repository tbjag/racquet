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
	<p>Signing you in...</p>
</div>
