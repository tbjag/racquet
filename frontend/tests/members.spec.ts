import { test, expect, type Page } from '@playwright/test';
import { setupAuthenticatedUser, loginUser, createRoom } from './helpers';

const rnd = () => Math.random().toString(36).slice(2, 8);

async function selectRoom(page: Page, roomName: string) {
	await page
		.getByTestId('room-list')
		.locator('[data-testid="room-item"]')
		.filter({ hasText: roomName })
		.click();
	await expect(page.getByTestId('chat-area')).toBeVisible();
}

async function authedPage(browser: import('@playwright/test').Browser, token: string) {
	const ctx = await browser.newContext();
	const page = await ctx.newPage();
	await page.goto('/');
	await page.evaluate((t) => localStorage.setItem('racquet_token', t), token);
	await page.goto('/');
	return { ctx, page };
}

test.describe('Member list', () => {
	test('shows the current user when alone in a room', async ({ page, request }) => {
		const { token, username } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		const memberList = page.getByTestId('member-list');
		await expect(memberList).toBeVisible();
		await expect(memberList.getByTestId('member-item')).toHaveCount(1);
		await expect(memberList).toContainText(username);
	});

	test('updates as users join and leave', async ({ browser, request }) => {
		const emailA = `a${rnd()}@test.com`;
		const tokenA = await loginUser(request, emailA);
		const usernameA = emailA.split('@')[0];

		const emailB = `b${rnd()}@test.com`;
		const tokenB = await loginUser(request, emailB);
		const usernameB = emailB.split('@')[0];

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		const a = await authedPage(browser, tokenA);
		await selectRoom(a.page, roomName);

		await expect(a.page.getByTestId('member-list').getByTestId('member-item')).toHaveCount(1);

		const b = await authedPage(browser, tokenB);
		await selectRoom(b.page, roomName);

		await expect(a.page.getByTestId('member-list').getByTestId('member-item')).toHaveCount(2);
		await expect(a.page.getByTestId('member-list')).toContainText(usernameA);
		await expect(a.page.getByTestId('member-list')).toContainText(usernameB);
		await expect(b.page.getByTestId('member-list').getByTestId('member-item')).toHaveCount(2);

		// When B disconnects, A's member list should shrink.
		await b.ctx.close();

		await expect(a.page.getByTestId('member-list').getByTestId('member-item')).toHaveCount(1);

		await a.ctx.close();
	});
});
