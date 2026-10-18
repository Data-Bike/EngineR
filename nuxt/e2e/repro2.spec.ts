import { test } from '@playwright/test';

test('repro edit deep', async ({ page }) => {
  page.on('console', m => console.log('CONSOLE', m.type(), m.text().slice(0,300)));
  page.on('pageerror', e => console.log('PAGEERROR', String(e).slice(0,500)));
  page.on('requestfailed', r => console.log('REQFAIL', r.url(), r.failure()?.errorText));
  page.on('response', r => { const u=r.url(); if (u.includes('/api/v1')) console.log('API', r.status(), u.replace('http://localhost:8010','').replace('http://localhost:3000','')); });
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"]').first().click();
  await page.waitForURL('**/', { timeout: 8000 }).catch(()=>{});
  await page.waitForTimeout(1500);
  await page.goto('/entities/00000000-0000-7000-9000-000000000003/edit');
  await page.waitForTimeout(4000);
  console.log('FINAL URL', page.url());
});

test('repro edit count inputs', async ({ page }) => {
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"]').first().click();
  await page.waitForURL('**/', { timeout: 8000 }).catch(()=>{});
  await page.waitForTimeout(1500);
  await page.goto('/entities/00000000-0000-7000-9000-000000000003/edit');
  await page.waitForTimeout(4000);
  console.log('URL', page.url());
  console.log('inputs', await page.locator('form input').count());
  console.log('textareas', await page.locator('form textarea').count());
  const html = await page.locator('form').first().innerHTML().catch(() => 'NO FORM');
  console.log('FORM HTML head:', html.slice(0, 800));
  await page.screenshot({ path: 'repro-edit2.png', fullPage: true });
});
