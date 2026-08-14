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

	test('renaming a room updates the header and the sidebar', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		const newName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();

		await page.getByTestId('rename-room-button').click();
		await page.getByTestId('room-name-edit-input').fill(newName);
		await page.getByTestId('save-room-name-button').click();

		await expect(page.getByTestId('room-name-edit-input')).not.toBeVisible();
		await expect(page.getByTestId('room-header')).toContainText(newName);
		await expect(page.getByTestId('room-list')).toContainText(newName);
		await expect(page.getByTestId('room-list')).not.toContainText(roomName);
	});

	test('renaming to an existing name shows an error and keeps edit mode', async ({
		page,
		request
	}) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomA = `rm${rnd()}`;
		const roomB = `rm${rnd()}`;
		await createRoom(request, token, roomA);
		await createRoom(request, token, roomB);

		await page.reload();
		await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomB }).click();

		await page.getByTestId('rename-room-button').click();
		await page.getByTestId('room-name-edit-input').fill(roomA);
		await page.getByTestId('save-room-name-button').click();

		await expect(page.getByTestId('toast-error')).toContainText('room name already taken');
		await expect(page.getByTestId('room-name-edit-input')).toBeVisible();
	});

	test('deleting a room removes it and clears the chat area', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();
		await expect(page.getByTestId('chat-area')).toBeVisible();

		await page.getByTestId('delete-room-button').click();
		await page.getByTestId('confirm-delete-room-button').click();

		await expect(page.getByTestId('chat-area')).not.toBeVisible();
		await expect(page.getByTestId('room-list')).not.toContainText(roomName);
	});

	test('deleting a room ejects another user sitting in it', async ({ page, request, browser }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		const other = await browser.newContext();
		const otherPage = await other.newPage();
		await setupAuthenticatedUser(otherPage, request);
		await otherPage.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();
		await expect(otherPage.getByTestId('chat-area')).toBeVisible();

		await page.reload();
		await page.getByTestId('room-list').locator('[data-testid="room-item"]').filter({ hasText: roomName }).click();
		await page.getByTestId('delete-room-button').click();
		await page.getByTestId('confirm-delete-room-button').click();

		await expect(otherPage.getByTestId('chat-area')).not.toBeVisible();
		await expect(otherPage.getByTestId('room-list')).not.toContainText(roomName);

		await other.close();
	});
});
