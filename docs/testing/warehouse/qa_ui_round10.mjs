// QA round 10 — error detail contract (API `detail` + UI rendering), nested
// i18n error codes, keyset pagination on /checks /requests /risks pages.
// Requires the stack up (web :3000, api :8010) and .wh_tokens.json.
import { mkdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dir = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dir, '..', '..', '..');
const { chromium } = await import(
  'file:///' + join(ROOT, 'nuxt/node_modules/playwright-core/index.mjs').replace(/\\/g, '/')
);

const WEB = 'http://localhost:3000';
const API = 'http://localhost:8010/api/v1';
const TOK = JSON.parse(readFileSync(join(__dir, '.wh_tokens.json'), 'utf-8'));
const token = TOK.admin || Object.values(TOK)[0];
const SHOTS = join(__dir, 'qa_shots_round10');
mkdirSync(SHOTS, { recursive: true });

const results = [];
const rec = (name, ok, note = '') => {
  results.push({ name, ok: Boolean(ok), note });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${note ? ` — ${note}` : ''}`);
};

async function api(method, path, body) {
  const r = await fetch(API + path, {
    method,
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  let j = null;
  try { j = await r.json(); } catch {}
  return { status: r.status, body: j };
}

const missing = await api('POST', '/entities', { type_id: 'legal_entity', fields: { name: `QA R10 ${Date.now()}` } });
rec('api: free-text 422 carries detail', missing.status === 422 && /required fields missing/.test(missing.body?.error?.detail ?? ''),
  JSON.stringify(missing.body?.error ?? {}));
const coded = await api('POST', '/requests', {
  target_entity_id: '00000000-0000-0000-0000-000000000000', requested_product: 'qa', due_date: '2026-10-21T12:00',
});
rec('api: machine-code 422 has no redundant detail', coded.status === 422 && coded.body?.error?.detail === undefined,
  JSON.stringify(coded.body?.error ?? {}));

for (const path of ['/checks', '/requests', '/assessments']) {
  const p1 = await api('GET', `${path}?limit=1`);
  const cursor = p1.body?.meta?.next_cursor;
  if (!cursor) {
    rec(`api: ${path} keyset (single page dataset)`, p1.status === 200, `rows=${p1.body?.data?.length}`);
    continue;
  }
  const p2 = await api('GET', `${path}?limit=1&cursor=${encodeURIComponent(cursor)}`);
  const a = JSON.stringify(p1.body.data[0]);
  const b = JSON.stringify(p2.body?.data?.[0]);
  rec(`api: ${path} second page differs from first`, p2.status === 200 && a !== b, `cursor=${String(cursor).slice(0, 8)}…`);
}

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e)));
const listCalls = [];
page.on('request', (r) => {
  const u = new URL(r.url());
  if (r.method() === 'GET' && /\/api\/v1\/(checks|requests|assessments)$/.test(u.pathname)) listCalls.push(u.pathname + u.search);
});

await page.goto(`${WEB}/login`, { waitUntil: 'networkidle' });
await page.fill('input[name="username"], input[type="text"]', 'admin');
await page.fill('input[type="password"]', 'enginer-demo');
await page.click('button[type="submit"]');
await page.waitForURL((u) => !u.pathname.includes('/login'), { timeout: 15000 });
await page.waitForLoadState('networkidle');

await page.goto(`${WEB}/objects/legal_entity/create?catalog=general`, { waitUntil: 'networkidle' });
await page.waitForSelector('form input', { timeout: 10000 }).catch(() => {});
await page.locator('form input').first().fill(`QA R10 ${Date.now()}`);
await page.locator('form button[type="submit"]').first().click();
await page.waitForTimeout(1500);
const createText = await page.locator('body').innerText();
rec('ui: non-field 422 shows translated label plus server detail',
  /Ошибка валидации: required fields missing/.test(createText),
  (createText.match(/Ошибка валидации[^\n]*/) ?? ['—'])[0]);
await page.screenshot({ path: join(SHOTS, 'create-422-detail.png') });
const leave = page.locator('[data-testid="leave-confirm"]');
await page.click('a[href="/objects"]').catch(() => {});
await page.waitForTimeout(600);
if (await leave.count()) await leave.click();

for (const [path, filterValue] of [['/checks', 'completed'], ['/requests', 'pending'], ['/risks', 'approved']]) {
  listCalls.length = 0;
  await page.goto(`${WEB}${path}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(500);
  const first = listCalls[0] ?? '';
  rec(`ui: ${path} renders and fetches the list`, first !== '', first);
  const select = page.locator('main [role="combobox"]', { hasText: /статус/i }).first();
  if (await select.count()) {
    await select.click();
    await page.locator('[role="option"]').nth(1).click().catch(() => {});
    await page.waitForTimeout(300);
    listCalls.length = 0;
    await page.locator('main button', { hasText: /^\s*(Поиск|Найти|Search)\s*$/ }).first().click().catch(() => {});
    await page.waitForTimeout(800);
    rec(`ui: ${path} status filter goes to the server`, listCalls.some((c) => c.includes('status=')), listCalls.join(' | ') || 'no call');
  } else {
    rec(`ui: ${path} status filter goes to the server`, false, 'status select not found');
  }
}

await browser.close();
const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail, pageErrors: ${pageErrors.length}`);
if (pageErrors.length) console.log(pageErrors.slice(0, 5).join('\n'));
process.exit(failed === 0 && pageErrors.length === 0 ? 0 : 1);
