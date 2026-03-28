import type { Page, APIRequestContext } from '@playwright/test';

const API_URL = 'http://localhost:3000';

export async function registerUser(
	request: APIRequestContext,
	username: string,
	password: string
): Promise<{ id: string; username: string }> {
	const res = await request.post(`${API_URL}/api/register`, {
		data: { username, password }
	});
	if (!res.ok()) throw new Error(`Register failed: ${res.status()}`);
	return res.json();
}

export async function loginUser(
	request: APIRequestContext,
	username: string,
	password: string
): Promise<string> {
	const res = await request.post(`${API_URL}/api/login`, {
		data: { username, password }
	});
	if (!res.ok()) throw new Error(`Login failed: ${res.status()}`);
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

export async function loginViaUI(page: Page, username: string, password: string) {
	await page.goto('/login');
	await page.getByTestId('username-input').fill(username);
	await page.getByTestId('password-input').fill(password);
	await page.getByTestId('submit-button').click();
	await page.waitForURL('/');
}

export async function setupAuthenticatedUser(
	page: Page,
	request: APIRequestContext
): Promise<{ username: string; token: string }> {
	const username = `u${Math.random().toString(36).slice(2, 10)}`;
	const password = 'testpassword123';
	await registerUser(request, username, password);
	const token = await loginUser(request, username, password);

	await page.goto('/');
	await page.evaluate(
		(t) => localStorage.setItem('racquet_token', t),
		token
	);
	await page.goto('/');

	return { username, token };
}
