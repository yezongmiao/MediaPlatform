import { existsSync } from 'node:fs';
import { defineConfig, devices } from '@playwright/test';

// Prefer the browser the user already has. Playwright can drive system Edge
// directly; downloading a pinned Chromium is only needed on CI or when no
// system browser is available. Set CHROMIUM_PATH to force a managed binary,
// or PW_BROWSER=chromium to ignore Edge.
const edgeCandidates = [
  process.env.EDGE_PATH,
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  '/usr/bin/microsoft-edge',
  '/usr/bin/microsoft-edge-stable',
].filter((candidate): candidate is string => Boolean(candidate));

const systemEdge = process.env.PW_BROWSER === 'chromium'
  ? undefined
  : edgeCandidates.find((candidate) => existsSync(candidate));
const executablePath = process.env.CHROMIUM_PATH ?? systemEdge;

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
