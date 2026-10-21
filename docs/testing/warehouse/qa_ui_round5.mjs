/* QA round 5: verify UI fixes from deep review —
   1. enum label fallbacks (no raw i18n keys on checks/tasks tabs)
   2. locale-formatted timestamps (no raw ISO strings)
   3. keyboard-accessible table rows (audit, admin users)
   Run: node docs/testing/warehouse/qa_ui_round5.mjs */
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..', '..');
const { chromium } = await import(
  'file:///' + join(ROOT, 'nuxt/node_modules/playwright-core/index.mjs').replace(/\\/g, '/')
);

const BASE = 'http://localhost:3000';
const API = 'http://localhost:8010/api/v1';
const results = [];
const shot = (n) => `docs/testing/warehouse/qa_shots_round5/${n}.png`;
import { mkdirSync } from 'node:fs';
mkdirSync('docs/testing/warehouse/qa_shots_round5', { recursive: true });

function report(name, ok, detail = '') {
  results.push({ name, ok, detail });
  console.log(`${ok === true ? 'PASS' : ok === false ? 'FAIL' : 'CHECK'} ${name} ${detail}`);
}

async function login(page, user, pass) {
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await page.fill('input[name="username"], input[type="text"]', user);
  await page.fill('input[type="password"]', pass);
  await page.click('button[type="submit"]');
  await page.waitForURL((u) => !u.pathname.includes('login'), { timeout: 15000 });
}

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
const consoleErrors = [];
page.on('console', (m) => { if (m.type() === 'error') consoleErrors.push(m.text().slice(0, 160)); });

await login(page, 'admin', 'enginer-demo');

// QA admin api token from bootstrap
import { readFileSync } from 'node:fs';
const tok = JSON.parse(readFileSync('docs/testing/warehouse/.wh_tokens.json', 'utf8')).admin;

// --- 1. entity tabs: raw key check -------------------------------------
let pid = null;
if (tok) {
  const z = await fetch(`${API}/entities?type_id=wh_zone&limit=1`, { headers: { Authorization: `Bearer ${tok}` } })
    .then((r) => r.json());
  pid = z.data?.[0]?.public_id;
}
if (pid) {
  for (const tab of ['checks', 'tasks', 'risks', 'relations', 'history']) {
    await page.goto(`${BASE}/entities/${pid}?tab=${tab}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(800);
    const bodyText = await page.evaluate(() => document.body.innerText);
    const rawKeys = bodyText.match(/[a-z]+\.[a-z_]+\.[a-z_.]+/g)?.filter((k) => !k.includes(' ')) ?? [];
    const isoLeft = bodyText.match(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/g) ?? [];
    report(`tab ${tab}: no raw i18n keys`, rawKeys.length === 0, rawKeys.slice(0, 3).join(','));
    report(`tab ${tab}: no raw ISO timestamps`, isoLeft.length === 0, `${isoLeft.length} found`);
    await page.screenshot({ path: shot(`tab_${tab}`) });
  }
} else {
  report('entity tabs', 'skip', 'no wh_zone entity / no token');
}

// --- 2. audit keyboard row ---------------------------------------------
await page.goto(`${BASE}/audit`, { waitUntil: 'networkidle' });
await page.waitForTimeout(600);
const focusableRows = await page.evaluate(() =>
  document.querySelectorAll('tr[tabindex="0"], tr[role="button"]').length);
report('audit: keyboard-focusable rows', focusableRows > 0, `${focusableRows} rows`);
if (focusableRows > 0) {
  const row = page.locator('tr[tabindex="0"]').first();
  await row.focus();
  const before = await row.getAttribute('aria-expanded');
  await row.press('Enter');
  await page.waitForTimeout(400);
  const after = await row.getAttribute('aria-expanded');
  const detailRow = await page.locator('tr[tabindex="0"] ~ tr, tr:not([tabindex]) td[colspan]').count();
  report('audit: Enter toggles row', before !== after, `expanded ${before}->${after}`);
}
await page.screenshot({ path: shot('audit_keyboard') });

// --- 3. admin users keyboard -------------------------------------------
await page.goto(`${BASE}/admin/users`, { waitUntil: 'networkidle' });
await page.waitForTimeout(600);
const userRows = await page.evaluate(() =>
  document.querySelectorAll('tr[tabindex="0"], tr[role="button"]').length);
report('admin users: keyboard rows', userRows > 0, `${userRows} rows`);

// --- 4. NUXT_E1005 noise on denied deep link ----------------------------
consoleErrors.length = 0;
// risk_analyst token for a denied admin page
await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
await login(page, 'risk_analyst', 'enginer-demo');
await page.goto(`${BASE}/admin/users`, { waitUntil: 'networkidle' });
await page.waitForTimeout(800);
const e1005 = consoleErrors.filter((e) => e.includes('NUXT_E1005') || e.includes('E1005'));
const body403 = await page.evaluate(() => document.body.innerText);
report('denied deep link renders 403', /403|forbidden|доступ/i.test(body403), '');
report('no NUXT_E1005 noise', e1005.length === 0, `${e1005.length} console errors: ${e1005[0] ?? ''}`);
await page.screenshot({ path: shot('analyst_denied') });

// --- summary ------------------------------------------------------------
const fails = results.filter((r) => r.ok === false);
console.log(`\n${results.length - fails.length}/${results.length} ok, ${fails.length} failed`);
await browser.close();
process.exit(fails.length ? 1 : 0);
