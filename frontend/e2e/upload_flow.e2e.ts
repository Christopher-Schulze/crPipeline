import { test, expect } from '@playwright/test';
import fs from 'fs';

// Login and upload a small text file

test('document upload works', async ({ page }, testInfo) => {
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  const filePath = testInfo.outputPath('sample.txt');
  fs.writeFileSync(filePath, 'hello');
  const fileInput = page.locator('input[type=file]');
  await fileInput.setInputFiles(filePath);
  // After upload input should be cleared
  await expect(fileInput).toHaveValue('');
});
