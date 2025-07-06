import { test, expect } from '@playwright/test';

// Full login against running backend seeded via scripts/seed_demo.sh

test('user can log in via UI', async ({ page }) => {
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');
  await expect(page).toHaveURL(/dashboard/);
});
