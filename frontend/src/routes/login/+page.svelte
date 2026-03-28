<script lang="ts">
	import { goto } from '$app/navigation';
	import { login } from '$lib/api';
	import { setToken } from '$lib/auth';

	let username = $state('');
	let password = $state('');
	let error = $state('');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		try {
			const result = await login(username, password);
			setToken(result.token);
			goto('/');
		} catch (err: any) {
			error = err.message;
		}
	}
</script>

<div class="auth-page">
	<h1>Login</h1>
	<form onsubmit={handleSubmit}>
		<input data-testid="username-input" type="text" placeholder="Username" bind:value={username} />
		<input data-testid="password-input" type="password" placeholder="Password" bind:value={password} />
		{#if error}
			<p data-testid="error-message" class="error">{error}</p>
		{/if}
		<button data-testid="submit-button" type="submit">Login</button>
	</form>
	<p>Don't have an account? <a href="/register">Register</a></p>
</div>
