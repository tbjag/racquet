import { browser } from '$app/environment';

const STORAGE_KEY = 'racquet_audio_smoothing';

function read(): boolean {
	if (!browser) return true;
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === 'true') return true;
	if (stored === 'false') return false;
	return true;
}

export const audioSmoothing = $state({ enabled: read() });

export function setAudioSmoothing(enabled: boolean): void {
	audioSmoothing.enabled = enabled;
	if (browser) localStorage.setItem(STORAGE_KEY, String(enabled));
}
