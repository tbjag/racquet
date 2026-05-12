import { browser } from '$app/environment';

const STORAGE_KEY = 'racquet_screen_audio_volume';

function clamp(v: number): number {
	if (Number.isNaN(v)) return 1;
	if (v < 0) return 0;
	if (v > 1) return 1;
	return v;
}

function read(): number {
	if (!browser) return 1;
	const stored = localStorage.getItem(STORAGE_KEY);
	if (stored === null) return 1;
	return clamp(parseFloat(stored));
}

export const screenAudioVolume = $state({ value: read() });

export function setScreenAudioVolume(value: number): void {
	const v = clamp(value);
	screenAudioVolume.value = v;
	if (browser) localStorage.setItem(STORAGE_KEY, String(v));
}
