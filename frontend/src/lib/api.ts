const API_BASE = 'http://localhost:3000';

export async function register(
	username: string,
	password: string
): Promise<{ id: string; username: string }> {
	const res = await fetch(`${API_BASE}/api/register`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ username, password })
	});
	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: 'Registration failed' }));
		throw new Error(body.error || 'Registration failed');
	}
	return res.json();
}

export async function login(
	username: string,
	password: string
): Promise<{ token: string }> {
	const res = await fetch(`${API_BASE}/api/login`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ username, password })
	});
	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: 'Invalid credentials' }));
		throw new Error(body.error || 'Invalid credentials');
	}
	return res.json();
}

export async function getRooms(
	token: string
): Promise<Array<{ id: string; name: string; created_by: string; created_at: string }>> {
	const res = await fetch(`${API_BASE}/api/rooms`, {
		headers: { Authorization: `Bearer ${token}` }
	});
	if (!res.ok) throw new Error('Failed to fetch rooms');
	return res.json();
}

export async function createRoom(
	token: string,
	name: string
): Promise<{ id: string; name: string; created_by: string; created_at: string }> {
	const res = await fetch(`${API_BASE}/api/rooms`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json',
			Authorization: `Bearer ${token}`
		},
		body: JSON.stringify({ name })
	});
	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: 'Failed to create room' }));
		throw new Error(body.error || 'Failed to create room');
	}
	return res.json();
}

export async function getMessages(
	token: string,
	roomId: string,
	limit?: number,
	before?: string
): Promise<
	Array<{
		id: string;
		room_id: string;
		user_id: string;
		username: string;
		content: string;
		created_at: string;
	}>
> {
	const params = new URLSearchParams();
	if (limit) params.set('limit', String(limit));
	if (before) params.set('before', before);
	const qs = params.toString();
	const res = await fetch(`${API_BASE}/api/rooms/${roomId}/messages${qs ? `?${qs}` : ''}`, {
		headers: { Authorization: `Bearer ${token}` }
	});
	if (!res.ok) throw new Error('Failed to fetch messages');
	return res.json();
}
