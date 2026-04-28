<script lang="ts">
	type Props = {
		inCall: boolean;
		audioMuted: boolean;
		videoMuted: boolean;
		isSharing: boolean;
		canShare: boolean;
		onToggleCall: () => void;
		onToggleMute: () => void;
		onToggleVideo: () => void;
		onToggleScreenShare: () => void;
	};

	let {
		inCall,
		audioMuted,
		videoMuted,
		isSharing,
		canShare,
		onToggleCall,
		onToggleMute,
		onToggleVideo,
		onToggleScreenShare
	}: Props = $props();
</script>

<div class="call-controls">
	<button data-testid="call-button" onclick={onToggleCall}>
		{inCall ? 'Leave Call' : 'Join Call'}
	</button>
	{#if inCall}
		<button data-testid="mute-button" onclick={onToggleMute}>
			{audioMuted ? 'Unmute' : 'Mute'}
		</button>
		<button data-testid="video-toggle-button" onclick={onToggleVideo}>
			{videoMuted ? 'Video On' : 'Video Off'}
		</button>
		<button
			data-testid="screen-share-button"
			onclick={onToggleScreenShare}
			disabled={!canShare}
		>
			{isSharing ? 'Stop Sharing' : 'Share Screen'}
		</button>
	{/if}
</div>

<style>
	.call-controls {
		display: flex;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-4);
		border-bottom: 1px solid var(--border);
	}
</style>
