<script lang="ts">
	type Props = {
		stream: MediaStream;
		testid?: string;
		muted?: boolean;
		class?: string;
	};

	let { stream, testid, muted = false, class: className = '' }: Props = $props();

	function bindStream(node: HTMLVideoElement, s: MediaStream) {
		node.srcObject = s;
		return {
			update(next: MediaStream) {
				node.srcObject = next;
			},
			destroy() {
				node.srcObject = null;
			}
		};
	}
</script>

<!-- svelte-ignore a11y_media_has_caption -->
<video
	data-testid={testid}
	class={className}
	autoplay
	{muted}
	playsinline
	use:bindStream={stream}
></video>
