<script lang="ts">
	type Props = {
		inCall: boolean;
		audioMuted: boolean;
		videoMuted: boolean;
		isSharing: boolean;
		canShare: boolean;
		chatHidden: boolean;
		onToggleCall: () => void;
		onToggleMute: () => void;
		onToggleVideo: () => void;
		onToggleScreenShare: () => void;
		onToggleChat: () => void;
	};

	let {
		inCall,
		audioMuted,
		videoMuted,
		isSharing,
		canShare,
		chatHidden,
		onToggleCall,
		onToggleMute,
		onToggleVideo,
		onToggleScreenShare,
		onToggleChat
	}: Props = $props();
</script>

<div class="call-controls">
	<button
		data-testid="call-button"
		class={inCall ? 'btn danger' : 'btn primary'}
		onclick={onToggleCall}
	>
		{inCall ? 'Leave Call' : 'Join Call'}
	</button>
	{#if inCall}
		<button
			data-testid="mute-button"
			class={audioMuted ? 'btn toggled' : 'btn'}
			onclick={onToggleMute}
		>
			{audioMuted ? 'Unmute' : 'Mute'}
		</button>
		<button
			data-testid="video-toggle-button"
			class={videoMuted ? 'btn toggled' : 'btn'}
			onclick={onToggleVideo}
		>
			{videoMuted ? 'Video On' : 'Video Off'}
		</button>
		<button
			data-testid="screen-share-button"
			class={isSharing ? 'btn toggled' : 'btn'}
			onclick={onToggleScreenShare}
			disabled={!canShare}
		>
			{isSharing ? 'Stop Sharing' : 'Share Screen'}
		</button>
		<button
			data-testid="chat-toggle"
			class={chatHidden ? 'btn toggled' : 'btn'}
			onclick={onToggleChat}
		>
			{chatHidden ? 'Show Chat' : 'Hide Chat'}
		</button>
	{/if}
</div>

<style>
	.call-controls {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--bg);
	}

	.btn {
		padding: var(--space-2) var(--space-3);
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		font-size: 0.85rem;
		font-weight: 500;
	}

	.btn:hover:not(:disabled) {
		background: var(--bg-hover);
		border-color: var(--border-strong);
	}

	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn.primary {
		background: var(--accent);
		color: var(--accent-text);
		border-color: var(--accent);
	}

	.btn.primary:hover:not(:disabled) {
		background: var(--accent-hover);
		border-color: var(--accent-hover);
	}

	.btn.danger {
		background: var(--danger);
		color: var(--text-inverse);
		border-color: var(--danger);
	}

	.btn.toggled {
		background: var(--danger-bg);
		color: var(--danger);
		border-color: var(--danger);
	}
</style>
