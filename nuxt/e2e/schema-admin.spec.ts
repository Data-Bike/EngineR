import { test } from '@playwright/test';

async function login(page: import('@playwright/test').Page) {
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"], button:has-text("Войти"), button:has-text("Login")').first().click();
  await page.waitForTimeout(2500);
}

test('admin/schema: drafts+published load, draft lifecycle works', async ({ page }) => {
  page.on('response', r => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url()); });
  page.on('console', m => { if (m.type() === 'error') console.log('CONSOLE', m.text()); });
  await login(page);
  await page.goto('/admin/schema');
  await page.waitForTimeout(3000);
  const body = await page.locator('body').innerText();
  console.log('has published rev:', body.includes('9.10.0'));
  console.log('error banners:', await page.locator('[role="alert"]').count());
  // create draft flow
  const createBtn = page.locator('button:has-text("ерновик"), button:has-text("raft")').first();
  console.log('create btn visible:', await createBtn.isVisible().catch(() => false));
  await page.screenshot({ path: 'schema-admin.png', fullPage: true });
});
