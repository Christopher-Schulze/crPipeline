import { test, expect } from '@playwright/test';

// Create, edit, clone and delete a pipeline using the real UI

test('pipeline CRUD via UI', async ({ page }) => {
  // Login with seeded admin user
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  await page.goto('/pipelines');

  // --- create pipeline ---
  await page.click('text=Create New Pipeline');
  await page.fill('input[placeholder="Pipeline name"]', 'UI E2E Pipeline');
  await page.fill('input[placeholder="New Stage Type"]', 'parse');
  await page.click('text=Add Stage');
  page.once('dialog', d => d.accept());
  await page.click('text=Save');

  const createdRow = page.locator('tr', { hasText: 'UI E2E Pipeline' });
  await expect(createdRow).toBeVisible();

  // --- edit pipeline ---
  await createdRow.locator('text=Edit').click();
  await page.fill('input[placeholder="Pipeline name"]', 'UI E2E Pipeline Updated');
  page.once('dialog', d => d.accept());
  await page.click('text=Save');

  const updatedRow = page.locator('tr', { hasText: 'UI E2E Pipeline Updated' });
  await expect(updatedRow).toBeVisible();

  // --- clone pipeline ---
  page.once('dialog', d => d.accept());
  await updatedRow.locator('text=Clone').click();
  const copyRow = page.locator('tr', { hasText: 'UI E2E Pipeline Updated (copy)' });
  await expect(copyRow).toBeVisible();

  // --- delete cloned pipeline ---
  page.once('dialog', d => d.accept());
  await copyRow.locator('text=Delete').click();
  await expect(page.locator('tr', { hasText: 'UI E2E Pipeline Updated (copy)' })).toHaveCount(0);

  // --- delete original pipeline ---
  page.once('dialog', d => d.accept());
  await updatedRow.locator('text=Delete').click();
  await expect(page.locator('tr', { hasText: 'UI E2E Pipeline Updated' })).toHaveCount(0);
});
