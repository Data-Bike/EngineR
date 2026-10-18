import { test } from '@playwright/test';
test('admin/predicates loads', async ({ page }) => {
  page.on('response', r => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url()); });
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"]').first().click();
  await page.waitForTimeout(2500);
  await page.goto('/admin/predicates');
  await page.waitForTimeout(3000);
  const body = await page.locator('body').innerText();
  console.log('predicates listed:', (body.match(/full_name|works_for|owns/g) ?? []).length);
});
