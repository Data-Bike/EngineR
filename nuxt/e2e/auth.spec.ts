/**
 * E2E tests: login flow and auth redirect (docs/16 §15).
 */

import { test, expect } from '@playwright/test';

test.describe('Authentication', () => {
  test('redirects to login when unauthenticated', async ({ page }) => {
    await page.goto('/');
    // Should redirect to /login since we're not authenticated
    await expect(page).toHaveURL(/\/login/);
  });

  test('login page renders correctly', async ({ page }) => {
    await page.goto('/login');
    await expect(page.locator('h1')).toContainText(/Вход|Sign in/i);
    await expect(page.locator('input[name="username"], input')).toBeVisible();
    await expect(page.locator('input[type="password"]')).toBeVisible();
    await expect(page.locator('button[type="submit"]')).toBeVisible();
  });

  test('login page is keyboard accessible', async ({ page }) => {
    await page.goto('/login');
    // Tab through form elements
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(() => document.activeElement?.tagName);
    expect(['INPUT', 'BUTTON']).toContain(focused);
  });
});
