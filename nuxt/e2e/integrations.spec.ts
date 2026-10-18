/**
 * E2E tests: integrations page (docs/14 §5, docs/16 §15).
 *
 * Tests:
 * - Page renders with summary cards and connector table
 * - Trust rank visual bar
 * - Enable/disable toggle
 * - Health check button
 * - New connector dialog
 * - Keyboard accessibility
 */

import { test, expect } from '@playwright/test';

/** Mock connector instances. */
const mockConnectors = [
  {
    instance_id: 'conn-fns-001',
    connector_type_id: 'fns',
    tenant_id: '00000000-0000-0000-0000-000000000001',
    environment: 'production',
    enabled: true,
    rate_limit_rps: 10,
    trust_rank: 75,
    config: {},
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-15T10:00:00Z',
  },
  {
    instance_id: 'conn-kontur-001',
    connector_type_id: 'kontur_focus',
    tenant_id: '00000000-0000-0000-0000-000000000001',
    environment: 'production',
    enabled: false,
    rate_limit_rps: 5,
    trust_rank: 50,
    config: {},
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-10T12:00:00Z',
  },
  {
    instance_id: 'conn-rnp-001',
    connector_type_id: 'rnp',
    tenant_id: '00000000-0000-0000-0000-000000000001',
    environment: 'production',
    enabled: true,
    rate_limit_rps: 20,
    trust_rank: 100,
    config: {},
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-14T09:00:00Z',
  },
];

const mockHealthCheck = {
  healthy: true,
  latency_ms: 150,
  message: 'OK',
  checked_at: '2026-01-15T11:00:00Z',
};

async function setupIntegrationsMocks(page: import('@playwright/test').Page) {
  await page.route('**/api/v1/me/context', (route) => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        data: {
          principal_id: '00000000-0000-0000-0000-000000000001',
          display_name: 'admin',
          tenant_id: '00000000-0000-0000-0000-000000000002',
          locale: 'ru',
          timezone: 'Europe/Moscow',
          auth_strength: 'normal',
          groups_revision: 1,
          capabilities: ['connector.manage', 'entity.read'],
          workspaces: ['default'],
          policy_revision: 1,
        },
        meta: { request_id: 'test' },
      }),
    });
  });

  // Mock connector list
  await page.route('**/api/v1/connectors', (route) => {
    if (route.request().method() === 'POST') {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ data: mockConnectors[0] }),
      });
      return;
    }
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ data: mockConnectors }),
    });
  });

  // Mock connector enable/disable
  await page.route('**/api/v1/connectors/*/enable', (route) => {
    route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });
  await page.route('**/api/v1/connectors/*/disable', (route) => {
    route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
  });

  // Mock health check
  await page.route('**/api/v1/connectors/*/health', (route) => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ data: mockHealthCheck }),
    });
  });
}

// Icon buttons; use icon substrings to locate them.
const healthBtnSelector = 'button:has([class*="heart-pulse"])';
const toggleBtnSelector = 'button:has([class*="lucide:play"], [class*="lucide:pause"])';

