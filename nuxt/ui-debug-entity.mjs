import { chromium } from 'playwright';
const BASE = 'http://localhost:3000';
async function main() {
  const browser = await chromium.launch({ headless: true });
  const page = await (await browser.newContext()).newPage();
  page.on('console', msg => console.log('CONSOLE:', msg.text()));
  page.on('pageerror', err => console.log('PAGEERROR:', err.message));
  page.on('requestfailed', req => console.log('REQFAIL:', req.url(), req.failure()?.errorText));

  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await page.fill('input[name="username"]', 'admin');
  await page.fill('input[name="password"]', 'admin');
  await page.click('button[type="submit"]');
  await page.waitForTimeout(2000);

  // Entity overview - legal entity
  await page.goto(`${BASE}/entities/ent-001`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(3000);

  const h3s = await page.locator('h3').allTextContents();
  console.log('H3s:', JSON.stringify(h3s));

  const dts = await page.locator('dt').allTextContents();
  console.log('DTs (labels):', JSON.stringify(dts));

  const dds = await page.locator('dd').allTextContents();
  console.log('DDs (values):', JSON.stringify(dds));

  // Check if "ООО Ромашка" or "ИНН" appears
  const bodyText = await page.textContent('body');
  console.log('Has "Ромашка":', bodyText?.includes('Ромашка'));
  console.log('Has "7701234567":', bodyText?.includes('7701234567'));
  console.log('Has "ИНН":', bodyText?.includes('ИНН'));
  console.log('Has "full_name":', bodyText?.includes('full_name'));

  await browser.close();
}
main().catch(console.error);
