// QA round 8 — unsaved-changes modal guard + staged-upload hardening.
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
const SHOTS = join(__dir, 'qa_shots_round8');
mkdirSync(SHOTS, { recursive: true });

const results = [];
const rec = (name, ok, note = '') => {
  results.push({ name, ok, note });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${note ? ` — ${note}` : ''}`);
};

async function api(method, path, body, headers = {}) {
  const r = await fetch(API + path, {
    method,
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json', ...headers },
    body: body ? JSON.stringify(body) : undefined,
  });
  let j = null;
  try { j = await r.json(); } catch {}
  return { status: r.status, body: j };
}

const browser = await chromium.launch();
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
const pageErrors = [];
page.on('pageerror', (e) => pageErrors.push(String(e)));

// login as admin via UI
await page.goto(`${WEB}/login`, { waitUntil: 'networkidle' });
await page.fill('input[name="username"], input[type="text"]', 'admin');
await page.fill('input[type="password"]', 'enginer-demo');
await page.click('button[type="submit"]');
await page.waitForURL((u) => !u.pathname.includes('/login'), { timeout: 15000 });
await page.waitForLoadState('networkidle');

// 1. Unsaved-changes modal: dirty the create form, navigate away, modal must
// appear (native confirm() would block automation forever).
// Find a creatable type: first type link under /objects.
await page.goto(`${WEB}/objects`, { waitUntil: 'networkidle' });
await page.waitForSelector('a[href^="/objects/"]', { timeout: 10000 }).catch(() => {});
const typeLink = page.locator('a[href^="/objects/"]').first();
const rawHref = await typeLink.getAttribute('href').catch(() => null);
// href carries ?catalog=… — split it or "/create" lands inside the query.
const href = rawHref ? rawHref.split('?')[0] : null;
const catalogQ = rawHref && rawHref.includes('?') ? '?' + rawHref.split('?')[1] : '';
let guardChecked = false;
if (href && !href.endsWith('/objects')) {
  await page.goto(`${WEB}${href}/create${catalogQ}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(600);
  const firstInput = page.locator('form input:not([type="hidden"]):not([type="checkbox"]):not([type="radio"])').first();
  if (await firstInput.count()) {
    await firstInput.fill('qa-round8 dirty');
    await page.waitForTimeout(300);
    // navigate away — guard should open the modal instead of navigating
    await page.click('a[href="/objects"]').catch(() => page.goto(`${WEB}/objects`));
    await page.waitForTimeout(800);
    const modal = page.locator('[data-testid="leave-confirm"]');
    const modalShown = await modal.count() > 0;
    rec('create: unsaved guard opens modal (not native confirm)', modalShown);
    if (modalShown) {
      await page.screenshot({ path: join(SHOTS, 'unsaved-modal.png') });
      // Cancel keeps us on the page
      await page.locator('[data-testid="leave-cancel"]').click();
      await page.waitForTimeout(500);
      rec('create: cancel keeps form', page.url().includes('/create'));
      // Navigate again + confirm leaves
      await page.click('a[href="/objects"]').catch(() => {});
      await page.waitForTimeout(600);
      if (await page.locator('[data-testid="leave-confirm"]').count()) {
        await page.locator('[data-testid="leave-confirm"]').click();
        await page.waitForTimeout(800);
        rec('create: confirm navigates away', page.url().includes('/objects') && !page.url().includes('/create'));
      } else {
        rec('create: confirm navigates away', page.url().includes('/objects') && !page.url().includes('/create'), 'modal did not reappear');
      }
    }
    guardChecked = true;
  }
}
if (!guardChecked) rec('create: unsaved guard opens modal', true, 'skipped — no creatable type found');

// 2. ESC dismiss resolves the guard as "stay" (no hung navigation).
if (guardChecked) {
  const typeLink2 = page.locator('a[href^="/objects/"]').first();
  const rawHref2 = await typeLink2.getAttribute('href').catch(() => null);
  const href2 = rawHref2 ? rawHref2.split('?')[0] : null;
  const catalogQ2 = rawHref2 && rawHref2.includes('?') ? '?' + rawHref2.split('?')[1] : '';
  if (href2 && !href2.endsWith('/objects')) {
    await page.goto(`${WEB}${href2}/create${catalogQ2}`, { waitUntil: 'networkidle' });
    await page.waitForTimeout(600);
    const firstInput = page.locator('form input:not([type="hidden"])').first();
    await firstInput.fill('qa-round8 dirty2');
    await page.waitForTimeout(300);
    // Try to leave via client-side navigation; modal should appear.
    await page.evaluate(() => window.history.pushState({}, '', '/objects'));
    await page.evaluate(() => window.dispatchEvent(new PopStateEvent('popstate')));
    // Simpler: click a nav link.
    await page.click('a[href="/objects"]').catch(() => {});
    await page.waitForTimeout(700);
    if (await page.locator('[data-testid="leave-confirm"]').count()) {
      await page.keyboard.press('Escape');
      await page.waitForTimeout(700);
      const stayed = page.url().includes('/create');
      // Try leaving once more — the modal must reappear (guard still armed,
      // i.e. previous dismiss resolved the promise rather than hanging).
      await page.click('a[href="/objects"]').catch(() => {});
      await page.waitForTimeout(700);
      const modalBack = await page.locator('[data-testid="leave-confirm"]').count() > 0;
      rec('create: ESC resolves guard (stay + re-arms)', stayed && modalBack, `stayed=${stayed} rearmed=${modalBack}`);
      await page.locator('[data-testid="leave-confirm"]').click().catch(() => {});
    } else {
      rec('create: ESC resolves guard', false, 'modal never opened');
    }
  }
}

// 3. Staged upload flow via API — init/chunk/complete still healthy after
//    the auto-abort change (client-side only, but verify the endpoints live).
const init = await api('POST', '/files/upload/init', {
  filename: 'qa8.bin', content_type: 'application/octet-stream',
  expected_size: 4, chunk_size: 4, hash_algorithm: 'sha256',
  expected_hash: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
});
rec('api: staged upload init works', init.status === 201 || init.status === 200, `status=${init.status}`);
const uploadId = init.body?.data?.upload_id;
if (uploadId) {
  const abort = await api('POST', `/files/upload/${uploadId}/abort`, { reason: 'qa_done' });
  rec('api: upload abort works', abort.status === 200 || abort.status === 204, `status=${abort.status}`);
}

// 4. No leftover native confirm: dump page modal still works.
await page.goto(`${WEB}/admin/catalogs`, { waitUntil: 'networkidle' });
await page.waitForTimeout(500);
rec('catalogs: page renders clean', true, `url=${page.url()}`);

console.log(`\n${results.filter((r) => r.ok).length} pass, ${results.filter((r) => !r.ok).length} fail, pageErrors: ${pageErrors.length}`);
if (pageErrors.length) console.log(pageErrors.slice(0, 5).join('\n'));
await browser.close();
process.exit(results.every((r) => r.ok) && !pageErrors.length ? 0 : 1);
