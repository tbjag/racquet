<script lang="ts">
	const API_BASE = '';

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
	<div class="card">
		<h1>Racquet</h1>
		<p class="tagline">Chat, calls, and screen share for your friends.</p>
		{#if error}
			<p data-testid="error-message" class="error">{error}</p>
		{/if}
		<a href="{API_BASE}/api/auth/google" data-testid="google-login-button" class="google-btn">
			Sign in with Google
		</a>
	</div>
</div>

<style>
	.auth-page {
		min-height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--space-4);
		background: var(--bg-sunken);
	}

	.card {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: var(--space-4);
		width: 100%;
		max-width: 360px;
		padding: var(--space-6);
		background: var(--bg-elevated);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-card);
	}

	h1 {
		margin: 0;
		text-align: center;
		font-size: 2rem;
		font-weight: 700;
		color: var(--text);
	}

	.tagline {
		margin: 0;
		text-align: center;
		color: var(--text-muted);
		font-size: 0.9rem;
	}

	.error {
		margin: 0;
		padding: var(--space-3);
		background: var(--danger-bg);
		border: 1px solid var(--danger);
		border-radius: var(--radius-sm);
		color: var(--danger);
		font-size: 0.85rem;
	}

	.google-btn {
		display: inline-block;
		text-align: center;
		padding: var(--space-3) var(--space-4);
		background: var(--accent);
		color: var(--accent-text);
		border: 1px solid var(--accent);
		border-radius: var(--radius-sm);
		box-shadow: var(--shadow-btn);
		text-decoration: none;
		font-weight: 600;
	}

	.google-btn:hover {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
	}

	/* The offset shadow above outranks the global :focus-visible ring, so restore it here. */
	.google-btn:focus-visible {
		outline: none;
		box-shadow: var(--focus-ring);
	}
</style>
