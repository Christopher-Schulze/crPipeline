import { FullConfig } from '@playwright/test';
import { execSync } from 'child_process';
import http from 'http';
import path from 'path';

async function waitFor(url: string, timeout = 60000) {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      await new Promise<void>((resolve, reject) => {
        const req = http.get(url, res => { res.resume(); resolve(); });
        req.on('error', reject);
      });
      return;
    } catch {
      await new Promise(res => setTimeout(res, 1000));
    }
  }
  throw new Error(`Timed out waiting for ${url}`);
}

export default async function globalSetup(config: FullConfig) {
  const root = path.join(__dirname, '..', '..');
  execSync('docker compose up -d', { cwd: root, stdio: 'inherit' });
  try {
    await waitFor('http://localhost:5173');
    await waitFor('http://localhost:8080/health');
    execSync('./scripts/seed_demo.sh', { cwd: root, stdio: 'inherit' });
  } catch (err) {
    execSync('docker compose down', { cwd: root, stdio: 'inherit' });
    throw err;
  }
}
