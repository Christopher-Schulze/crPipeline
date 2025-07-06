import { defineConfig } from '@playwright/test';
import path from 'path';

export default defineConfig({
  testDir: './e2e',
  testMatch: /.*\.e2e\.ts/,
  globalSetup: path.join(__dirname, 'e2e/global-setup.ts'),
  globalTeardown: path.join(__dirname, 'e2e/global-teardown.ts'),
  use: {
    baseURL: 'http://localhost:5173',
    headless: true,
  },
});
