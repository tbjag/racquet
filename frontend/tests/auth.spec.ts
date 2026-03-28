import { test, expect } from '@playwright/test';
import { registerUser } from './helpers';

test.describe('Registration', () => {
	test('register page renders with form fields', async ({ page }) => {
		await page.goto('/register');
		await expect(page.getByTestId('username-input')).toBeVisible();
		await expect(page.getByTestId('password-input')).toBeVisible();
		await expect(page.getByTestId('submit-button')).toBeVisible();
	});

	test('successful registration redirects to login', async ({ page }) => {
		await page.goto('/register');
		const username = `r${Math.random().toString(36).slice(2, 10)}`;
		await page.getByTestId('username-input').fill(username);
		await page.getByTestId('password-input').fill('testpassword123');
		await page.getByTestId('submit-button').click();
		await expect(page).toHaveURL('/login');
	});

	test('duplicate username shows error', async ({ page, request }) => {
		const username = `d${Math.random().toString(36).slice(2, 10)}`;
		await registerUser(request, username, 'testpassword123');

		await page.goto('/register');
		await page.getByTestId('username-input').fill(username);
		await page.getByTestId('password-input').fill('testpassword123');
		await page.getByTestId('submit-button').click();

		await expect(page.getByTestId('error-message')).toBeVisible();
	});
});

test.describe('Login', () => {
	test('login page renders with form fields', async ({ page }) => {
		await page.goto('/login');
		await expect(page.getByTestId('username-input')).toBeVisible();
		await expect(page.getByTestId('password-input')).toBeVisible();
		await expect(page.getByTestId('submit-button')).toBeVisible();
	});

	test('successful login redirects to app', async ({ page, request }) => {
		const username = `l${Math.random().toString(36).slice(2, 10)}`;
		await registerUser(request, username, 'testpassword123');

		await page.goto('/login');
		await page.getByTestId('username-input').fill(username);
		await page.getByTestId('password-input').fill('testpassword123');
		await page.getByTestId('submit-button').click();

		await expect(page).toHaveURL('/');
	});

	test('wrong credentials shows error', async ({ page }) => {
		await page.goto('/login');
		await page.getByTestId('username-input').fill('nonexistent');
		await page.getByTestId('password-input').fill('wrongpassword1');
		await page.getByTestId('submit-button').click();

		await expect(page.getByTestId('error-message')).toBeVisible();
	});
});

test.describe('Auth guard', () => {
	test('unauthenticated user is redirected to login', async ({ page }) => {
		await page.goto('/');
		await expect(page).toHaveURL('/login');
	});
});
