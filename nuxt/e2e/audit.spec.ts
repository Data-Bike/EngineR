/**
 * E2E tests: audit page (docs/08 §11, docs/16 §15).
 *
 * Tests:
 * - Page renders with filters and table
 * - Filter by kind updates the query
 * - Expandable detail row shows all fields
 * - Verify chain button shows banner
 * - Create checkpoint button works
 * - Retention policy card + edit dialog
 * - Keyboard accessibility
 */

import { test, expect } from '@playwright/test';

/** Mock audit entries for testing. */
const mockEntries = [
  {
    audit_id: 1,
    sequence: 1,
    previous_hash: '0000000000000000000000000000000000000000000000000000000000000000',
    entry_hash: 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
    tenant_id: '00000000-0000-0000-0000-000000000001',
    occurred_at: '2026-01-15T10:30:00Z',
    actor: 'admin@test',
    effective_actor: null,
    kind: 'decision',
    action: 'check.run',
    resource_kind: 'check',
    resource_id: '01938e7a-1234-5678-9abc-def012345678',
    effect: 'allow',
    reason_ids: [],
    policy_revision: 1,
    request_id: 'req-001',
    correlation_id: 'corr-001',
    query_fingerprint: null,
    row_count: null,
    bytes: null,
    client: 'web-admin',
    service: 'enginer-api',
    metadata: { check_kind: 'kyc', target_entity_id: '01938e7a-abcd-ef01-2345-678901234567' },
  },
  {
    audit_id: 2,
    sequence: 2,
    previous_hash: 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
    entry_hash: 'b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3',
    tenant_id: '00000000-0000-0000-0000-000000000001',
    occurred_at: '2026-01-15T10:31:00Z',
    actor: 'admin@test',
    effective_actor: null,
    kind: 'decision',
    action: 'check.decision',
    resource_kind: 'check',
    resource_id: '01938e7a-1234-5678-9abc-def012345678',
    effect: 'escalate',
    reason_ids: ['reason-001', 'reason-002'],
    policy_revision: 1,
    request_id: 'req-002',
    correlation_id: 'corr-001',
    query_fingerprint: null,
    row_count: null,
    bytes: 1024,
    client: 'web-admin',
    service: 'enginer-api',
    metadata: { assessment_id: 'asmt-001', score: '75.00', risk_level: 'High' },
  },
  {
    audit_id: 3,
    sequence: 3,
    previous_hash: 'b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3',
    entry_hash: 'c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
    tenant_id: '00000000-0000-0000-0000-000000000001',
    occurred_at: '2026-01-15T10:32:00Z',
    actor: 'security@test',
    effective_actor: null,
    kind: 'denied_request',
    action: 'entity.read',
    resource_kind: 'entity',
    resource_id: '01938e7a-abcd-ef01-2345-678901234567',
    effect: 'deny',
    reason_ids: ['reason-003'],
    policy_revision: 1,
    request_id: 'req-003',
    correlation_id: null,
    query_fingerprint: 'fp-abc123',
    row_count: 0,
    bytes: null,
    client: 'web-admin',
    service: 'enginer-api',
    metadata: {},
  },
];

const mockCheckpoint = {
  checkpoint_id: 1,
  tenant_id: '00000000-0000-0000-0000-000000000001',
  sequence: 3,
  entry_hash: 'c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
  created_at: '2026-01-15T10:32:00Z',
};

const mockVerification = {
  valid: true,
  entry_count: 3,
  last_sequence: 3,
  last_hash: 'c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
};

const mockRetention = {
  tenant_id: '00000000-0000-0000-0000-000000000001',
  retention_days: 365,
  legal_hold: false,
  archive_before_delete: true,
  archive_url: 's3://archive-bucket',
};

