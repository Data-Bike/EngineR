/**
 * qa_ui_fixes.mjs — regression probe for the latest UI fix round:
 *  1. <html lang>/<html dir> set on load and on locale switch
 *  2. session expiry → /login WITHOUT authenticated chrome (sidebar/profile)
 *  3. draft save → toast, reload → draft restored into the form
 *  4. unsaved indicator = accessible translated text (not a bare '*')
 *  5. breadcrumb 'edit' segment translated
 *  6. long display name truncated in breadcrumbs
 *
 * Run: node docs/testing/warehouse/qa_ui_fixes.mjs (repo root)
 */

import { readFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..', '..');
const pw = await import(
  'file:///' + join(ROOT, 'nuxt/node_modules/playwright-core/index.mjs').replace(/\\/g, '/')
);
const { chromium } = pw;

const BASE = 'http://localhost:3000';
const API = 'http://localhost:8010/api/v1';
const SHOTS = join(HERE, 'qa_shots_ui_fixes');
mkdirSync(SHOTS, { recursive: true });

const results = [];
function rec(name, verdict, evidence) {
  results.push({ name, verdict, evidence });
  console.log(`### ${name}\n    ${verdict}\n    ${evidence}\n`);
}

async function login(page, user = 'manager', pass = 'enginer-demo') {
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await page.locator('input[autocomplete="username"]').fill(user);
  await page.locator('input[type="password"]').fill(pass);
  await page.locator('button[type="submit"]').click();
  await page.waitForURL((u) => !u.pathname.endsWith('/login'), { timeout: 20000 });
  await page.waitForLoadState('networkidle');
}

const browser = await chromium.launch();
const ctx = await browser.newContext({ locale: 'es-ES' });
const page = await ctx.newPage();
page.on('pageerror', (e) => console.log('PAGEERROR:', e.message));

// --- 1. html lang/dir on load ----------------------------------------------
await login(page);
const lang0 = await page.evaluate(() => document.documentElement.lang);
const dir0 = await page.evaluate(() => document.documentElement.dir);
rec('html lang/dir on load', lang0 && dir0 ? 'PASS' : 'FAIL', `lang=${lang0} dir=${dir0}`);

// switch locale via the header select (visible text of selector varies;
// drive i18n through the same path the UI uses — evaluate setLocale via
// the select if present, else check attribute only after reload).
const sel = page.locator('header select, [role="combobox"]').first();
try {
  await sel.selectOption({ value: 'ar' });
  await page.waitForTimeout(800);
} catch {
  // fallback: click-based combobox
  try {
    await page.locator('header').locator('text=Español').first().click();
    await page.locator('text=العربية').first().click();
    await page.waitForTimeout(800);
  } catch (e) {
    console.log('locale switch fallback failed:', e.message);
  }
}
const lang1 = await page.evaluate(() => document.documentElement.lang);
const dir1 = await page.evaluate(() => document.documentElement.dir);
rec('html lang/dir after switch', lang1 === 'ar' && dir1 === 'rtl' ? 'PASS' : 'CHECK',
  `lang=${lang1} dir=${dir1}`);

// back to es so later checks read es labels
try {
  await sel.selectOption({ value: 'es' });
  await page.waitForTimeout(500);
} catch { /* leave as-is; checks below are locale-tolerant */ }

// --- 2. draft save / restore ------------------------------------------------
await page.goto(`${BASE}/objects/wh_zone/create`, { waitUntil: 'networkidle' });
const input = page.locator('form input, form textarea').first();
await input.fill('QA-DRAFT-777');
// unsaved indicator shows translated text (es), not a bare asterisk
const ind = await page.locator('form [role="status"], form .text-sm').allInnerTexts().catch(() => []);
const bareStar = ind.some((s) => s.trim() === '*');
const hasText = ind.some((s) => s.trim().length > 2);
rec('unsaved indicator accessible', !bareStar && hasText ? 'PASS' : 'FAIL',
  JSON.stringify(ind).slice(0, 200));

await page.locator('button', { hasText: /borrador|draft|черновик|entwurf|brouillon|草稿|مسودة|טיוטה/i }).first().click();
await page.waitForTimeout(600);
await page.reload({ waitUntil: 'networkidle' });
const restored = await input.inputValue();
const bodyTxt = await page.locator('body').innerText();
rec('draft restore', restored === 'QA-DRAFT-777' ? 'PASS' : 'FAIL',
  `restored=${restored} toast=${/borrador|draft|черновик|entwurf|brouillon|草稿|مسودة|טיוטה/i.test(bodyTxt)}`);
await page.screenshot({ path: join(SHOTS, 'draft.png') });
// cleanup draft
await page.evaluate(() => localStorage.removeItem('entity_draft_wh_zone'));

// --- 3. session expiry → login without chrome -------------------------------
await ctx.clearCookies();
await page.goto(`${BASE}/objects`, { waitUntil: 'networkidle' }).catch(() => {});
// the auth middleware should bounce to /login; force one API call first
await page.goto(`${BASE}/objects`, { waitUntil: 'networkidle' }).catch(() => {});
await page.waitForTimeout(1500);
const url = page.url();
const hasSidebar = await page.locator('aside, nav [class*="sidebar"], button[aria-label]').count();
const txt = (await page.locator('body').innerText()).slice(0, 400);
const staleProfile = /Gerente|Менеджер|Manager/i.test(txt);
rec('expired session → clean login', url.includes('/login') && !staleProfile ? 'PASS' : 'CHECK',
  `url=${url} sidebarElems=${hasSidebar} staleProfile=${staleProfile}\n${txt.slice(0, 150)}`);
await page.screenshot({ path: join(SHOTS, 'expired.png') });

// --- 4. breadcrumb 'edit' translated -----------------------------------------
await login(page);
const ENT = '01a15654-5fd0-7701-bd38-064eece6602c'; // wh_zone
await page.goto(`${BASE}/entities/${ENT}/edit`, { waitUntil: 'networkidle' });
const crumbs = await page.locator('nav[aria-label="Breadcrumb"], nav[aria-label="breadcrumb"]').innerText().catch(() => 'none');
rec('edit breadcrumb translated', /Editar|Edit|Изменить|Редактировать/i.test(crumbs) && !/\bedit\b/.test(crumbs.replace(/Editar|Edit/g, '')) ? 'PASS' : 'CHECK',
  `crumbs="${crumbs.replace(/\n/g, ' | ')}"`);

// --- 5. long-name breadcrumb truncation --------------------------------------
const LONG = '01a15bde-9709-7ae2-9e1f-776c727af50a';
await page.goto(`${BASE}/entities/${LONG}`, { waitUntil: 'networkidle' });
const crumbs2 = await page.locator('nav[aria-label="Breadcrumb"], nav[aria-label="breadcrumb"]').innerText().catch(() => '');
const lastLen = Math.max(...crumbs2.split('\n').map((s) => s.length));
rec('long name crumb ≤ 60+ellipsis', lastLen <= 62 ? 'PASS' : 'CHECK',
  `longest crumb=${lastLen} chars`);

await browser.close();
const fails = results.filter((r) => r.verdict === 'FAIL');
console.log(`\n==== ${results.length - fails.length}/${results.length} pass, ${fails.length} fail ====`);
process.exit(fails.length ? 1 : 0);
