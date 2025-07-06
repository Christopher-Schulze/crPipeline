import { test, expect } from '@playwright/test';
import fs from 'fs';

// Full document upload and analysis flow

test('document upload triggers analysis pipeline', async ({ page }, testInfo) => {
  await page.goto('/login');
  await page.fill('input[type=email]', 'demo@example.com');
  await page.fill('input[type=password]', 'password');
  await page.click('button[type=submit]');
  await page.waitForURL('/dashboard');

  // Get organization ID of logged in user
  const orgId = await page.evaluate(async () => {
    const res = await fetch('/api/me');
    const data = await res.json();
    return data.org_id as string;
  });

  // Create simple pipeline via API
  const pipelineId = await page.evaluate(async (orgId) => {
    const res = await fetch('/api/pipelines', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ org_id: orgId, name: 'E2E Pipeline', stages: [{ type: 'parse' }] })
    });
    const data = await res.json();
    return data.id as string;
  }, orgId);

  // Prepare file and send upload request from browser context
  const filePath = testInfo.outputPath('sample.txt');
  fs.writeFileSync(filePath, 'hello world');
  const fileBase64 = fs.readFileSync(filePath, 'base64');

  const jobId = await page.evaluate(async ({ orgId, pipelineId, fileBase64 }) => {
    const bytes = Uint8Array.from(atob(fileBase64), c => c.charCodeAt(0));
    const file = new File([bytes], 'sample.txt', { type: 'text/plain' });
    const fd = new FormData();
    fd.append('file', file);
    const uploadRes = await fetch(`/api/upload?org_id=${orgId}&pipeline_id=${pipelineId}&is_target=true`, {
      method: 'POST',
      body: fd
    });
    const doc = await uploadRes.json();
    for (let i = 0; i < 30; i++) {
      const list = await fetch(`/api/jobs/${orgId}`).then(r => r.json());
      const job = list.find((j: any) => j.document_id === doc.id);
      if (job) return job.id as string;
      await new Promise(res => setTimeout(res, 1000));
    }
    return null;
  }, { orgId, pipelineId, fileBase64 });

  expect(jobId).not.toBeNull();

  await page.waitForFunction(async (id => {
    const r = await fetch(`/api/jobs/${id}/details`);
    if (!r.ok) return false;
    const d = await r.json();
    return d.status === 'completed';
  }), jobId, { timeout: 90000 });

  const outputCount = await page.evaluate(async id => {
    const res = await fetch(`/api/jobs/${id}/details`);
    const data = await res.json();
    return data.stage_outputs.length as number;
  }, jobId);

  expect(outputCount).toBeGreaterThan(0);
});