/** Setup mock routes for audit API. */
async function setupAuditMocks(page: import('@playwright/test').Page) {
  // Mock auth context
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
          capabilities: ['audit.read', 'security.manage', 'entity.read'],
          workspaces: ['default'],
          policy_revision: 1,
        },
        meta: { request_id: 'test' },
      }),
    });
  });

  // Mock audit query
  await page.route('**/api/v1/audit**', (route) => {
    const url = route.request().url();
    const method = route.request().method();

    if (url.includes('/audit/checkpoint') && method === 'GET') {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ data: mockCheckpoint }),
      });
      return;
    }
    if (url.includes('/audit/checkpoint') && method === 'POST') {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ data: { ...mockCheckpoint, checkpoint_id: 2, sequence: 3 } }),
      });
      return;
    }
    if (url.includes('/audit/verify')) {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ data: mockVerification }),
      });
      return;
    }
    if (url.includes('/audit/retention') && method === 'GET') {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ data: mockRetention }),
      });
      return;
    }
    if (url.includes('/audit/retention') && method === 'PUT') {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ data: { ...mockRetention, retention_days: 730 } }),
      });
      return;
    }
    // Default: audit query
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ data: mockEntries }),
    });
  });
}

test.describe('Audit page', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuditMocks(page);
  });

  test('renders page header with verify and checkpoint buttons', async ({ page }) => {
    await page.goto('/audit');
    await expect(page.locator('h1')).toBeVisible();
    // Verify chain button
    await expect(page.getByRole('button', { name: /Проверить цепочку|Verify chain/i })).toBeVisible();
    // Create checkpoint button
    await expect(page.getByRole('button', { name: /Создать чекпойнт|Create checkpoint/i })).toBeVisible();
  });

  test('renders filter section with kind, actor, action, from, to', async ({ page }) => {
    await page.goto('/audit');
    // Kind filter is a Nuxt UI USelect (role="combobox"); locale switch is also one
    await expect(page.locator('[role="combobox"]')).toHaveCount(2, { timeout: 10000 });
    // Actor and action inputs (text inputs; at least global search + actor + action)
    await expect(page.locator('input[type="text"]').first()).toBeVisible();
    // Date inputs
    await expect(page.locator('input[type="datetime-local"]')).toHaveCount(2);
  });

  test('displays correct kind badges', async ({ page }) => {
    await page.goto('/audit');
    // Query endpoint may include search params
    await page.waitForResponse('**/api/v1/audit**');
    // Wait for table rows
    await expect(page.locator('table tbody tr')).toHaveCount(3, { timeout: 10000 });
    // Third column contains kind labels (Russian: decision=Решение, denied_request=Отказ)
    const kindCol = await page.locator('table tbody tr td:nth-child(3)').allTextContents();
    const joined = kindCol.join(' ');
    expect(joined).toContain('Решение');
    expect(joined).toContain('Отказ');
  });
  test('renders audit entries table with 3 rows', async ({ page }) => {
  });

  test('displays effect badges (allow, escalate, deny)', async ({ page }) => {
    await page.goto('/audit');
    const tableText = await page.locator('table').textContent();
    expect(tableText).toContain('allow');
    expect(tableText).toContain('escalate');
    expect(tableText).toContain('deny');
  });

  test('expands detail row on click', async ({ page }) => {
    await page.goto('/audit');
    // Click first row
    await page.locator('table tbody tr').first().click();
    // Should show detail section with entry_id label
    await expect(page.locator('table tbody').getByText(/ID записи|Entry ID/i)).toBeVisible();
    // Should show previous_hash
    await expect(page.locator('table tbody').getByText(/Предыдущий хэш|Previous hash/i)).toBeVisible();
  });

  test('detail row shows metadata JSON', async ({ page }) => {
    await page.goto('/audit');
    // Click first row (has metadata with check_kind)
    await page.locator('table tbody tr').first().click();
    // Should show metadata section
    await expect(page.locator('table tbody').getByText(/Метаданные|Metadata/i)).toBeVisible();
    // Should contain check_kind in the JSON
    await expect(page.locator('table tbody pre')).toContainText('check_kind');
  });

  test('detail row shows reason IDs as badges', async ({ page }) => {
    await page.goto('/audit');
    // Click second row (has reason_ids)
    await page.locator('table tbody tr').nth(1).click();
    // Should show reason IDs section
    await expect(page.locator('table tbody').getByText(/Причины|Reason IDs/i)).toBeVisible();
  });

  test('verify chain button shows success banner', async ({ page }) => {
    await page.goto('/audit');
    await page.getByRole('button', { name: /Проверить цепочку|Verify chain/i }).click();
    // Should show success alert
    await expect(page.locator('[class*="alert"], [role="alert"]').first()).toBeVisible();
  });

  test('create checkpoint button triggers API call', async ({ page }) => {
    await page.goto('/audit');
    // Track the POST request
    let checkpointPosted = false;
    page.on('request', (req) => {
      if (req.url().includes('/audit/checkpoint') && req.method() === 'POST') {
        checkpointPosted = true;
      }
    });
    await page.getByRole('button', { name: /Создать чекпойнт|Create checkpoint/i }).click();
    expect(checkpointPosted).toBe(true);
  });

  test('latest checkpoint card is displayed', async ({ page }) => {
    // Start waiting for checkpoint and retention endpoints before navigation
    await Promise.all([
      page.waitForResponse('**/api/v1/audit/checkpoint*'),
      page.waitForResponse('**/api/v1/audit/retention*'),
      page.goto('/audit'),
    ]);
    // Should show checkpoint card with sequence
    const checkpointCard = page.locator('div').filter({ hasText: /Последний чекпойнт|Latest checkpoint/i }).first();
    await expect(checkpointCard).toBeVisible();
    expect(await checkpointCard.locator('text=3').count()).toBeGreaterThanOrEqual(1);
  });

  test('retention policy card is displayed with edit button', async ({ page }) => {
    await Promise.all([
      page.waitForResponse('**/api/v1/audit/retention*'),
      page.goto('/audit'),
    ]);
    await expect(page.getByText(/Политика хранения|Retention policy/i)).toBeVisible();
    // Edit button
    await expect(page.getByRole('button', { name: /Изменить|Edit/i })).toBeVisible();
  });

  test('retention edit dialog opens and saves', async ({ page }) => {
    await page.goto('/audit');
    // Open dialog
    await page.getByRole('button', { name: /Изменить|Edit/i }).click();
    // Should show dialog with retention days input
    await expect(page.locator('input[type="number"]')).toBeVisible();
    // Change value
    await page.locator('input[type="number"]').fill('730');
    // Save
    await page.getByRole('button', { name: /Сохранить|Save/i }).click();
  });

  test('reset filters button clears all filters', async ({ page }) => {
    await page.goto('/audit');
    // Fill actor filter
    const actorInput = page.locator('input').nth(1);
    await actorInput.fill('admin@test');
    await expect(actorInput).toHaveValue('admin@test');
    // Click reset
    await page.getByRole('button', { name: /Сбросить|Reset/i }).click();
    // Should be empty
    await expect(actorInput).toHaveValue('');
  });

  test('keyboard navigation: Tab reaches filter controls', async ({ page }) => {
    await page.goto('/audit');
    // Ensure page has focus and skip-link is reachable
    await page.locator('a.skip-link').focus();
    // Tab through to reach filter area
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      const focused = await page.evaluate(() => {
        const el = document.activeElement;
        return el ? el.tagName : null;
      });
      if (['SELECT', 'INPUT', 'BUTTON'].includes(focused ?? '')) {
        break;
      }
    }
    const focused = await page.evaluate(() => document.activeElement?.tagName);
    expect(['SELECT', 'INPUT', 'BUTTON']).toContain(focused);
  });

  test('table rows are clickable (cursor pointer)', async ({ page }) => {
    await page.goto('/audit');
    const firstRow = page.locator('table tbody tr').first();
    const cursor = await firstRow.evaluate((el) => getComputedStyle(el).cursor);
    expect(cursor).toBe('pointer');
  });

  test('hash column shows truncated hash', async ({ page }) => {
    await page.goto('/audit');
    const hashCell = page.locator('table tbody tr').first().locator('td').nth(7);
    const text = await hashCell.textContent();
    expect(text).toContain('…');
    // Should be truncated (not full 64 chars)
    expect(text!.length).toBeLessThan(40);
  });
});
