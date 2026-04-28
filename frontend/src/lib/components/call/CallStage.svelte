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

{#if screenActive && screenStream}
	<VideoTile
		stream={screenStream}
		testid="screen-share-tile"
		muted={!!localScreenStream}
		class="screen-share-tile"
	/>
{/if}

{#if localStream}
	<VideoTile
		stream={localStream}
		testid="local-video"
		muted
		class={screenActive ? 'camera-strip' : ''}
	/>
{/if}

{#if remoteStreams.length > 0}
	<div
		data-testid="remote-streams"
		class={'remote-streams ' + (screenActive ? 'camera-strip' : '')}
	>
		{#each remoteStreams as peer (peer.userId)}
			<VideoTile stream={peer.stream} testid="remote-stream" />
		{/each}
	</div>
{/if}
