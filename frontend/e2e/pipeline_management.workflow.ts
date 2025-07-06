import { test, expect } from '@playwright/test';

// Full pipeline management flow: create, edit, clone and delete via UI

test('user can manage pipelines', async ({ page }) => {
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  await page.goto('/pipelines');

  // create pipeline
  await page.click('text=Create New Pipeline');
  await page.fill('input[placeholder="Pipeline name"]', 'E2E Pipeline');
  await page.fill('input[placeholder="New Stage Type"]', 'parse');
  await page.click('text=Add Stage');
  page.once('dialog', d => d.accept());
  await page.click('text=Save');
  await expect(page.locator('tr', { hasText: 'E2E Pipeline' })).toBeVisible();

  // edit pipeline
  await page.locator('tr', { hasText: 'E2E Pipeline' }).locator('text=Edit').click();
  await page.fill('input[placeholder="Pipeline name"]', 'E2E Pipeline Updated');
  page.once('dialog', d => d.accept());
  await page.click('text=Save');
  await expect(page.locator('tr', { hasText: 'E2E Pipeline Updated' })).toBeVisible();

  // clone pipeline
  page.once('dialog', d => d.accept());
  await page.locator('tr', { hasText: 'E2E Pipeline Updated' }).locator('text=Clone').click();
  await expect(page.locator('tr', { hasText: 'E2E Pipeline Updated (copy)' })).toBeVisible();

  // delete original
  page.once('dialog', d => d.accept());
  page.once('dialog', d => d.accept());
  await page.locator('tr', { hasText: 'E2E Pipeline Updated' }).locator('text=Delete').click();
  await expect(page.locator('tr', { hasText: 'E2E Pipeline Updated' })).toHaveCount(0);

  // delete clone
  page.once('dialog', d => d.accept());
  page.once('dialog', d => d.accept());
  await page.locator('tr', { hasText: 'E2E Pipeline Updated (copy)' }).locator('text=Delete').click();
  await expect(page.locator('tr', { hasText: 'E2E Pipeline Updated (copy)' })).toHaveCount(0);
});
