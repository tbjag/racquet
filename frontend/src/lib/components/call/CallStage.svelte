<script lang="ts">
	import VideoTile from './VideoTile.svelte';

	export type RemotePeer = { userId: string; username: string; stream: MediaStream };

	type Props = {
		localStream: MediaStream | null;
		remoteStreams: RemotePeer[];
		localScreenStream: MediaStream | null;
		remoteScreenStream: { userId: string; username: string; stream: MediaStream } | null;
	};

	let { localStream, remoteStreams, localScreenStream, remoteScreenStream }: Props = $props();

	const screenStream = $derived(localScreenStream ?? remoteScreenStream?.stream ?? null);
	const screenActive = $derived(screenStream !== null);

	let theaterMode = $state(false);
	let camerasVisible = $state(true);

	$effect(() => {
		if (!screenActive) theaterMode = false;
	});

	$effect(() => {
		if (!theaterMode) return;
		const onKey = (e: KeyboardEvent) => {
			if (e.key === 'Escape') theaterMode = false;
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});
</script>

<div class="call-stage" class:has-screen={screenActive}>
	{#if screenActive && screenStream}
		<div class="screen-wrap">
			<VideoTile
				stream={screenStream}
				testid="screen-share-tile"
				muted={!!localScreenStream}
				class="screen-share-tile"
			/>
			<button
				type="button"
				class="theater-toggle"
				data-testid="theater-toggle"
				onclick={() => (theaterMode = true)}
				aria-label="Enter theater mode"
			>
				Theater
			</button>
		</div>
	{/if}

	<div class="cameras" class:strip={screenActive}>
		{#if localStream}
			<div class="tile">
				<VideoTile
					stream={localStream}
					testid="local-video"
					muted
					class={screenActive ? 'camera-strip' : ''}
				/>
				<span class="label">You</span>
			</div>
		{/if}

		{#if remoteStreams.length > 0}
			<div
				data-testid="remote-streams"
				class={'remote-streams ' + (screenActive ? 'camera-strip' : '')}
			>
				{#each remoteStreams as peer (peer.userId)}
					<div class="tile">
						<VideoTile stream={peer.stream} testid="remote-stream" />
						<span class="label">{peer.username}</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
</div>

{#if theaterMode && screenActive && screenStream}
	<div class="theater-overlay" role="dialog" aria-label="Theater mode">
		<VideoTile
			stream={screenStream}
			testid="theater-screen-tile"
			muted={!!localScreenStream}
			class="theater-video"
		/>
		<div class="theater-buttons">
			<button
				type="button"
				class="btn"
				data-testid="theater-cameras-toggle"
				onclick={() => (camerasVisible = !camerasVisible)}
			>
				{camerasVisible ? 'Hide Cameras' : 'Show Cameras'}
			</button>
			<button
				type="button"
				class="btn"
				data-testid="theater-exit"
				onclick={() => (theaterMode = false)}
			>
				Exit Theater
			</button>
		</div>
		{#if camerasVisible && remoteStreams.length > 0}
			<div class="theater-cameras" data-testid="theater-cameras">
				{#each remoteStreams as peer (peer.userId)}
					<div class="tile">
						<VideoTile stream={peer.stream} testid="remote-stream" />
						<span class="label">{peer.username}</span>
					</div>
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.call-stage {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: var(--bg-sunken);
		border-bottom: 1px solid var(--border);
	}

	.screen-wrap {
		position: relative;
		display: flex;
		justify-content: center;
		background: #000;
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.screen-wrap :global(.screen-share-tile) {
		width: 100%;
		max-height: 50vh;
		object-fit: contain;
		background: #000;
		display: block;
	}

	.theater-toggle {
		position: absolute;
		top: var(--space-2);
		right: var(--space-2);
		padding: var(--space-1) var(--space-2);
		background: rgba(0, 0, 0, 0.55);
		color: #fff;
		border: 1px solid rgba(255, 255, 255, 0.2);
		border-radius: var(--radius-sm);
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		z-index: 1;
	}

	.theater-toggle:hover {
		background: rgba(0, 0, 0, 0.75);
		border-color: rgba(255, 255, 255, 0.4);
	}

	.cameras {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		align-items: flex-start;
	}

	.cameras.strip {
		flex-wrap: nowrap;
		overflow-x: auto;
	}

	.tile {
		position: relative;
		width: 240px;
		height: 135px;
		flex: 0 0 auto;
		background: #000;
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.cameras.strip .tile {
		width: 160px;
		height: 90px;
	}

	.tile :global(video) {
		width: 100%;
		height: 100%;
		display: block;
		object-fit: cover;
		background: #000;
	}

	.label {
		position: absolute;
		left: var(--space-2);
		bottom: var(--space-2);
		padding: 2px var(--space-2);
		background: rgba(0, 0, 0, 0.6);
		color: #fff;
		font-size: 0.75rem;
		border-radius: var(--radius-sm);
		pointer-events: none;
		max-width: calc(100% - var(--space-4));
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.remote-streams {
		display: contents;
	}

	.theater-overlay {
		position: fixed;
		inset: 0;
		z-index: 250;
		background: #000;
	}

	.theater-overlay :global(.theater-video) {
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: #000;
		display: block;
	}

	.theater-buttons {
		position: absolute;
		top: var(--space-3);
		right: var(--space-3);
		display: flex;
		gap: var(--space-2);
		z-index: 2;
	}

	.theater-buttons .btn {
		padding: var(--space-2) var(--space-3);
		background: var(--bg-elevated);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		font-size: 0.85rem;
		font-weight: 500;
		cursor: pointer;
	}

	.theater-buttons .btn:hover {
		background: var(--bg-hover);
		border-color: var(--border-strong);
	}

	.theater-cameras {
		position: absolute;
		bottom: var(--space-3);
		right: var(--space-3);
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		max-height: 70vh;
		overflow-y: auto;
		z-index: 2;
	}

	.theater-cameras .tile {
		width: 160px;
		height: 90px;
	}
</style>
