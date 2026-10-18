import { test } from '@playwright/test';

async function login(page: any) {
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"]').first().click();
  await page.waitForURL('**/', { timeout: 8000 }).catch(()=>{});
  await page.waitForTimeout(1500);
}

test('repro: list + advanced search labels', async ({ page }) => {
  page.on('response', r => { const u=r.url(); if (u.includes('/api/v1')) console.log('API', r.status(), u.split('/api/v1')[1]); });
  await login(page);
  await page.goto('/objects/legal_entity');
  await page.waitForTimeout(3000);
  await page.locator('button:has-text("Расширенный"), button:has-text("Advanced")').first().click().catch(()=>{});
  await page.waitForTimeout(500);
  const labels = await page.locator('label').allTextContents();
  console.log('LABELS:', JSON.stringify(labels));
  await page.screenshot({ path: 'repro-list.png', fullPage: true });
});

test('repro: create → dedup dialog', async ({ page }) => {
  page.on('response', r => { const u=r.url(); if (u.includes('/api/v1')) console.log('API', r.status(), u.split('/api/v1')[1]); });
  await login(page);
  await page.goto('/objects/legal_entity/create');
  await page.waitForTimeout(2500);
  console.log('create inputs:', await page.locator('form input').count());
  // fill first two text inputs (name, inn)
  const inputs = page.locator('form input');
  await inputs.nth(0).fill('ООО «Тест-Новый»');
  await inputs.nth(2).fill('9998887776');
  await page.locator('button[type="submit"]').first().click();
  await page.waitForTimeout(2500);
  const dlg = await page.locator('[role="dialog"], .fixed').allTextContents();
  console.log('DIALOG:', JSON.stringify(dlg).slice(0, 800));
  await page.screenshot({ path: 'repro-create.png', fullPage: true });
});

test('repro: request create target field', async ({ page }) => {
  await login(page);
  await page.goto('/requests');
  await page.waitForTimeout(2000);
  await page.locator('button:has-text("Новая заявка"), button:has-text("New")').first().click();
  await page.waitForTimeout(800);
  const combo = page.locator('[role="dialog"] input, [role="dialog"] [role="combobox"] input').first();
  await combo.fill('ром');
  await page.waitForTimeout(1500);
  const items = await page.locator('[role="option"], [role="listbox"] *').allTextContents();
  console.log('OPTIONS:', JSON.stringify(items.slice(0,10)));
  await page.screenshot({ path: 'repro-request.png', fullPage: true });
});
