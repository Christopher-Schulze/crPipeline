import { test, expect } from '@playwright/test';

test('login success', async ({ page }) => {
  await page.route('**/api/login', route => route.fulfill({ status: 200, body: '{"success":true}' }));
  await page.route('**/api/me', route => route.fulfill({ status: 200, body: JSON.stringify({ user_id: '1', org_id: '1', role: 'admin' }) }));

  await page.goto('/login');
  await page.fill('input[type=email]', 'user@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');
  await expect(page).toHaveURL(/dashboard/);
});
