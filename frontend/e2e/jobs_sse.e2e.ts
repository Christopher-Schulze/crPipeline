import { test, expect } from '@playwright/test';

// Verify that a new job pushed via SSE appears in the list

test('new job arrives via SSE', async ({ page }) => {
  await page.route('**/api/me', route => route.fulfill({ status: 200, body: JSON.stringify({ user_id: '1', org_id: 'o1', role: 'admin' }) }));
  await page.route('**/api/jobs/o1', route => route.fulfill({ status: 200, body: '[]' }));
  await page.route('**/api/jobs/new123/details', route => route.fulfill({ status: 200, body: JSON.stringify({
    id: 'new123',
    org_id: 'o1',
    document_id: 'd2',
    pipeline_id: 'p2',
    status: 'pending',
    job_created_at: new Date().toISOString(),
    document_name: 'NewDoc.pdf',
    pipeline_name: 'Pipe2',
    stage_outputs: []
  }) }));

  await page.addInitScript(() => {
    class MockEventSource {
      onopen: (() => void) | null = null;
      onmessage: ((e: MessageEvent) => void) | null = null;
      onerror: (() => void) | null = null;
      constructor(public url: string) {
        setTimeout(() => {
          this.onopen && this.onopen(new Event('open'));
          this.onmessage && this.onmessage(new MessageEvent('message', { data: JSON.stringify({ job_id: 'new123', org_id: 'o1', status: 'pending' }) }));
        }, 100);
      }
      close() {}
    }
    // @ts-ignore
    window.EventSource = MockEventSource;
  });

  await page.goto('/dashboard');
  await expect(page.locator('text=NewDoc.pdf')).toBeVisible();
});
