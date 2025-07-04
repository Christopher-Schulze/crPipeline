import { test, expect } from '@playwright/test';

// This test mocks backend endpoints to exercise the login flow
// and the pipeline editor in the built application.

test('login and create pipeline', async ({ page }) => {
  // Initial checkAuth request should respond with 401 to show login page
  await page.route('**/api/me', route => {
    const isAuthCheck = route.request().method() === 'GET';
    route.fulfill({ status: isAuthCheck ? 401 : 200, body: '{}' });
  });

  // Mock login endpoint
  await page.route('**/api/login', route => {
    route.fulfill({ status: 200, body: '{}' });
  });

  // After login, /api/me should return session info
  await page.route('**/api/me', route => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ org_id: 'testorg', user_id: 'user', role: 'admin' })
    });
  });

  // Pipelines API
  await page.route('**/api/pipelines/testorg', route => {
    route.fulfill({ status: 200, contentType: 'application/json', body: '[]' });
  });
  await page.route('**/api/pipelines', route => {
    if (route.request().method() === 'POST') {
      route.fulfill({ status: 200, body: '{}' });
    } else {
      route.continue();
    }
  });

  // Open login page
  await page.goto('/login');
  await page.fill('input[type="email"]', 'user@example.com');
  await page.fill('input[type="password"]', 'password');
  await page.click('text=Login');

  // Wait for redirect to dashboard after login
  await page.waitForURL('/dashboard');

  // Navigate to pipelines page
  await page.goto('/pipelines');
  await page.click('text=Create New Pipeline');

  await page.fill('input[placeholder="Pipeline name"]', 'My Pipeline');
  await page.fill('input[placeholder="New Stage Type"]', 'parse');
  await page.click('text=Add Stage');

  const [req] = await Promise.all([
    page.waitForRequest(request => request.url().includes('/api/pipelines') && request.method() === 'POST'),
    page.click('text=Save')
  ]);

  expect(req.method()).toBe('POST');
});
