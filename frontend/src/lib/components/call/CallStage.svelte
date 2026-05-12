<script lang="ts">
	import VideoTile from './VideoTile.svelte';

	export type RemotePeer = { userId: string; username: string; stream: MediaStream };

	type Props = {
		localStream: MediaStream | null;
		remoteStreams: RemotePeer[];
		localScreenStream: MediaStream | null;
		remoteScreenStream: { userId: string; username: string; stream: MediaStream } | null;
	};

	type Tile = {
		id: string;
		kind: 'camera' | 'screen';
		stream: MediaStream;
		label: string;
		muted: boolean;
		testid: 'local-video' | 'remote-stream' | 'screen-share-tile';
	};

	let { localStream, remoteStreams, localScreenStream, remoteScreenStream }: Props = $props();

	const tiles = $derived<Tile[]>([
		...(localScreenStream
			? [
					{
						id: 'screen:local',
						kind: 'screen' as const,
						stream: localScreenStream,
						label: 'Your screen',
						muted: true,
						testid: 'screen-share-tile' as const
					}
				]
			: []),
		...(remoteScreenStream
			? [
					{
						id: `screen:${remoteScreenStream.userId}`,
						kind: 'screen' as const,
						stream: remoteScreenStream.stream,
						label: `${remoteScreenStream.username} (screen)`,
						muted: false,
						testid: 'screen-share-tile' as const
					}
				]
			: []),
		...(localStream
			? [
					{
						id: 'cam:local',
						kind: 'camera' as const,
						stream: localStream,
						label: 'You',
						muted: true,
						testid: 'local-video' as const
					}
				]
			: []),
		...remoteStreams.map((p) => ({
			id: `cam:${p.userId}`,
			kind: 'camera' as const,
			stream: p.stream,
			label: p.username,
			muted: false,
			testid: 'remote-stream' as const
		}))
	]);

	const screenActive = $derived(localScreenStream !== null || remoteScreenStream !== null);
	const screenStream = $derived(localScreenStream ?? remoteScreenStream?.stream ?? null);

	let focusedTileId = $state<string | null>(null);
	let theaterMode = $state(false);
	let camerasVisible = $state(true);

	const focusedTile = $derived(tiles.find((t) => t.id === focusedTileId) ?? null);
	const stripTiles = $derived(focusedTile ? tiles.filter((t) => t.id !== focusedTileId) : []);

	$effect(() => {
		if (focusedTileId && !tiles.some((t) => t.id === focusedTileId)) {
			focusedTileId = null;
		}
	});

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

	function toggleFocus(id: string) {
		focusedTileId = focusedTileId === id ? null : id;
	}

	function onTileKey(e: KeyboardEvent, id: string) {
		if (e.key === 'Enter' || e.key === ' ') {
			e.preventDefault();
			toggleFocus(id);
		}
	}
</script>

<div class="call-stage">
	{#if focusedTile}
		<div class="focused-area">
			{@render renderTile(focusedTile, 'focused')}
		</div>
		{#if stripTiles.length > 0}
			<div class="strip" data-testid="tile-strip">
				{#each stripTiles as t (t.id)}
					{@render renderTile(t, 'strip')}
				{/each}
			</div>
		{/if}
	{:else}
		<div class="grid">
			{#each tiles as t (t.id)}
				{@render renderTile(t, 'grid')}
			{/each}
		</div>
	{/if}
</div>

{#snippet renderTile(t: Tile, size: 'focused' | 'strip' | 'grid')}
	<div
		class="tile"
		class:focused={size === 'focused'}
		class:strip-item={size === 'strip'}
		role="button"
		tabindex="0"
		aria-pressed={focusedTileId === t.id}
		aria-label={focusedTileId === t.id ? `Unfocus ${t.label}` : `Focus ${t.label}`}
		onclick={() => toggleFocus(t.id)}
		onkeydown={(e) => onTileKey(e, t.id)}
		data-testid={`tile-wrapper-${t.id}`}
	>
		<VideoTile stream={t.stream} testid={t.testid} muted={t.muted} />
		<span class="label">{t.label}</span>
		{#if t.kind === 'screen'}
			<button
				type="button"
				class="theater-toggle"
				data-testid="theater-toggle"
				onclick={(e) => {
					e.stopPropagation();
					theaterMode = true;
				}}
				aria-label="Enter theater mode"
			>
				Theater
			</button>
		{/if}
	</div>
{/snippet}

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
					<div class="theater-tile">
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
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		background: var(--bg-sunken);
		border-bottom: 1px solid var(--border);
	}

	.grid {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		gap: var(--space-2);
		align-content: start;
		overflow-y: auto;
	}

	.focused-area {
		flex: 1;
		min-height: 0;
		display: flex;
		justify-content: center;
		align-items: stretch;
	}

	.strip {
		flex: 0 0 auto;
		display: flex;
		flex-wrap: nowrap;
		gap: var(--space-2);
		overflow-x: auto;
		overflow-y: visible;
	}

	.tile {
		position: relative;
		width: 100%;
		aspect-ratio: 16 / 9;
		max-width: 960px;
		justify-self: center;
		min-height: 0;
		background: #000;
		border-radius: var(--radius-md);
		overflow: hidden;
		cursor: pointer;
		border: 2px solid transparent;
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
	}

	.tile:hover {
		border-color: var(--border-strong);
	}

	.tile:focus-visible {
		outline: none;
		box-shadow: var(--focus-ring);
	}

	.tile.focused {
		width: 100%;
		height: 100%;
		max-width: none;
		aspect-ratio: auto;
		border-color: var(--accent);
		cursor: zoom-out;
	}

	.strip .tile {
		width: 160px;
		height: 90px;
		max-width: none;
		flex: 0 0 auto;
		aspect-ratio: auto;
	}

	.tile :global(video) {
		width: 100%;
		height: 100%;
		display: block;
		object-fit: contain;
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

	.theater-tile {
		position: relative;
		width: 160px;
		height: 90px;
		background: #000;
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.theater-tile :global(video) {
		width: 100%;
		height: 100%;
		display: block;
		object-fit: contain;
		background: #000;
	}
</style>
