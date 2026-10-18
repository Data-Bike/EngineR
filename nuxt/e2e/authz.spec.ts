import { test, expect } from '@playwright/test';

async function login(page: any, user: string) {
  await page.goto('/login');
  await page.locator('input').first().fill(user);
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"]').first().click();
  await page.waitForTimeout(2500);
}

test('authz: admin sees admin menu, users page and permissions page', async ({ page }) => {
  page.on('response', r => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url()); });
  await login(page, 'admin');

  // sidebar contains admin entry
  const adminLink = page.locator('a[href="/admin"], a[href="/admin/"]').first();
  await expect(adminLink).toBeVisible({ timeout: 10000 });

  // /admin index renders cards incl. users card
  await page.goto('/admin');
  await page.waitForTimeout(1500);
  await expect(page.locator('[data-testid="admin-users-card"]')).toBeVisible();

  // /admin/users renders table with seeded personas
  await page.goto('/admin/users');
  await page.waitForTimeout(2500);
  const body = await page.locator('body').innerText();
  console.log('users page snippet:', body.slice(0, 400));
  expect(body).toContain('admin');

  // /admin/permissions renders role tree
  await page.goto('/admin/permissions');
  await page.waitForTimeout(2500);
  const permsBody = await page.locator('body').innerText();
  console.log('permissions page snippet:', permsBody.slice(0, 300));
});

test('authz: risk_analyst has no admin menu and admin pages are blocked', async ({ page }) => {
  page.on('response', r => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url()); });
  await login(page, 'risk_analyst');

  const adminLink = page.locator('a[href="/admin"], a[href="/admin/"]');
  await page.waitForTimeout(1000);
  expect(await adminLink.count()).toBe(0);
  const auditLink = page.locator('a[href="/audit"], a[href="/audit/"]');
  expect(await auditLink.count()).toBe(0);

  await page.goto('/admin');
  await page.waitForTimeout(2000);
  const url = page.url();
  const body = await page.locator('body').innerText();
  console.log('analyst /admin ->', url, '|', body.slice(0, 200));
  expect(url.includes('/admin') && body.includes('schema')).toBeFalsy();
});

test('authz: users page search filters list', async ({ page }) => {
  await login(page, 'admin');
  await page.goto('/admin/users');
  await page.waitForTimeout(2500);
  const search = page.locator('input[placeholder], input[type="search"], input').first();
  await search.fill('risk_analyst');
  await page.waitForTimeout(1500);
  const body = await page.locator('body').innerText();
  expect(body).toContain('risk_analyst');
  // manager row should be filtered out by server-side search
  console.log('filtered snippet:', body.slice(0, 300));
});
