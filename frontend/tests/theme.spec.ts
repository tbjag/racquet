import { test, expect } from '@playwright/test';
import { setupAuthenticatedUser } from './helpers';

test.describe('Theme', () => {
	test('toggle is visible in user profile after login', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		const profile = page.getByTestId('user-profile');
		await expect(profile.getByTestId('theme-toggle')).toBeVisible();
	});

	test('cycles light → dark → system', async ({ page, request }) => {
		await page.addInitScript(() => localStorage.setItem('racquet_theme', 'light'));
		await setupAuthenticatedUser(page, request);

		const html = page.locator('html');
		const toggle = page.getByTestId('theme-toggle');

		await expect(html).toHaveAttribute('data-theme', 'light');
		await expect(toggle).toHaveAttribute('data-theme-mode', 'light');

		await toggle.click();
		await expect(html).toHaveAttribute('data-theme', 'dark');
		await expect(toggle).toHaveAttribute('data-theme-mode', 'dark');

		await toggle.click();
		await expect(toggle).toHaveAttribute('data-theme-mode', 'system');

		await toggle.click();
		await expect(html).toHaveAttribute('data-theme', 'light');
		await expect(toggle).toHaveAttribute('data-theme-mode', 'light');
	});

	test('persists across reload', async ({ page, request }) => {
		await setupAuthenticatedUser(page, request);
		await page.evaluate(() => localStorage.setItem('racquet_theme', 'dark'));
		await page.reload();
		await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');

		await page.evaluate(() => localStorage.setItem('racquet_theme', 'light'));
		await page.reload();
		await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
	});

	test('no FOUC: data-theme is set synchronously before SvelteKit hydrates', async ({
		page,
		request
	}) => {
		await page.addInitScript(() => localStorage.setItem('racquet_theme', 'dark'));
		await setupAuthenticatedUser(page, request);

		// Capture the html data-theme attribute as soon as the document loads,
		// before SvelteKit bootstraps. The inline app.html script must set it.
		await page.goto('/', { waitUntil: 'commit' });
		const earlyTheme = await page.evaluate(() => document.documentElement.dataset.theme);
		expect(earlyTheme).toBe('dark');
	});

	test('system mode follows prefers-color-scheme', async ({ browser, request }) => {
		const ctx = await browser.newContext({ colorScheme: 'dark' });
		const page = await ctx.newPage();
		await page.addInitScript(() => localStorage.setItem('racquet_theme', 'system'));
		await setupAuthenticatedUser(page, request);
		await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
		await ctx.close();
	});
});
