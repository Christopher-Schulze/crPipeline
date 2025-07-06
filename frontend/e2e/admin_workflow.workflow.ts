import { test, expect } from '@playwright/test';

// Admin user invites another user and updates their role

test('admin invitation and role change', async ({ page }) => {
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  await page.goto('/admin');
  await page.click('text=Rollen\xC3\xA4nderungen');
  await page.click('text=Benutzer einladen');

  const email = `e2e-${Date.now()}@example.com`;
  await page.fill('#invite-email', email);
  const orgSelect = page.locator('#invite-org-select');
  if (await orgSelect.count()) {
    await orgSelect.selectOption({ index: 0 });
  }
  await page.click('text=Send Invitation');
  await expect(page.locator('text=Invite New User')).toBeHidden({ timeout: 10000 });

  const row = page.locator('tr', { hasText: email });
  await expect(row).toBeVisible({ timeout: 10000 });
  await row.locator('text=Edit Role').click();

  await page.selectOption('#role-select', 'org_admin');
  const modalOrgSelect = page.locator('#org-select');
  if (await modalOrgSelect.count()) {
    await modalOrgSelect.selectOption({ index: 0 });
  }
  await page.click('text=Save Changes');
  await expect(page.locator('text=Edit User:')).toBeHidden({ timeout: 10000 });

  await expect(row).toContainText('org_admin');
});
