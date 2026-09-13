import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  use: { baseURL: 'http://127.0.0.1:8091/boxmonitor/', viewport: { width: 1440, height: 1100 }, trace: 'retain-on-failure' },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }, { name: 'firefox', use: { browserName: 'firefox' } }],
  webServer: { command: 'node server.mjs', url: 'http://127.0.0.1:8091/boxmonitor/', reuseExistingServer: !process.env.CI },
});
