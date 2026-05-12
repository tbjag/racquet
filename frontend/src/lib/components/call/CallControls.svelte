<script lang="ts">
	import type { ScreenShareMode } from '$lib/stores/screenShareMode.svelte';

	type Props = {
		inCall: boolean;
		audioMuted: boolean;
		videoMuted: boolean;
		isSharing: boolean;
		canShare: boolean;
		chatHidden: boolean;
		screenShareMode: ScreenShareMode;
		audioSmoothingEnabled: boolean;
		hasScreenAudio: boolean;
		screenAudioVolume: number;
		onToggleCall: () => void;
		onToggleMute: () => void;
		onToggleVideo: () => void;
		onToggleScreenShare: () => void;
		onToggleChat: () => void;
		onChangeScreenShareMode: (mode: ScreenShareMode) => void;
		onToggleAudioSmoothing: () => void;
		onChangeScreenAudioVolume: (v: number) => void;
	};

	let {
		inCall,
		audioMuted,
		videoMuted,
		isSharing,
		canShare,
		chatHidden,
		screenShareMode,
		audioSmoothingEnabled,
		hasScreenAudio,
		screenAudioVolume,
		onToggleCall,
		onToggleMute,
		onToggleVideo,
		onToggleScreenShare,
		onToggleChat,
		onChangeScreenShareMode,
		onToggleAudioSmoothing,
		onChangeScreenAudioVolume
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
			data-testid="audio-smoothing-toggle"
			class={!audioSmoothingEnabled ? 'btn toggled' : 'btn'}
			onclick={onToggleAudioSmoothing}
			title="Even out volume between speakers"
		>
			{audioSmoothingEnabled ? 'Smoothing On' : 'Smoothing Off'}
		</button>
		<button
			data-testid="screen-share-button"
			class={isSharing ? 'btn toggled' : 'btn'}
			onclick={onToggleScreenShare}
			disabled={!canShare}
		>
			{isSharing ? 'Stop Sharing' : 'Share Screen'}
		</button>
		{#if isSharing && hasScreenAudio}
			<label class="volume-control" title="System audio volume">
				<span aria-hidden="true">🔊</span>
				<input
					data-testid="screen-audio-volume"
					type="range"
					min="0"
					max="1"
					step="0.01"
					value={screenAudioVolume}
					oninput={(e) =>
						onChangeScreenAudioVolume(Number((e.currentTarget as HTMLInputElement).value))}
				/>
			</label>
		{/if}
		<div class="mode-group" role="group" aria-label="Screen share mode">
			<button
				data-testid="screen-share-mode-motion"
				class={screenShareMode === 'motion' ? 'btn seg active' : 'btn seg'}
				aria-pressed={screenShareMode === 'motion'}
				onclick={() => onChangeScreenShareMode('motion')}
				title="Optimize for motion (30 fps, smooth playback)"
			>
				Motion
			</button>
			<button
				data-testid="screen-share-mode-detail"
				class={screenShareMode === 'detail' ? 'btn seg active' : 'btn seg'}
				aria-pressed={screenShareMode === 'detail'}
				onclick={() => onChangeScreenShareMode('detail')}
				title="Optimize for detail (low fps, sharp text)"
			>
				Detail
			</button>
		</div>
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

	.mode-group {
		display: inline-flex;
	}

	.mode-group .btn.seg {
		border-radius: 0;
	}

	.mode-group .btn.seg:first-child {
		border-top-left-radius: var(--radius-sm);
		border-bottom-left-radius: var(--radius-sm);
	}

	.mode-group .btn.seg:last-child {
		border-top-right-radius: var(--radius-sm);
		border-bottom-right-radius: var(--radius-sm);
		margin-left: -1px;
	}

	.mode-group .btn.seg.active {
		background: var(--accent);
		color: var(--accent-text);
		border-color: var(--accent);
		z-index: 1;
	}

	.volume-control {
		display: inline-flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0 var(--space-2);
		background: var(--bg-elevated);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		font-size: 0.85rem;
	}

	.volume-control input[type='range'] {
		width: 100px;
		accent-color: var(--accent);
	}
</style>
