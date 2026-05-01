import { browser } from '$app/environment';

export type ScreenShareMode = 'motion' | 'detail';

const STORAGE_KEY = 'racquet_screen_share_mode';

function read(): ScreenShareMode {
	if (!browser) return 'motion';
	const stored = localStorage.getItem(STORAGE_KEY);
	return stored === 'motion' || stored === 'detail' ? stored : 'motion';
}

export const screenShareMode = $state({ mode: read() });

export function setScreenShareMode(mode: ScreenShareMode): void {
	screenShareMode.mode = mode;
	if (browser) localStorage.setItem(STORAGE_KEY, mode);
}
