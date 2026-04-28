export type ConnectionStatus =
	| 'closed'
	| 'connecting'
	| 'open'
	| 'reconnecting'
	| 'auth_failed';

class ConnectionStore {
	status = $state<ConnectionStatus>('closed');
	nextRetryAt = $state<number | null>(null); // epoch ms; null when not waiting

	set(status: ConnectionStatus, nextRetryAt: number | null = null) {
		this.status = status;
		this.nextRetryAt = nextRetryAt;
	}
}

export const connection = new ConnectionStore();
