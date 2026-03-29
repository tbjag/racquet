<script lang="ts">
	import { goto } from '$app/navigation';
	import { register } from '$lib/api';

	let username = $state('');
	let password = $state('');
	let error = $state('');

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		try {
			await register(username, password);
			goto('/login');
		} catch (err: any) {
			error = err.message;
		}
	}
</script>

<div class="auth-page">
	<h1>Register</h1>
	<form onsubmit={handleSubmit}>
		<input data-testid="username-input" type="text" placeholder="Username" bind:value={username} />
		<input data-testid="password-input" type="password" placeholder="Password" bind:value={password} />
		{#if error}
			<p data-testid="error-message" class="error">{error}</p>
		{/if}
		<button data-testid="submit-button" type="submit">Register</button>
	</form>
	<p>Already have an account? <a href="/login">Login</a></p>
</div>
