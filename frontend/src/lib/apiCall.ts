import { pushToast } from '$lib/stores/toast.svelte';

export async function apiCall<T>(
	fn: () => Promise<T>,
	opts?: { errorMessage?: string; onError?: (err: Error) => void }
): Promise<T | null> {
	try {
		return await fn();
	} catch (err) {
		const error = err instanceof Error ? err : new Error(String(err));
		pushToast({ kind: 'error', message: opts?.errorMessage ?? error.message });
		opts?.onError?.(error);
		return null;
	}
}
