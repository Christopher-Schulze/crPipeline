import { test, expect } from '@playwright/test';
import fs from 'fs';

test('file upload', async ({ page }, testInfo) => {
  await page.route('**/api/upload**', route => route.fulfill({ status: 200, body: '{}' }));
  await page.route('**/api/me', route => route.fulfill({ status: 200, body: JSON.stringify({ user_id: '1', org_id: '1', role: 'admin' }) }));

  await page.goto('/dashboard');

  const filePath = testInfo.outputPath('sample.txt');
  fs.writeFileSync(filePath, 'hello');
  await page.setInputFiles('input[type=file]', filePath);
  await expect(page.locator('input[type=file]')).toHaveValue('');
});
