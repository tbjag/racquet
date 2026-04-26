import { test, expect } from '@playwright/test';
import { setupAuthenticatedUser } from './helpers';

test.describe('Error toasts', () => {
	test('failed GET /api/rooms shows an error toast', async ({ page, request }) => {
		// Intercept BEFORE navigating so the initial load fails.
		await page.route('**/api/rooms', (route) => {
			if (route.request().method() === 'GET') {
				route.fulfill({ status: 500, contentType: 'application/json', body: '{}' });
			} else {
				route.continue();
			}
		});

		await setupAuthenticatedUser(page, request);

		const toast = page.getByTestId('toast-error');
		await expect(toast).toBeVisible();
		await expect(toast).toContainText('rooms');
	});

	test('failed POST /api/rooms shows an error toast', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);

		await page.route('**/api/rooms', (route) => {
			if (route.request().method() === 'POST') {
				route.fulfill({ status: 500, contentType: 'application/json', body: '{}' });
			} else {
				route.continue();
			}
		});

		await page.getByTestId('create-room-button').click();
		await page.getByTestId('room-name-input').fill('test-room');
		await page.getByTestId('room-submit-button').click();

		await expect(page.getByTestId('toast-error')).toBeVisible();
	});

	test('toast can be dismissed by clicking it', async ({ page, request }) => {
		await page.route('**/api/rooms', (route) => {
			if (route.request().method() === 'GET') {
				route.fulfill({ status: 500, contentType: 'application/json', body: '{}' });
			} else {
				route.continue();
			}
		});

		await setupAuthenticatedUser(page, request);

		const toast = page.getByTestId('toast-error');
		await expect(toast).toBeVisible();
		await toast.click();
		await expect(toast).not.toBeVisible();
	});

	test('toast auto-dismisses after its TTL', async ({ page, request }) => {
		await page.route('**/api/rooms', (route) => {
			if (route.request().method() === 'GET') {
				route.fulfill({ status: 500, contentType: 'application/json', body: '{}' });
			} else {
				route.continue();
			}
		});

		await setupAuthenticatedUser(page, request);

		const toast = page.getByTestId('toast-error');
		await expect(toast).toBeVisible();
		// Default error TTL is 8000ms; allow extra slack.
		await expect(toast).not.toBeVisible({ timeout: 12000 });
	});
});
