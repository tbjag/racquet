<script lang="ts">
	const API_BASE = 'http://localhost:3000';

	let error = $state('');

	if (typeof window !== 'undefined') {
		const params = new URLSearchParams(window.location.search);
		const urlError = params.get('error');
		if (urlError === 'not_allowed') {
			error = 'Your Google account is not authorized to use this app.';
		} else if (urlError === 'oauth_error') {
			error = 'Google sign-in failed. Please try again.';
		}
	}
</script>

<div class="auth-page">
	<h1>Racquet</h1>
	{#if error}
		<p data-testid="error-message" class="error">{error}</p>
	{/if}
	<a href="{API_BASE}/api/auth/google" data-testid="google-login-button" class="google-btn">
		Sign in with Google
	</a>
</div>
