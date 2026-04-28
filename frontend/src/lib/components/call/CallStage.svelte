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
</style>
