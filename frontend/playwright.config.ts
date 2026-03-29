import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
	testDir: './tests',
	fullyParallel: false,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: 1,
	reporter: 'html',
	use: {
		baseURL: 'http://localhost:5173',
		testIdAttribute: 'data-testid',
		trace: 'on-first-retry'
	},
	projects: [
		{
			name: 'chromium',
			use: {
				...devices['Desktop Chrome'],
				launchOptions: {
					args: [
						'--use-fake-ui-for-media-stream',
						'--use-fake-device-for-media-stream'
					]
				}
			}
		}
	],
	webServer: [
		{
			command: 'cargo run',
			cwd: '..',
			port: 3000,
			reuseExistingServer: true,
			env: {
				DATABASE_URL: 'sqlite:/tmp/racquet-e2e.db?mode=rwc'
			},
			timeout: 120000
		},
		{
			command: 'npm run dev',
			port: 5173,
			reuseExistingServer: true
		}
	]
});
