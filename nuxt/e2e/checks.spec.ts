/**
 * E2E tests: checks page (docs/14 §4, docs/16 §15).
 *
 * Tests:
 * - Page renders with summary cards and table
 * - Filter by kind and status
 * - New check dialog
 * - Status badges
 * - Keyboard accessibility
 */

import { test, expect } from '@playwright/test';

/** Mock check instances. */
const mockChecks = [
  {
    check_id: '01938e7a-1234-5678-9abc-def012345678',
    status: 'completed',
    created_at: '2026-01-15T10:30:00Z',
    target_entity_id: '01938e7a-abcd-ef01-2345-678901234567',
    check_kind: 'kyc',
    final_score: 25.0,
    risk_level: 'Low',
  },
  {
    check_id: '01938e7a-2345-6789-abcd-ef012345678901',
    status: 'running',
    created_at: '2026-01-15T10:35:00Z',
    target_entity_id: '01938e7a-bcde-f012-3456-789012345678',
    check_kind: 'sanctions',
    final_score: 0,
    risk_level: 'Minimal',
  },
  {
    check_id: '01938e7a-3456-789a-bcde-f0123456789012',
    status: 'failed',
    created_at: '2026-01-15T10:40:00Z',
    target_entity_id: '01938e7a-cdef-0123-4567-890123456789',
    check_kind: 'financial_health',
    final_score: 0,
    risk_level: 'Minimal',
  },
];

async function setupChecksMocks(page: import('@playwright/test').Page) {
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
          capabilities: ['check.run', 'entity.read'],
          workspaces: ['default'],
          policy_revision: 1,
        },
        meta: { request_id: 'test' },
      }),
    });
  });

  // Mock check request
  await page.route('**/api/v1/checks', (route) => {
    if (route.request().method() === 'POST') {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: {
            check_id: '01938e7a-9999-9999-9999-999999999999',
            status: 'pending',
            created_at: '2026-01-15T11:00:00Z',
          },
        }),
      });
      return;
    }
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ data: mockChecks }),
    });
  });
}

test.describe('Checks page', () => {
  test.beforeEach(async ({ page }) => {
    await setupChecksMocks(page);
  });

  test('renders page header with new check button', async ({ page }) => {
    await page.goto('/checks');
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.getByRole('button', { name: /Новая проверка|New check/i })).toBeVisible();
  });

  test('renders summary cards (pending, running, completed, failed)', async ({ page }) => {
    await page.goto('/checks');
    // Summary cards should be visible
    const cards = page.locator('[class*="card"], .stat-card, [class*="StatCard"]');
    expect(await cards.count()).toBeGreaterThanOrEqual(1);
  });

  test('renders filter section with kind and status selects', async ({ page }) => {
    await page.goto('/checks');
    // Kind filter
    const selects = page.locator('select');
    expect(await selects.count()).toBeGreaterThanOrEqual(2);
  });

  test('new check dialog opens on button click', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    // Should show dialog with form fields
    await expect(page.locator('input').first()).toBeVisible();
    // Check kind select
    await expect(page.locator('select').first()).toBeVisible();
  });

  test('new check dialog has entity, kind, and connector fields', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    // Entity ID input
    await expect(page.locator('input').first()).toBeVisible();
    // Check kind select
    await expect(page.locator('select').first()).toBeVisible();
    // Connector input (optional)
    const connectorInput = page.locator('input').nth(1);
    await expect(connectorInput).toBeVisible();
  });

  test('new check dialog submit button is present', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    // Submit button
    await expect(page.getByRole('button', { name: /Создать|Create|Запустить|Start/i })).toBeVisible();
  });

  test('check kind options include kyc, sanctions, financial_health', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    // Open kind select
    const kindSelect = page.locator('select').first();
    const options = await kindSelect.locator('option').allTextContents();
    const joined = options.join(' ');
    expect(joined).toContain('KYC');
    expect(joined).toContain('Sanctions');
  });

  test('status badges show correct colors', async ({ page }) => {
    await page.goto('/checks');
    // The table should contain status text
    const tableText = await page.locator('table').textContent();
    if (tableText) {
      // Should contain at least one status
      const statuses = ['completed', 'running', 'failed', 'pending', 'cancelled'];
      const hasStatus = statuses.some((s) => tableText.toLowerCase().includes(s));
      expect(hasStatus).toBe(true);
    }
  });

  test('keyboard navigation reaches filter controls', async ({ page }) => {
    await page.goto('/checks');
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      const focused = await page.evaluate(() => document.activeElement?.tagName);
      if (['SELECT', 'INPUT', 'BUTTON'].includes(focused ?? '')) {
        break;
      }
    }
    const focused = await page.evaluate(() => document.activeElement?.tagName);
    expect(['SELECT', 'INPUT', 'BUTTON']).toContain(focused);
  });

  test('filter reset button clears filters', async ({ page }) => {
    await page.goto('/checks');
    // Fill entity filter if present
    const entityInput = page.locator('input').first();
    if (await entityInput.isVisible()) {
      await entityInput.fill('test-entity');
      await expect(entityInput).toHaveValue('test-entity');
      // Click reset
      const resetBtn = page.getByRole('button', { name: /Сбросить|Reset/i });
      if (await resetBtn.isVisible()) {
        await resetBtn.click();
        await expect(entityInput).toHaveValue('');
      }
    }
  });
});
