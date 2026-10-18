import { test, expect } from '@playwright/test';

test('repro: edit page fields state', async ({ page }) => {
  page.on('console', m => { if (m.type() === 'error' || m.type() === 'warning') console.log('CONSOLE', m.type(), m.text()); });
  page.on('response', r => { if (r.status() >= 400) console.log('HTTP', r.status(), r.url()); });
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"], button:has-text("Войти"), button:has-text("Login")').first().click();
  await page.waitForTimeout(2500);
  console.log('after login url:', page.url());
  await page.goto('/entities/00000000-0000-7000-9000-000000000003/edit');
  await page.waitForTimeout(3000);
  const inputs = await page.locator('form input, form textarea, form [role="switch"], form [role="combobox"]').all();
  console.log('form inputs:', inputs.length);
  for (const i of inputs) {
    console.log('  input disabled=', await i.isDisabled(), 'placeholder=', await i.getAttribute('placeholder'), 'value=', (await i.inputValue().catch(()=>'')));
  }
  await page.screenshot({ path: 'repro-edit.png', fullPage: true });
});

test('repro: create page dedup + request target', async ({ page }) => {
  await page.goto('/login');
  await page.locator('input').first().fill('admin');
  await page.locator('input[type="password"]').fill('enginer-demo');
  await page.locator('button[type="submit"], button:has-text("Войти")').first().click();
  await page.waitForTimeout(2500);
  // request page: target autocomplete
  await page.goto('/requests');
  await page.waitForTimeout(2000);
  const btns = await page.locator('button').allTextContents();
  console.log('request page buttons:', JSON.stringify(btns.slice(0,15)));
});
