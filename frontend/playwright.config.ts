import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  testMatch: /.*\.e2e\.ts/,
  webServer: {
    command: 'npm run preview -- --port=4173',
    port: 4173,
    reuseExistingServer: !process.env.CI,
  },
  use: {
    baseURL: 'http://localhost:4173',
    headless: true,
  },
});
