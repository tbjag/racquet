import type { Page, APIRequestContext } from '@playwright/test';

const API_URL = 'http://localhost:3000';

export async function loginUser(request: APIRequestContext, email: string): Promise<string> {
	const res = await request.post(`${API_URL}/api/auth/test-login`, {
		data: { email }
	});
	if (!res.ok()) throw new Error(`Test login failed: ${res.status()}`);
	const body = await res.json();
	return body.token;
}

export async function createRoom(
	request: APIRequestContext,
	token: string,
	name: string
): Promise<{ id: string; name: string }> {
	const res = await request.post(`${API_URL}/api/rooms`, {
		data: { name },
		headers: { Authorization: `Bearer ${token}` }
	});
	if (!res.ok()) throw new Error(`Create room failed: ${res.status()}`);
	return res.json();
}

export async function setupAuthenticatedUser(
	page: Page,
	request: APIRequestContext
): Promise<{ username: string; token: string }> {
	const email = `test-${Math.random().toString(36).slice(2, 10)}@test.com`;
	const token = await loginUser(request, email);

	await page.goto('/');
	await page.evaluate(
		(t) => localStorage.setItem('racquet_token', t),
		token
	);
	await page.goto('/');

	// Username is derived from email prefix
	const username = email.split('@')[0];
	return { username, token };
}
