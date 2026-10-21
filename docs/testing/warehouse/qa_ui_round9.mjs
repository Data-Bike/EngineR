// QA round 9 — unsaved-changes guard semantics (markSaved), translated API
// errors (displayError), /api/v1/health alias, requests due_date contract.
// Requires the stack up (web :3000, api :8010) and .wh_tokens.json.
import { mkdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { randomUUID } from 'node:crypto';

const __dir = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dir, '..', '..', '..');
const { chromium } = await import(
  'file:///' + join(ROOT, 'nuxt/node_modules/playwright-core/index.mjs').replace(/\\/g, '/')
);

const WEB = 'http://localhost:3000';
const API = 'http://localhost:8010/api/v1';
const TOK = JSON.parse(readFileSync(join(__dir, '.wh_tokens.json'), 'utf-8'));
const token = TOK.admin || Object.values(TOK)[0];
const SHOTS = join(__dir, 'qa_shots_round9');
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

const tag = `QA R9 ${Date.now()}`;
const inn10 = (nine) => {
  const w = [2, 4, 10, 3, 5, 9, 4, 6, 8];
  return String(nine.split('').reduce((acc, d, i) => acc + Number(d) * w[i], 0) % 11 % 10);
};
const nine = String(Date.now()).slice(-9).padStart(9, '0');
const inn = nine + inn10(nine);
const created = await api('POST', '/entities', {
  type_id: 'legal_entity',
  fields: { name: tag, inn },
});
rec('api: QA legal_entity created', created.status === 200 || created.status === 201, `status=${created.status}`);
const pid = created.body?.data?.public_id;

const live = await fetch('http://localhost:8010/api/v1/health/live');
rec('health: /api/v1/health/live alias responds', live.status === 200, `status=${live.status}`);

const dateOk = await api('POST', '/requests', {
  target_entity_id: randomUUID(), requested_product: 'qa', urgency: 'normal', due_date: '2026-12-31',
});
rec('requests: YYYY-MM-DD due_date accepted (fails later on unknown target, not 422)', dateOk.status === 404, `status=${dateOk.status}`);
const dateBad = await api('POST', '/requests', {
  target_entity_id: randomUUID(), requested_product: 'qa', urgency: 'normal', due_date: '2026-10-21T12:00',
});
rec('requests: datetime-local due_date rejected with 422', dateBad.status === 422, `status=${dateBad.status}`);

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e)));

await page.goto(`${WEB}/login`, { waitUntil: 'networkidle' });
await page.fill('input[name="username"], input[type="text"]', 'admin');
await page.fill('input[type="password"]', 'enginer-demo');
await page.click('button[type="submit"]');
await page.waitForURL((u) => !u.pathname.includes('/login'), { timeout: 15000 });
await page.waitForLoadState('networkidle');

const leaveModal = () => page.locator('[data-testid="leave-confirm"]').count();
const leaveTo = async (path) => {
  await page.click(`a[href="${path}"]`).catch(() => {});
  await page.waitForTimeout(700);
};

await page.goto(`${WEB}/objects/legal_entity/create?catalog=general`, { waitUntil: 'networkidle' });
await page.waitForSelector('form input', { timeout: 10000 }).catch(() => {});
await leaveTo('/objects');
rec('guard: untouched create form leaves without modal', (await leaveModal()) === 0 && page.url().endsWith('/objects'), page.url());

if (pid) {
  await page.goto(`${WEB}/entities/${pid}/edit`, { waitUntil: 'networkidle' });
  await page.waitForSelector('form input', { timeout: 10000 }).catch(() => {});
  await leaveTo('/objects');
  rec('guard: untouched edit form leaves without modal', (await leaveModal()) === 0 && page.url().endsWith('/objects'), page.url());

  await page.goto(`${WEB}/entities/${pid}/edit`, { waitUntil: 'networkidle' });
  await page.waitForSelector('form input', { timeout: 10000 }).catch(() => {});
  await page.locator('form input').first().fill(`${tag} dirty`);
  const bump = await api('PUT', `/entities/${pid}`, { fields: [{ predicate_id: 'name', value: `${tag} bumped` }] });
  rec('api: concurrent revision bump lands (forces stale 409 on save)', bump.status === 200, `status=${bump.status}`);
  await page.locator('form button[type="submit"]').first().click();
  await page.waitForTimeout(1200);
  rec('edit: stale save stays on the form', page.url().includes('/edit'), page.url());
  await leaveTo('/objects');
  const held = (await leaveModal()) > 0;
  rec('guard: unsaved edits after a FAILED save are still guarded', held);
  if (held) {
    await page.screenshot({ path: join(SHOTS, 'guard-after-failed-save.png') });
    await page.locator('[data-testid="leave-cancel"]').click();
    await page.waitForTimeout(500);
    rec('guard: cancel keeps the edit form', page.url().includes('/edit'));
  }

  await page.goto(`${WEB}/entities/${pid}/edit`, { waitUntil: 'networkidle' });
  await page.waitForSelector('form input', { timeout: 10000 }).catch(() => {});
  await page.locator('form input').first().fill(`${tag} saved`);
  await page.locator('form button[type="submit"]').first().click();
  await page.waitForURL((u) => u.pathname === `/entities/${pid}`, { timeout: 15000 }).catch(() => {});
  rec('edit: successful save navigates to the entity card', page.url().endsWith(`/entities/${pid}`), page.url());
  rec('guard: successful save releases navigation (no modal)', (await leaveModal()) === 0);
}

await page.goto(`${WEB}/entities/00000000-0000-0000-0000-000000000000`, { waitUntil: 'networkidle' });
await page.waitForTimeout(800);
const body = await page.locator('body').innerText();
rec('displayError: unknown entity shows translated text, not raw server English',
  !body.includes('entity not found'), body.includes('Не найдено') ? 'shows Не найдено' : 'no translated text found');

await browser.close();

if (pid) {
  const retired = await api('POST', `/entities/${pid}/retire`, { reason: 'qa_round9 cleanup' });
  rec('cleanup: QA entity retired', retired.status === 200, `status=${retired.status}`);
}

const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail, pageErrors: ${pageErrors.length}`);
if (pageErrors.length) console.log(pageErrors.slice(0, 5).join('\n'));
process.exit(failed === 0 && pageErrors.length === 0 ? 0 : 1);
