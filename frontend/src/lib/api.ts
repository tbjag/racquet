const API_BASE = 'http://localhost:3000';

export async function getProfile(
	token: string
): Promise<{ id: string; email: string; username: string }> {
	const res = await fetch(`${API_BASE}/api/profile`, {
		headers: { Authorization: `Bearer ${token}` }
	});
	if (!res.ok) throw new Error('Failed to fetch profile');
	return res.json();
}

export async function updateProfile(
	token: string,
	username: string
): Promise<{ token: string; user: { id: string; email: string; username: string } }> {
	const res = await fetch(`${API_BASE}/api/profile`, {
		method: 'PUT',
		headers: {
			'Content-Type': 'application/json',
			Authorization: `Bearer ${token}`
		},
		body: JSON.stringify({ username })
	});
	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: 'Failed to update profile' }));
		throw new Error(body.error || 'Failed to update profile');
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
