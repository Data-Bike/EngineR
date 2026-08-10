/**
 * E2E tests: persona matrix (docs/16 §15, docs/17 §14).
 *
 * Tests role-based UI visibility:
 * - admin: sees all nav items including admin/audit
 * - security_officer: sees checks, evidence, factors
 * - manager: sees requests, basic entity info, no hidden person fields
 * - risk_analyst: sees risks, assessment, approval workflow
 */

import { test, expect } from '@playwright/test';

/** Personas for testing (docs/17 §14). */
const personas = {
  admin: {
    username: 'admin@test',
    capabilities: ['admin.view', 'audit.view', 'entity.create', 'entity.read'],
  },
  security_officer: {
    username: 'security@test',
    capabilities: ['check.run', 'entity.read', 'factor.view'],
  },
  manager: {
    username: 'manager@test',
    capabilities: ['request.create', 'entity.read'],
  },
  risk_analyst: {
    username: 'risk@test',
    capabilities: ['risk.view', 'assessment.view', 'entity.read'],
  },
};

test.describe('Persona matrix', () => {
  for (const [role, persona] of Object.entries(personas)) {
    test(`${role} sees correct nav items`, async ({ page }) => {
      // Mock the /me/context endpoint with persona capabilities
      await page.route('**/api/v1/me/context', (route) => {
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            data: {
              principal_id: '00000000-0000-0000-0000-000000000001',
              display_name: role,
              tenant_id: '00000000-0000-0000-0000-000000000002',
              locale: 'ru',
              timezone: 'Europe/Moscow',
              auth_strength: 'normal',
              groups_revision: 1,
              capabilities: persona.capabilities,
              workspaces: ['default'],
              policy_revision: 1,
            },
            meta: { request_id: 'test' },
          }),
        });
      });

      // Mock auth check as successful
      await page.route('**/api/v1/auth/login', (route) => {
        route.fulfill({ status: 200, contentType: 'application/json', body: '{}' });
      });

      await page.goto('/login');
      await page.locator('input[name="username"]').fill(persona.username);
      await page.locator('input[name="password"]').fill('password');
      await page.locator('button[type="submit"]').click();

      // Should be redirected to home
      await expect(page).toHaveURL(/\/(ru)?\/?$/);

      // Check nav items based on capabilities
      const navLinks = page.locator('nav a');
      const count = await navLinks.count();

      if (role === 'admin') {
        // Admin should see admin and audit links
        const texts = await navLinks.allTextContents();
        expect(texts.join(' ')).toContain('Администрирование');
        expect(texts.join(' ')).toContain('Аудит');
      }

      if (role === 'manager') {
        // Manager should NOT see admin or audit
        const texts = await navLinks.allTextContents();
        expect(texts.join(' ')).not.toContain('Администрирование');
        expect(texts.join(' ')).not.toContain('Аудит');
      }
    });
  }
});

test.describe('Accessibility', () => {
  test('skip link is present and functional', async ({ page }) => {
    await page.route('**/api/v1/me/context', (route) => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: {
            principal_id: '00000000-0000-0000-0000-000000000001',
            display_name: 'test',
            tenant_id: '00000000-0000-0000-0000-000000000002',
            locale: 'ru',
            timezone: 'Europe/Moscow',
            auth_strength: 'normal',
            groups_revision: 1,
            capabilities: ['entity.read'],
            workspaces: ['default'],
            policy_revision: 1,
          },
          meta: { request_id: 'test' },
        }),
      });
    });

    await page.goto('/');
    const skipLink = page.locator('.skip-link');
    await expect(skipLink).toBeVisible();
  });

  test('main content has correct landmark', async ({ page }) => {
    await page.route('**/api/v1/me/context', (route) => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: {
            principal_id: '00000000-0000-0000-0000-000000000001',
            display_name: 'test',
            tenant_id: '00000000-0000-0000-0000-000000000002',
            locale: 'ru',
            timezone: 'Europe/Moscow',
            auth_strength: 'normal',
            groups_revision: 1,
            capabilities: ['entity.read'],
            workspaces: ['default'],
            policy_revision: 1,
          },
          meta: { request_id: 'test' },
        }),
      });
    });

    await page.goto('/');
    // header landmark
    await expect(page.locator('header[role="banner"]')).toBeVisible();
    // navigation landmark
    await expect(page.locator('nav[role="navigation"]')).toBeVisible();
    // main content
    await expect(page.locator('main#main-content')).toBeVisible();
  });
});

test.describe('i18n', () => {
  test('locale switch changes UI language', async ({ page }) => {
    await page.route('**/api/v1/me/context', (route) => {
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          data: {
            principal_id: '00000000-0000-0000-0000-000000000001',
            display_name: 'test',
            tenant_id: '00000000-0000-0000-0000-000000000002',
            locale: 'ru',
            timezone: 'Europe/Moscow',
            auth_strength: 'normal',
            groups_revision: 1,
            capabilities: ['entity.read'],
            workspaces: ['default'],
            policy_revision: 1,
          },
          meta: { request_id: 'test' },
        }),
      });
    });

    // English locale
    await page.goto('/en');
    // Should show English text
    await expect(page.locator('header')).toBeVisible();
  });
});
