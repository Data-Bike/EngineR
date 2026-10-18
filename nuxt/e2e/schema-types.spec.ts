import { test } from '@playwright/test';

async function login(page: import('@playwright/test').Page) {
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"], button:has-text("Войти"), button:has-text("Login")').first().click();
  await page.waitForTimeout(2500);
}

test('admin/types: create type via draft lifecycle', async ({ page }) => {
  page.on('response', r => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url()); });
  const alias = `e2e_type_${Date.now().toString(36)}`;
  await login(page);
  await page.goto('/admin/types');
  await page.waitForTimeout(3000);
  console.log('types visible:', await page.locator('p.font-medium').count());

  await page.locator('button:has-text("Создать тип"), button:has-text("type")').first().click();
  await page.waitForTimeout(800);
  const dialog = page.locator('[role="dialog"]');
  await dialog.locator('input').first().fill(alias);
  await dialog.locator('input').nth(1).fill('E2E тип');
  // add one field
  await dialog.locator('button:has-text("Добавить поле"), button:has-text("поле")').first().click().catch(() => {});
  await page.waitForTimeout(400);
  await dialog.locator('button:has-text("Сохранить"), button:has-text("Save")').last().click();
  await page.waitForTimeout(5000);
  const body = await page.locator('body').innerText();
  console.log('type present:', body.includes(alias));
  console.log('error toast:', body.match(/error|ошибка/gi)?.length ?? 0);
});
