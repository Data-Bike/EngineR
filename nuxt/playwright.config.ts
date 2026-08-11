import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright E2E configuration (docs/16 §15).
 *
 * Persona matrix: admin, security_officer, manager, risk_analyst.
 * Tests run against the Nuxt dev server or a preview build.
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: 1,
  reporter: 'html',
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:3000',
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
    locale: 'ru-RU',
  },
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },
  ],
  webServer: process.env.CI ? {
    command: 'npm run preview',
    port: 3000,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  } : undefined,
});
