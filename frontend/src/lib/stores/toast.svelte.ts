import { browser } from '$app/environment';

export type ToastKind = 'error' | 'info' | 'success';

export type Toast = {
	id: string;
	kind: ToastKind;
	message: string;
};

export const toasts = $state<Toast[]>([]);

const DEFAULT_TTL: Record<ToastKind, number> = {
	error: 8000,
	info: 5000,
	success: 5000
};

export function pushToast(input: { kind: ToastKind; message: string; ttlMs?: number }): string {
	const id = crypto.randomUUID();
	toasts.push({ id, kind: input.kind, message: input.message });
	const ttl = input.ttlMs ?? DEFAULT_TTL[input.kind];
	if (browser && ttl > 0) {
		setTimeout(() => dismiss(id), ttl);
	}
	return id;
}

export function dismiss(id: string): void {
	const i = toasts.findIndex((t) => t.id === id);
	if (i !== -1) toasts.splice(i, 1);
}
