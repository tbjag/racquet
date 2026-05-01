import { test, expect } from '@playwright/test';
import { setupAuthenticatedUser, createRoom } from './helpers';

const rnd = () => Math.random().toString(36).slice(2, 8);

async function selectRoom(page: import('@playwright/test').Page, roomName: string) {
	await page
		.getByTestId('room-list')
		.locator('[data-testid="room-item"]')
		.filter({ hasText: roomName })
		.click();
	await expect(page.getByTestId('chat-area')).toBeVisible();
}

// Stubs getDisplayMedia to return a fake getUserMedia stream so screen-share
// tests can run in headless Chromium without real desktop capture.
async function stubGetDisplayMedia(page: import('@playwright/test').Page) {
	await page.addInitScript(() => {
		const md = navigator.mediaDevices as MediaDevices & {
			getDisplayMedia?: (c?: MediaStreamConstraints) => Promise<MediaStream>;
		};
		md.getDisplayMedia = async () =>
			navigator.mediaDevices.getUserMedia({ video: true, audio: true });
	});
}

test.describe('WebRTC calls', () => {
	test('call button appears when room is selected', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await expect(page.getByTestId('call-button')).toBeVisible();
		await expect(page.getByTestId('call-button')).toHaveText('Join Call');
	});

	test('clicking join call shows local video and changes button to leave call', async ({
		page,
		request
	}) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await page.getByTestId('call-button').click();

		await expect(page.getByTestId('local-video')).toBeVisible();
		await expect(page.getByTestId('call-button')).toHaveText('Leave Call');
	});

	test('clicking leave call hides local video and reverts button', async ({
		page,
		request
	}) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		// Join then leave
		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('local-video')).toBeVisible();

		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('local-video')).not.toBeVisible();
		await expect(page.getByTestId('call-button')).toHaveText('Join Call');
	});

	test('mute button toggles audio mute state', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();

		const muteBtn = page.getByTestId('mute-button');
		await expect(muteBtn).toBeVisible();
		await expect(muteBtn).toHaveText('Mute');

		await muteBtn.click();
		await expect(muteBtn).toHaveText('Unmute');

		await muteBtn.click();
		await expect(muteBtn).toHaveText('Mute');
	});

	test('video toggle button toggles video state', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();

		const videoBtn = page.getByTestId('video-toggle-button');
		await expect(videoBtn).toBeVisible();
		await expect(videoBtn).toHaveText('Video Off');

		await videoBtn.click();
		await expect(videoBtn).toHaveText('Video On');

		await videoBtn.click();
		await expect(videoBtn).toHaveText('Video Off');
	});

	test('two users in a call see each other\'s remote streams', async ({
		browser,
		request
	}) => {
		// Set up two browser contexts with permissions
		const contextA = await browser.newContext({
			permissions: ['camera', 'microphone']
		});
		const contextB = await browser.newContext({
			permissions: ['camera', 'microphone']
		});
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();

		// Create two authenticated users
		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		const { token: tokenB } = await setupAuthenticatedUser(pageB, request);

		// Create a shared room
		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		// Both users navigate and select the room
		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		// Both join the call
		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();

		// Each user should see a remote stream from the other user
		await expect(pageA.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await contextA.close();
		await contextB.close();
	});

	test('user leaving call removes their remote stream from other user', async ({
		browser,
		request
	}) => {
		const contextA = await browser.newContext({
			permissions: ['camera', 'microphone']
		});
		const contextB = await browser.newContext({
			permissions: ['camera', 'microphone']
		});
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		const { token: tokenB } = await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		// Both join
		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();

		// Wait for remote streams to appear
		await expect(pageA.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		// User B leaves the call
		await pageB.getByTestId('call-button').click();

		// User A should no longer see the remote stream
		await expect(pageA.locator('[data-testid="remote-stream"]')).toHaveCount(0, {
			timeout: 10000
		});

		await contextA.close();
		await contextB.close();
	});

	test('screen share button is hidden until joined to a call', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await expect(page.getByTestId('screen-share-button')).toHaveCount(0);

		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('screen-share-button')).toBeVisible();
		await expect(page.getByTestId('screen-share-button')).toHaveText('Share Screen');
	});

	test('user A starts screen share, user B sees a focused screen tile', async ({
		browser,
		request
	}) => {
		const contextA = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const contextB = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();
		await stubGetDisplayMedia(pageA);
		await stubGetDisplayMedia(pageB);

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await pageA.getByTestId('screen-share-button').click();

		await expect(pageB.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });
		// Camera tile for A should still be present on B's side
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1);

		await contextA.close();
		await contextB.close();
	});

	test('user A stops screen share, focused tile vanishes and camera tile remains', async ({
		browser,
		request
	}) => {
		const contextA = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const contextB = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();
		await stubGetDisplayMedia(pageA);
		await stubGetDisplayMedia(pageB);

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await pageA.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });
		await expect(pageA.getByTestId('screen-share-button')).toHaveText('Stop Sharing');

		await pageA.getByTestId('screen-share-button').click();

		await expect(pageB.getByTestId('screen-share-tile')).toHaveCount(0, { timeout: 10000 });
		await expect(pageA.getByTestId('screen-share-button')).toHaveText('Share Screen');
		// Camera tile is still there
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1);

		await contextA.close();
		await contextB.close();
	});

	test("user B's share button is disabled while user A is sharing", async ({
		browser,
		request
	}) => {
		const contextA = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const contextB = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();
		await stubGetDisplayMedia(pageA);
		await stubGetDisplayMedia(pageB);

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await pageA.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });

		await expect(pageB.getByTestId('screen-share-button')).toBeDisabled();

		// When A stops, B's button becomes enabled again
		await pageA.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-button')).toBeEnabled({ timeout: 10000 });

		await contextA.close();
		await contextB.close();
	});

	test('if sharer leaves the call mid-share, the other user can start sharing', async ({
		browser,
		request
	}) => {
		const contextA = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const contextB = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();
		await stubGetDisplayMedia(pageA);
		await stubGetDisplayMedia(pageB);

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await pageA.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });
		await expect(pageB.getByTestId('screen-share-button')).toBeDisabled();

		// A leaves the call without stopping share first
		await pageA.getByTestId('call-button').click();

		await expect(pageB.getByTestId('screen-share-tile')).toHaveCount(0, { timeout: 10000 });
		await expect(pageB.getByTestId('screen-share-button')).toBeEnabled({ timeout: 10000 });

		// B can now start
		await pageB.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-button')).toHaveText('Stop Sharing', {
			timeout: 10000
		});

		await contextA.close();
		await contextB.close();
	});

	test('theater toggle expands screen share into a full-viewport overlay', async ({
		page,
		request
	}) => {
		await stubGetDisplayMedia(page);
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();
		await page.getByTestId('screen-share-button').click();
		await expect(page.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });

		await page.getByTestId('theater-toggle').click();

		await expect(page.getByTestId('theater-screen-tile')).toBeVisible();
		await expect(page.getByTestId('theater-exit')).toBeVisible();
	});

	test('pressing Escape exits theater mode', async ({ page, request }) => {
		await stubGetDisplayMedia(page);
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();
		await page.getByTestId('screen-share-button').click();
		await expect(page.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });
		await page.getByTestId('theater-toggle').click();
		await expect(page.getByTestId('theater-screen-tile')).toBeVisible();

		await page.keyboard.press('Escape');

		await expect(page.getByTestId('theater-screen-tile')).toHaveCount(0);
		// Regular layout still has the screen tile
		await expect(page.getByTestId('screen-share-tile')).toBeVisible();
	});

	test('clicking theater-exit returns to the normal call stage', async ({ page, request }) => {
		await stubGetDisplayMedia(page);
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();
		await page.getByTestId('screen-share-button').click();
		await expect(page.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });
		await page.getByTestId('theater-toggle').click();
		await expect(page.getByTestId('theater-screen-tile')).toBeVisible();

		await page.getByTestId('theater-exit').click();

		await expect(page.getByTestId('theater-screen-tile')).toHaveCount(0);
		await expect(page.getByTestId('screen-share-tile')).toBeVisible();
	});

	test('theater mode auto-exits when the screen share ends', async ({ browser, request }) => {
		const contextA = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const contextB = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();
		await stubGetDisplayMedia(pageA);
		await stubGetDisplayMedia(pageB);

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await pageA.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });

		// B enters theater
		await pageB.getByTestId('theater-toggle').click();
		await expect(pageB.getByTestId('theater-screen-tile')).toBeVisible();

		// A stops sharing — B's theater overlay should auto-exit
		await pageA.getByTestId('screen-share-button').click();

		await expect(pageB.getByTestId('theater-screen-tile')).toHaveCount(0, { timeout: 10000 });

		await contextA.close();
		await contextB.close();
	});

	test('chat-toggle hides and restores message list during a call', async ({
		page,
		request
	}) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		// Toggle is hidden until the user is in a call
		await expect(page.getByTestId('chat-toggle')).toHaveCount(0);

		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('local-video')).toBeVisible();

		// Chat is visible by default
		await expect(page.getByTestId('message-list')).toBeVisible();

		const chatToggle = page.getByTestId('chat-toggle');
		await expect(chatToggle).toBeVisible();
		await expect(chatToggle).toHaveText('Hide Chat');

		// Hide
		await chatToggle.click();
		await expect(page.getByTestId('message-list')).toHaveCount(0);
		await expect(chatToggle).toHaveText('Show Chat');

		// Restore
		await chatToggle.click();
		await expect(page.getByTestId('message-list')).toBeVisible();
		await expect(chatToggle).toHaveText('Hide Chat');
	});

	test('chatHidden resets when leaving the call', async ({ page, request }) => {
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		await page.getByTestId('call-button').click();
		await page.getByTestId('chat-toggle').click();
		await expect(page.getByTestId('message-list')).toHaveCount(0);

		// Leave the call — chat returns automatically
		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('message-list')).toBeVisible();

		// Re-join — chat is visible (state was reset)
		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('message-list')).toBeVisible();
		await expect(page.getByTestId('chat-toggle')).toHaveText('Hide Chat');
	});

	test('screen-share-mode toggle is hidden until in-call, defaults to motion, persists across reload', async ({
		page,
		request
	}) => {
		await stubGetDisplayMedia(page);
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);

		// Hidden when not in a call
		await expect(page.getByTestId('screen-share-mode-motion')).toHaveCount(0);
		await expect(page.getByTestId('screen-share-mode-detail')).toHaveCount(0);

		await page.getByTestId('call-button').click();

		const motion = page.getByTestId('screen-share-mode-motion');
		const detail = page.getByTestId('screen-share-mode-detail');

		// Default = motion
		await expect(motion).toBeVisible();
		await expect(detail).toBeVisible();
		await expect(motion).toHaveAttribute('aria-pressed', 'true');
		await expect(detail).toHaveAttribute('aria-pressed', 'false');

		// Switch to detail
		await detail.click();
		await expect(detail).toHaveAttribute('aria-pressed', 'true');
		await expect(motion).toHaveAttribute('aria-pressed', 'false');

		// Persists across reload
		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();
		await expect(page.getByTestId('screen-share-mode-detail')).toHaveAttribute(
			'aria-pressed',
			'true'
		);
	});

	test('mode toggle does not break the screen-share flow when switched mid-share', async ({
		page,
		request
	}) => {
		await stubGetDisplayMedia(page);
		const { token } = await setupAuthenticatedUser(page, request);
		const roomName = `rm${rnd()}`;
		await createRoom(request, token, roomName);

		await page.reload();
		await selectRoom(page, roomName);
		await page.getByTestId('call-button').click();

		await page.getByTestId('screen-share-mode-detail').click();
		await page.getByTestId('screen-share-button').click();
		await expect(page.getByTestId('screen-share-tile')).toBeVisible();

		// Mid-share switch — share keeps running
		await page.getByTestId('screen-share-mode-motion').click();
		await expect(page.getByTestId('screen-share-tile')).toBeVisible();

		// Stop still works
		await page.getByTestId('screen-share-button').click();
		await expect(page.getByTestId('screen-share-tile')).toHaveCount(0);
	});

	test('theater overlay shows remote cameras and the toggle hides them', async ({
		browser,
		request
	}) => {
		const contextA = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const contextB = await browser.newContext({ permissions: ['camera', 'microphone'] });
		const pageA = await contextA.newPage();
		const pageB = await contextB.newPage();
		await stubGetDisplayMedia(pageA);
		await stubGetDisplayMedia(pageB);

		const { token: tokenA } = await setupAuthenticatedUser(pageA, request);
		await setupAuthenticatedUser(pageB, request);

		const roomName = `rm${rnd()}`;
		await createRoom(request, tokenA, roomName);

		await pageA.reload();
		await pageB.reload();
		await selectRoom(pageA, roomName);
		await selectRoom(pageB, roomName);

		await pageA.getByTestId('call-button').click();
		await pageB.getByTestId('call-button').click();
		await expect(pageB.locator('[data-testid="remote-stream"]')).toHaveCount(1, {
			timeout: 10000
		});

		await pageA.getByTestId('screen-share-button').click();
		await expect(pageB.getByTestId('screen-share-tile')).toBeVisible({ timeout: 10000 });

		await pageB.getByTestId('theater-toggle').click();
		await expect(pageB.getByTestId('theater-screen-tile')).toBeVisible();

		// B should see A's camera in the overlay panel
		await expect(pageB.getByTestId('theater-cameras')).toBeVisible();
		await expect(
			pageB.getByTestId('theater-cameras').locator('[data-testid="remote-stream"]')
		).toHaveCount(1);

		// Toggle hides the panel
		await pageB.getByTestId('theater-cameras-toggle').click();
		await expect(pageB.getByTestId('theater-cameras')).toHaveCount(0);

		// Toggle restores it
		await pageB.getByTestId('theater-cameras-toggle').click();
		await expect(pageB.getByTestId('theater-cameras')).toBeVisible();

		await contextA.close();
		await contextB.close();
	});
});
