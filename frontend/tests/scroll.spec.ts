import { test, expect, type Page } from '@playwright/test';
import { loginUser, createRoom } from './helpers';

const rnd = () => Math.random().toString(36).slice(2, 8);

async function authedPage(browser: import('@playwright/test').Browser, token: string) {
	const ctx = await browser.newContext();
	const page = await ctx.newPage();
	await page.goto('/');
	await page.evaluate((t) => localStorage.setItem('racquet_token', t), token);
	await page.goto('/');
	return { ctx, page };
}

async function selectRoom(page: Page, roomName: string) {
	await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();
	await expect(page.getByTestId('chat-area')).toBeVisible();
}

test.describe('Message list auto-scroll', () => {
	test('sticks to bottom when user is at the bottom; stays put when user has scrolled up', async ({
		browser,
		request
	}) => {
		const tokenA = await loginUser(request, `a${rnd()}@test.com`);
		const tokenB = await loginUser(request, `b${rnd()}@test.com`);
		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		const a = await authedPage(browser, tokenA);
		const b = await authedPage(browser, tokenB);
		await selectRoom(a.page, roomName);
		await selectRoom(b.page, roomName);

		// Fill the room with enough messages that the list overflows.
		for (let i = 0; i < 40; i++) {
			await a.page.getByTestId('message-input').fill(`msg ${i}`);
			await a.page.getByTestId('message-input').press('Enter');
		}
		await expect(b.page.getByTestId('message-item')).toHaveCount(40);

		const listB = b.page.getByTestId('message-list');

		// Scenario 1: B is at the bottom by default. A sends a message; B should still be near bottom.
		await expect
			.poll(async () =>
				listB.evaluate((el) => el.scrollHeight - el.scrollTop - el.clientHeight)
			)
			.toBeLessThan(80);

		await a.page.getByTestId('message-input').fill('arrived while at bottom');
		await a.page.getByTestId('message-input').press('Enter');
		await expect(b.page.getByTestId('message-item')).toHaveCount(41);

		const distanceFromBottomAfter = await listB.evaluate(
			(el) => el.scrollHeight - el.scrollTop - el.clientHeight
		);
		expect(distanceFromBottomAfter).toBeLessThan(80);

		// Scenario 2: B scrolls to the top. New incoming message must NOT yank B back down.
		await listB.evaluate((el) => {
			el.scrollTop = 0;
		});
		const scrollBefore = await listB.evaluate((el) => el.scrollTop);
		expect(scrollBefore).toBe(0);

		await a.page.getByTestId('message-input').fill('arrived while scrolled up');
		await a.page.getByTestId('message-input').press('Enter');
		await expect(b.page.getByTestId('message-item')).toHaveCount(42);

		// Give any errant auto-scroll a moment to fire before checking.
		await b.page.waitForTimeout(200);
		const scrollAfter = await listB.evaluate((el) => el.scrollTop);
		expect(scrollAfter).toBe(scrollBefore);

		await a.ctx.close();
		await b.ctx.close();
	});
});
