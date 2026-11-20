import { existsSync } from 'node:fs';
import path from 'node:path';
import { defineConfig, devices } from '@playwright/test';

const cachedChromium = path.join(
  process.env.HOME ?? '',
  'Library/Caches/ms-playwright/chromium-1228/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing',
);
const executablePath = process.env.CHROMIUM_PATH ?? (existsSync(cachedChromium) ? cachedChromium : undefined);

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  workers: 1,
  retries: 0,
  timeout: 90_000,
  expect: { timeout: 15_000 },
  reporter: [
    ['list'],
    ['html', { outputFolder: 'artifacts/playwright-report', open: 'never' }],
  ],
  use: {
    baseURL: 'http://127.0.0.1:4390',
    viewport: { width: 1440, height: 1000 },
    deviceScaleFactor: 1,
    screenshot: 'only-on-failure',
    trace: 'retain-on-failure',
    launchOptions: executablePath ? { executablePath } : {},
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'], viewport: { width: 1440, height: 1000 } },
    },
  ],
  webServer: {
    command: 'node scripts/e2e-server.mjs',
    url: 'http://127.0.0.1:4390/api/health',
    reuseExistingServer: false,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
});
