import { browser } from '$app/environment';

type Mode = 'light' | 'dark' | 'system';
type Effective = 'light' | 'dark';

const STORAGE_KEY = 'racquet_theme';

function readMode(): Mode {
	if (!browser) return 'system';
	const stored = localStorage.getItem(STORAGE_KEY);
	return stored === 'light' || stored === 'dark' || stored === 'system' ? stored : 'system';
}

function systemEffective(): Effective {
	return browser && window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function compute(mode: Mode): Effective {
	return mode === 'system' ? systemEffective() : mode;
}

const initial = readMode();
export const theme = $state({
	mode: initial,
	effective: compute(initial)
});

if (browser) {
	window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
		if (theme.mode === 'system') {
			theme.effective = e.matches ? 'dark' : 'light';
		}
	});
}

export function setMode(mode: Mode): void {
	theme.mode = mode;
	theme.effective = compute(mode);
	if (browser) localStorage.setItem(STORAGE_KEY, mode);
}

export function cycleMode(): void {
	const next: Mode =
		theme.mode === 'light' ? 'dark' : theme.mode === 'dark' ? 'system' : 'light';
	setMode(next);
}