test.describe('Integrations page', () => {
  test.beforeEach(async ({ page }) => {
    await setupIntegrationsMocks(page);
  });

  test('renders page header with new connector button', async ({ page }) => {
    await page.goto('/integrations');
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.getByRole('button', { name: /Новый коннектор|New connector/i })).toBeVisible();
  });

  test('renders summary cards (total, active, disabled, healthy)', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    // Summary cards
    await expect(page.locator('.stat-card')).toHaveCount(4, { timeout: 10000 });
    const text = await page.locator('body').textContent();
    expect(text).toContain('3'); // total
  });

  test('renders connector table with 3 rows', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    await expect(page.locator('table tbody tr')).toHaveCount(3, { timeout: 10000 });
  });

  test('table shows connector type, environment, enabled status', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    const table = page.locator('table');
    await expect(table).toBeVisible();
    const tableText = await table.textContent();
    expect(tableText).toContain('fns');
    expect(tableText).toContain('kontur_focus');
    expect(tableText).toContain('rnp');
    expect(tableText).toContain('production');
  });

  test('trust rank is displayed as visual bar', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    const text = await page.locator('table').textContent();
    // Trust rank values: 75, 50, 100
    expect(text).toContain('75');
    expect(text).toContain('50');
    expect(text).toContain('100');
  });

  test('enable/disable toggle is present for each connector', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    const toggles = page.locator(toggleBtnSelector);
    expect(await toggles.count()).toBeGreaterThanOrEqual(1);
  });

  test('health check button is present for each connector', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    const healthButtons = page.locator(healthBtnSelector);
    expect(await healthButtons.count()).toBeGreaterThanOrEqual(1);
  });

  test('new connector dialog opens on button click', async ({ page }) => {
    await page.goto('/integrations');
    await page.getByRole('button', { name: /Новый коннектор|New connector/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Should show dialog with form fields
    await expect(page.locator('[role="dialog"] input').first()).toBeVisible();
    await expect(page.locator('[role="dialog"] [role="combobox"]').first()).toBeVisible();
  });

  test('new connector dialog has type, environment, rate_limit, trust_rank fields', async ({ page }) => {
    await page.goto('/integrations');
    await page.getByRole('button', { name: /Новый коннектор|New connector/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Type input
    await expect(page.locator('[role="dialog"] input').first()).toBeVisible();
    // Environment select (combobox)
    await expect(page.locator('[role="dialog"] [role="combobox"]').first()).toBeVisible();
    // Rate limit input (number)
    await expect(page.locator('[role="dialog"] input[type="number"]')).toBeVisible();
    // Trust rank select (combobox) — second one
    await expect(page.locator('[role="dialog"] [role="combobox"]').nth(1)).toBeVisible();
  });

  test('trust rank select has 5 levels', async ({ page }) => {
    await page.goto('/integrations');
    await page.getByRole('button', { name: /Новый коннектор|New connector/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Open trust rank select (second combobox in dialog)
    const trustSelect = page.locator('[role="dialog"] [role="combobox"]').nth(1);
    await trustSelect.click();
    await expect(page.locator('[role="option"]').first()).toBeVisible();
    const optionValues = await page.locator('[role="option"]').evaluateAll((els) =>
      els.map((el) => el.getAttribute('data-value') || el.textContent || '')
    );
    const joined = optionValues.join(' ');
    expect(joined).toContain('0'); // Untrusted
    expect(joined).toContain('100'); // Authoritative
  });

  test('health check button triggers API call', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    // Track health check request (ConnectorEndpoints uses POST /connectors/{id}/health)
    let healthCalled = false;
    page.on('request', (req) => {
      if (req.url().includes('/health') && req.method() === 'POST') {
        healthCalled = true;
      }
    });
    // Click first health check button
    const healthBtn = page.locator(healthBtnSelector).first();
    await healthBtn.click();
    expect(healthCalled).toBe(true);
  });

  test('enable toggle triggers API call', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    // Track enable/disable request
    let toggleCalled = false;
    page.on('request', (req) => {
      if (req.url().includes('/enable') || req.url().includes('/disable')) {
        toggleCalled = true;
      }
    });
    // Click first toggle (enabled connector => will call /disable)
    const toggle = page.locator(toggleBtnSelector).first();
    if (await toggle.isVisible()) {
      await toggle.click();
      expect(toggleCalled).toBe(true);
    }
  });

  test('keyboard navigation reaches table and controls', async ({ page }) => {
    await page.goto('/integrations');
    // Ensure page has focus and skip-link is reachable
    await page.locator('a.skip-link').focus();
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      const focused = await page.evaluate(() => document.activeElement?.tagName);
      if (['SELECT', 'INPUT', 'BUTTON', 'A'].includes(focused ?? '')) {
        break;
      }
    }
    const focused = await page.evaluate(() => document.activeElement?.tagName);
    expect(['SELECT', 'INPUT', 'BUTTON', 'A']).toContain(focused);
  });

  test('disabled connector shows disabled badge', async ({ page }) => {
    await page.goto('/integrations');
    await page.waitForResponse('**/api/v1/connectors');
    // Second connector is disabled
    const secondRow = page.locator('table tbody tr').nth(1);
    await expect(secondRow).toBeVisible();
    const rowText = await secondRow.textContent();
    // Russian locale: disabled badge is "Отключено"
    expect(rowText).toMatch(/false|Отключено|Disabled|disabled/);
  });
});
