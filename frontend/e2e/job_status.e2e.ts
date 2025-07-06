import { test, expect } from '@playwright/test';

// Checks that recent job status is displayed

test('job status visible on dashboard', async ({ page }) => {
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  await expect(page.locator('text=Status')).toBeVisible();
});
