import { test, expect } from '@playwright/test';

// Creates a simple pipeline via UI

test('create pipeline', async ({ page }) => {
  // login first
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  await page.goto('/pipelines');
  await page.click('text=Create New Pipeline');
  await page.fill('input[placeholder="Pipeline name"]', 'Test Pipeline');
  await page.fill('input[placeholder="New Stage Type"]', 'parse');
  await page.click('text=Add Stage');

  // accept alert on save
  page.once('dialog', dialog => dialog.accept());
  await page.click('text=Save');
});
