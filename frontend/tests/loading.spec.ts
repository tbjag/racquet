import { test, expect } from '@playwright/test';
import { setupAuthenticatedUser } from './helpers';

const rnd = () => Math.random().toString(36).slice(2, 8);

test.describe('CreateRoomForm loading state', () => {
	test('submit button is disabled when input is empty or whitespace', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);

		await page.getByTestId('create-room-button').click();

		await expect(page.getByTestId('room-submit-button')).toBeDisabled();

		await page.getByTestId('room-name-input').fill('   ');
		await expect(page.getByTestId('room-submit-button')).toBeDisabled();

		await page.getByTestId('room-name-input').fill('valid-name');
		await expect(page.getByTestId('room-submit-button')).toBeEnabled();
	});

	test('submit button shows loading state while POST /api/rooms is in flight', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);

		// Delay the POST response so we can observe the in-flight UI.
		await page.route('**/api/rooms', async (route) => {
			if (route.request().method() === 'POST') {
				await new Promise((r) => setTimeout(r, 1500));
				await route.continue();
			} else {
				await route.continue();
			}
		});

		await page.getByTestId('create-room-button').click();
		await page.getByTestId('room-name-input').fill(`slow-${rnd()}`);
		await page.getByTestId('room-submit-button').click();

		// Mid-flight: input + button disabled, button text indicates work.
		await expect(page.getByTestId('room-submit-button')).toBeDisabled();
		await expect(page.getByTestId('room-submit-button')).toHaveText(/creating/i);
		await expect(page.getByTestId('room-name-input')).toBeDisabled();

		// After the request resolves, the form closes (parent unmounts it).
		await expect(page.getByTestId('room-name-input')).not.toBeVisible();
	});
});
