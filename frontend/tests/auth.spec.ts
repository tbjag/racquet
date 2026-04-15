import { test, expect } from '@playwright/test';
import { loginUser, setupAuthenticatedUser } from './helpers';

test.describe('Login', () => {
	test('login page shows Google sign-in button', async ({ page }) => {
		await page.goto('/login');
		await expect(page.getByTestId('google-login-button')).toBeVisible();
		await expect(page.getByTestId('google-login-button')).toContainText('Sign in with Google');
	});

	test('login page shows error for unauthorized account', async ({ page }) => {
		await page.goto('/login?error=not_allowed');
		await expect(page.getByTestId('error-message')).toBeVisible();
		await expect(page.getByTestId('error-message')).toContainText('not authorized');
	});

	test('login page shows error for oauth failure', async ({ page }) => {
		await page.goto('/login?error=oauth_error');
		await expect(page.getByTestId('error-message')).toBeVisible();
		await expect(page.getByTestId('error-message')).toContainText('failed');
	});
});

test.describe('Auth callback', () => {
	test('stores token and redirects to /', async ({ page, request }) => {
		const token = await loginUser(request, 'callback-test@test.com');

		await page.goto(`/auth/callback?token=${token}`);
		await expect(page).toHaveURL('/');

		const storedToken = await page.evaluate(() => localStorage.getItem('racquet_token'));
		expect(storedToken).toBe(token);
	});

	test('redirects to login without token', async ({ page }) => {
		await page.goto('/auth/callback');
		await expect(page).toHaveURL('/login');
	});
});

test.describe('Auth guard', () => {
	test('unauthenticated user is redirected to login', async ({ page }) => {
		await page.goto('/');
		await expect(page).toHaveURL('/login');
	});
});

test.describe('Profile', () => {
	test('profile shows username after login', async ({ page, request }) => {
		const { username } = await setupAuthenticatedUser(page, request);
		await expect(page.getByTestId('display-name')).toContainText(username);
	});

	test('user can update display name', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		await page.getByTestId('edit-name-button').click();
		await page.getByTestId('display-name-input').fill('New Name');
		await page.getByTestId('save-name-button').click();
		await expect(page.getByTestId('display-name')).toContainText('New Name');
	});

	test('logout redirects to login', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		await page.getByTestId('logout-button').click();
		await expect(page).toHaveURL('/login');
	});
});
