import { test, expect } from '@playwright/test';
import { setupAuthenticatedUser, createRoom } from './helpers';

const rnd = () => Math.random().toString(36).slice(2, 8);

test.describe('Room list', () => {
	test('room list is visible after login', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		await expect(page.getByTestId('room-list')).toBeVisible();
	});

	test('create room button opens form', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		await page.getByTestId('create-room-button').click();
		await expect(page.getByTestId('room-name-input')).toBeVisible();
	});

	test('creating a room adds it to the sidebar', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;

		await page.getByTestId('create-room-button').click();
		await page.getByTestId('room-name-input').fill(roomName);
		await page.getByTestId('room-submit-button').click();

		await expect(page.getByTestId('room-name-input')).not.toBeVisible();
		await expect(page.getByTestId('room-list')).toContainText(roomName);
	});

	test('multiple rooms appear in sidebar', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomA = `rm${rnd()}`;
		const roomB = `rm${rnd()}`;

		await createRoom(request, token, roomA);
		await createRoom(request, token, roomB);

		await page.reload();
		await expect(page.getByTestId('room-list')).toContainText(roomA);
		await expect(page.getByTestId('room-list')).toContainText(roomB);
	});

	test('clicking a room selects it and shows chat area', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();

		await expect(page.getByTestId('room-list').locator('[data-testid="room-item"].active')).toHaveCount(1);
		await expect(page.getByTestId('chat-area')).toBeVisible();
	});
});
