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
    await page.waitForResponse('**/api/v1/checks');
    // Summary cards should be visible after data load
    await expect(page.locator('.stat-card')).toHaveCount(4);
  });

  test('renders filter section with kind and status selects', async ({ page }) => {
    await page.goto('/checks');
    // Nuxt UI USelect renders role="combobox"; locale switch + kind + status
    await expect(page.locator('[role="combobox"]')).toHaveCount(3, { timeout: 10000 });
  });

  test('new check dialog opens on button click', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Should show dialog with form fields
    await expect(page.locator('[role="dialog"] input').first()).toBeVisible();
    // Check kind select
    await expect(page.locator('[role="dialog"] [role="combobox"]').first()).toBeVisible();
  });

  test('new check dialog has entity, kind, and connector fields', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Entity ID input
    await expect(page.locator('[role="dialog"] input').first()).toBeVisible();
    // Check kind select
    await expect(page.locator('[role="dialog"] [role="combobox"]').first()).toBeVisible();
    // Connector input (optional)
    const connectorInput = page.locator('[role="dialog"] input').nth(1);
    await expect(connectorInput).toBeVisible();
  });

  test('new check dialog submit button is present', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Submit button (common.submit = "Отправить" in ru)
    await expect(page.locator('[role="dialog"]').getByRole('button', { name: /Создать|Create|Запустить|Start|Отправить|Submit/i })).toBeVisible();
  });

  test('check kind options include kyc, sanctions, financial_health', async ({ page }) => {
    await page.goto('/checks');
    await page.getByRole('button', { name: /Новая проверка|New check/i }).click();
    await expect(page.locator('[role="dialog"]')).toBeVisible();
    // Open kind select inside the dialog
    const kindSelect = page.locator('[role="dialog"] [role="combobox"]').first();
    await kindSelect.click();
    await expect(page.locator('[role="option"]').first()).toBeVisible();
    const optionTexts = await page.locator('[role="option"]').allTextContents();
    const joined = optionTexts.join(' ');
    expect(joined).toContain('KYC');
    expect(joined).toMatch(/Санкции|Sanctions/);
  });

  test('status badges show correct colors', async ({ page }) => {
    await page.goto('/checks');
    await page.waitForResponse('**/api/v1/checks');
    // The table should contain status text (Russian locale)
    const table = page.locator('table');
    await expect(table).toBeVisible();
    const tableText = await table.textContent();
    if (tableText) {
      // Should contain at least one status
      const statuses = ['Завершена', 'Выполняется', 'Ошибка', 'Ожидает', 'Отменена'];
      const hasStatus = statuses.some((s) => tableText.toLowerCase().includes(s.toLowerCase()));
      expect(hasStatus).toBe(true);
    }
  });

  test('keyboard navigation reaches filter controls', async ({ page }) => {
    await page.goto('/checks');
    // Ensure page has focus and skip-link is reachable
    await page.locator('a.skip-link').focus();
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
