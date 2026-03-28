import { test, expect } from '@playwright/test';
import { setupAuthenticatedUser, registerUser, loginUser, createRoom } from './helpers';

const rnd = () => Math.random().toString(36).slice(2, 8);

async function selectRoom(page: import('@playwright/test').Page, roomName: string) {
	await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();
	await expect(page.getByTestId('chat-area')).toBeVisible();
}

test.describe('Chat messages', () => {
	test('empty room shows no messages placeholder', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await expect(page.getByTestId('no-messages-placeholder')).toBeVisible();
	});

	test('sending a message displays it in the chat', async ({ page, request }) => {
		const { token, username } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await page.getByTestId('message-input').fill('hello world');
		await page.getByTestId('message-input').press('Enter');

		const message = page.getByTestId('message-item').first();
		await expect(message).toBeVisible();
		await expect(message).toContainText('hello world');
		await expect(message).toContainText(username);
	});

	test('message input clears after sending', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await page.getByTestId('message-input').fill('test message');
		await page.getByTestId('message-input').press('Enter');

		await expect(page.getByTestId('message-input')).toHaveValue('');
	});

	test('messages persist across page reload', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await page.getByTestId('message-input').fill('persistent message');
		await page.getByTestId('message-input').press('Enter');
		await expect(page.getByTestId('message-item')).toHaveCount(1);

		await page.reload();
		await selectRoom(page, roomName);

		await expect(page.getByTestId('message-item')).toHaveCount(1);
		await expect(page.getByTestId('message-item').first()).toContainText('persistent message');
	});

	test('two users see messages in real time', async ({ browser, request }) => {
		const usernameA = `a${rnd()}`;
		const passwordA = 'testpassword123';
		await registerUser(request, usernameA, passwordA);
		const tokenA = await loginUser(request, usernameA, passwordA);

		const usernameB = `b${rnd()}`;
		const passwordB = 'testpassword123';
		await registerUser(request, usernameB, passwordB);
		const tokenB = await loginUser(request, usernameB, passwordB);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		// User A joins room
		const contextA = await browser.newContext();
		const pageA = await contextA.newPage();
		await pageA.goto('http://localhost:5173/');
		await pageA.evaluate((t) => localStorage.setItem('racquet_token', t), tokenA);
		await pageA.goto('http://localhost:5173/');
		await selectRoom(pageA, roomName);

		// User B joins room
		const contextB = await browser.newContext();
		const pageB = await contextB.newPage();
		await pageB.goto('http://localhost:5173/');
		await pageB.evaluate((t) => localStorage.setItem('racquet_token', t), tokenB);
		await pageB.goto('http://localhost:5173/');
		await selectRoom(pageB, roomName);

		// User A sends a message
		await pageA.getByTestId('message-input').fill('hello from A');
		await pageA.getByTestId('message-input').press('Enter');

		// User B should see it without refreshing
		await expect(pageB.getByTestId('message-item').first()).toContainText('hello from A');
		await expect(pageB.getByTestId('message-item').first()).toContainText(usernameA);

		// User B sends a reply
		await pageB.getByTestId('message-input').fill('hello from B');
		await pageB.getByTestId('message-input').press('Enter');

		// User A should see it
		await expect(pageA.getByTestId('message-item')).toHaveCount(2);
		await expect(pageA.getByTestId('message-item').last()).toContainText('hello from B');

		await contextA.close();
		await contextB.close();
	});

	test('messages appear in chronological order', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		const messages = ['first', 'second', 'third'];
		for (const msg of messages) {
			await page.getByTestId('message-input').fill(msg);
			await page.getByTestId('message-input').press('Enter');
			await expect(page.getByTestId('message-item').last()).toContainText(msg);
		}

		const items = page.getByTestId('message-item');
		await expect(items).toHaveCount(3);
		await expect(items.nth(0)).toContainText('first');
		await expect(items.nth(1)).toContainText('second');
		await expect(items.nth(2)).toContainText('third');
	});
});
