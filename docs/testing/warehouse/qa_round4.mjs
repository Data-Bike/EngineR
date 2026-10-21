/**
 * qa_round4.mjs — deep browser QA of the EngineR Nuxt admin UI pages that
 * were not covered by earlier rounds:
 *   /requests, /checks (+detail), /monitoring, /risks, /reports,
 *   /admin{,/types,/catalogs,/predicates,/users,/dump,/schema,/permissions},
 *   entity tabs (relations/checks/risks/tasks/documents/history),
 *   /search, /audit (filters, load-more, chain verification)
 * and a risk_analyst pass (sidebar gating + deep-link 403 handling).
 *
 * For every page: console errors, pageerrors, 4xx/5xx API responses,
 * horizontal overflow at 1440px and 400px, i18n missing-key literals,
 * [object Object] leaks, raw ISO timestamps in tables, and clickable
 * non-interactive elements (cursor-pointer without role/tabindex).
 *
 * Run:  node docs/testing/warehouse/qa_round4.mjs   (repo root)
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
const SHOTS = join(HERE, 'qa_shots_round4');
mkdirSync(SHOTS, { recursive: true });

const TOKENS = JSON.parse(readFileSync(join(HERE, '.wh_tokens.json'), 'utf8'));
const ADMIN_TOKEN = TOKENS.admin;

// --- fixtures -------------------------------------------------------------
const ENT_RU = '00000000-0000-7000-9000-000000000003'; // legal_entity ООО «Ромашка»

// --- collectors -------------------------------------------------------------
const results = [];
const apiCalls = [];    // {tag, method, url, status}
const consoleErrs = []; // {tag, text}
const pageErrs = [];    // {tag, text}
let tag = 'boot';

function rec(name, verdict, evidence) {
  results.push({ name, verdict, evidence });
  console.log(`\n### ${name}\n    ${verdict}\n    ${evidence}`);
}

function callsFor(t, statusMin = 400) {
  return apiCalls.filter((c) => c.tag === t && c.status >= statusMin);
}
function errsFor(t) {
  return {
    console: consoleErrs.filter((e) => e.tag === t),
    page: pageErrs.filter((e) => e.tag === t),
  };
}
function errSummary(t) {
  const e = errsFor(t);
  const c = callsFor(t);
  return (
    `consoleErr=${e.console.length}${e.console.length ? ' ' + JSON.stringify(e.console.slice(0, 3).map((x) => x.text.slice(0, 160))) : ''}` +
    ` | pageErr=${e.page.length}${e.page.length ? ' ' + JSON.stringify(e.page.slice(0, 3).map((x) => x.text.slice(0, 200))) : ''}` +
    ` | api>=400=${c.length}${c.length ? ' ' + JSON.stringify(c.slice(0, 6).map((x) => `${x.status} ${x.method} ${x.url}`)) : ''}`
  );
}

// --- helpers ---------------------------------------------------------------
async function apiFetch(path, opts = {}) {
  const r = await fetch(`${API}${path}`, {
    ...opts,
    headers: {
      Authorization: `Bearer ${ADMIN_TOKEN}`,
      'Content-Type': 'application/json',
      ...(opts.headers ?? {}),
    },
  });
  return { status: r.status, json: await r.json().catch(() => null) };
}

async function login(page, user, pass = 'enginer-demo') {
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await page.locator('input[autocomplete="username"]').fill(user);
  await page.locator('input[type="password"]').fill(pass);
  await page.locator('button[type="submit"]').click();
  await page.waitForURL((u) => !u.pathname.endsWith('/login'), { timeout: 20000 });
  await page.waitForLoadState('networkidle');
}

async function settle(page, ms = 1200) {
  await page.waitForLoadState('networkidle', { timeout: 9000 }).catch(() => {});
  await page.waitForTimeout(ms);
}

function wire(page) {
  page.on('console', (m) => {
    if (m.type() === 'error') consoleErrs.push({ tag, text: m.text() });
  });
  page.on('pageerror', (e) => pageErrs.push({ tag, text: String(e) }));
  page.on('response', (r) => {
    const u = r.url();
    if (u.includes('/api/')) {
      apiCalls.push({ tag, method: r.request().method(), url: u.replace(BASE, ''), status: r.status() });
    }
  });
  page.on('requestfailed', (r) => {
    if (r.url().includes('/api/')) {
      apiCalls.push({ tag, method: r.method(), url: r.url() + ` [FAILED:${r.failure()?.errorText}]`, status: 0 });
    }
  });
}

/** Scan the rendered body for i18n keys, [object Object], raw ISO dates. */
async function bodyLint(page) {
  const text = await page.locator('body').innerText().catch(() => '');
  const i18nKey = (text.match(/\b(?:nav|entity|requests|checks|risks|reports|audit|admin|catalog|users|schema|dump|common|search|errors|monitoring|integrations|docs|workflows|tasks|security|login|permissions)\.[a-zA-Z_][a-zA-Z0-9_.]*\b/g) ?? [])
    // exclude literal filenames (e.g. "dump.ndjson.xz" download links)
    .filter((k) => !/\.(xz|log|json|csv|zip|gz|ndjson)$/i.test(k));
  const objObj = (text.match(/\[object Object\]/g) ?? []).length;
  const iso = text.match(/\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?(\.\d+)?([+-]\d{2}:?\d{2}|Z)?\b/g) ?? [];
  const stuckLoading = /(^|\n)\s*(Загрузка|Loading|Laden|Cargando|Chargement|加载|تحميل|טוען)\s*(\.\.\.)?\s*($|\n)/i.test(text) && !/login/i.test(page.url());
  return {
    i18nKeys: [...new Set(i18nKey)],
    objObj,
    isoDates: [...new Set(iso)].slice(0, 5),
    isoCount: iso.length,
    stuckLoading,
  };
}

async function overflowInfo(page) {
  return page.evaluate(() => ({
    scrollW: Math.max(document.documentElement.scrollWidth, document.body?.scrollWidth ?? 0),
    innerW: window.innerWidth,
    overflow: Math.max(document.documentElement.scrollWidth, document.body?.scrollWidth ?? 0) > window.innerWidth + 1,
  }));
}

/** Elements styled clickable that are not keyboard/role accessible. */
async function deadClickables(page) {
  return page.evaluate(() => {
    const out = [];
    for (const el of document.querySelectorAll('[class*="cursor-pointer"]')) {
      const t = el.tagName;
      if (['A', 'BUTTON', 'INPUT', 'SELECT', 'TEXTAREA', 'SUMMARY'].includes(t)) continue;
      if (el.closest('a,button,[role="button"],[role="link"],label')) continue;
      if (el.getAttribute('role') || el.getAttribute('tabindex') !== null) continue;
      if (el.closest('tr') && el.tagName !== 'TR' && el.closest('tr').className.includes('cursor-pointer')) continue;
      out.push(`${t} ${(el.getAttribute('class') || '').slice(0, 80)}`);
      if (out.length > 5) break;
    }
    return out;
  });
}

/** Standard per-page audit at the current viewport. Returns the evidence bag. */
async function visit(page, url, name, { shot = true, allowedDenied = false } = {}) {
  tag = name;
  await page.goto(url, { waitUntil: 'domcontentloaded' }).catch((e) => rec(name + ' goto', 'FAIL', String(e)));
  await settle(page);
  const lint = await bodyLint(page);
  const ov = await overflowInfo(page);
  const dead = await deadClickables(page);
  const errs = errsFor(name);
  const bad = callsFor(name).filter((c) => !(allowedDenied && c.status === 403));
  const problems = [];
  if (errs.console.length) problems.push(`consoleErr=${errs.console.length}`);
  if (errs.page.length) problems.push(`pageErr=${errs.page.length}`);
  if (bad.length) problems.push(`api>=400=${bad.length}`);
  if (ov.overflow) problems.push(`h-overflow ${ov.scrollW}>${ov.innerW}`);
  if (lint.i18nKeys.length) problems.push(`i18n-keys:${lint.i18nKeys.slice(0, 5).join(',')}`);
  if (lint.objObj) problems.push(`[object Object]x${lint.objObj}`);
  if (lint.stuckLoading) problems.push('stuck-loading');
  if (shot) await page.screenshot({ path: join(SHOTS, `${name}.png`) });
  rec(name, problems.length ? 'CHECK' : 'PASS',
    `${url} | ${problems.join(' | ') || 'clean'} | ISO-dates=${lint.isoCount} ${lint.isoCount ? JSON.stringify(lint.isoDates.slice(0, 3)) : ''} | deadClick=${dead.length}${dead.length ? ' ' + JSON.stringify(dead.slice(0, 3)) : ''}\n    ${errSummary(name)}`);
  return { lint, ov, dead, errs, bad };
}

/** Empty-submit check for a modal dialog: open, verify submit disabled (or blocked), close. */
async function dialogEmptySubmit(page, openBtn, dlgName, shotPrefix) {
  await openBtn.click();
  await page.waitForTimeout(700);
  const dlg = page.locator('[role="dialog"]').last();
  const visible = await dlg.isVisible().catch(() => false);
  if (!visible) { rec(`${dlgName} opens`, 'FAIL', 'no [role=dialog] after click'); return null; }
  await page.screenshot({ path: join(SHOTS, `${shotPrefix}-dialog.png`) });
  const submit = dlg.locator('button').last();
  const disabled = await submit.evaluate((b) => b.disabled || b.getAttribute('aria-disabled') === 'true').catch(() => null);
  const apiBefore = apiCalls.length;
  await submit.click({ force: true }).catch(() => {});
  await page.waitForTimeout(600);
  const posted = apiCalls.slice(apiBefore).filter((c) => c.method === 'POST' && c.status !== 0);
  const stillOpen = await dlg.isVisible().catch(() => false);
  rec(`${dlgName} empty-submit`, (disabled || (stillOpen && posted.length === 0)) ? 'PASS' : 'FAIL',
    `submitDisabled=${disabled} stillOpen=${stillOpen} POSTs=${JSON.stringify(posted.map((p) => `${p.status} ${p.url}`))}`);
  return dlg;
}

async function closeDialog(page, dlg) {
  await page.keyboard.press('Escape');
  await page.waitForTimeout(500);
  const open = await dlg.isVisible().catch(() => false);
  return !open;
}

const browser = await chromium.launch();

// ===========================================================================
// PART A — admin persona @1440px
// ===========================================================================
{
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, ignoreHTTPSErrors: true });
  const page = await ctx.newPage();
  wire(page);
  tag = 'a-login';
  await login(page, 'admin');
  rec('A admin login', 'PASS', `landed ${page.url()}`);

  // --- /requests -----------------------------------------------------------
  const reqRes = await visit(page, `${BASE}/requests`, 'a-requests');
  // row detail link
  tag = 'a-requests-detail';
  const viewBtn = page.locator('table a, table button').last();
  if (await viewBtn.count()) {
    await viewBtn.click().catch(() => {});
    await settle(page);
    rec('a-requests-detail', page.url().includes('/entities/') ? 'PASS' : 'CHECK',
      `landed=${page.url()} | ${errSummary('a-requests-detail')}`);
  } else rec('a-requests-detail', 'INFO', 'no rows — skipped');
  await page.goto(`${BASE}/requests`, { waitUntil: 'domcontentloaded' });
  await settle(page);
  // new-request dialog
  tag = 'a-requests-dialog';
  const newReqBtn = page.locator('main button:has([class*="i-lucide:plus"])').first();
  const dlg1 = await dialogEmptySubmit(page, newReqBtn, 'requests-new', 'a-requests');
  if (dlg1) {
    const closed = await closeDialog(page, dlg1);
    if (!closed) {
      // fallback: click cancel (first button in footer)
      await dlg1.locator('button').first().click().catch(() => {});
      await page.waitForTimeout(400);
    }
    rec('requests dialog Esc-close', closed ? 'PASS' : 'CHECK', `closedByEsc=${closed}`);
  }

  // --- /checks --------------------------------------------------------------
  await visit(page, `${BASE}/checks`, 'a-checks');
  tag = 'a-checks-dialog';
  const newChkBtn = page.locator('main button:has([class*="i-lucide:plus"])').first();
  const dlg2 = await dialogEmptySubmit(page, newChkBtn, 'checks-new', 'a-checks');
  if (dlg2) await closeDialog(page, dlg2);
  // entity filter narrows rows client-side
  tag = 'a-checks-filter';
  const rows0 = await page.locator('tbody tr').count();
  let filtered = 'INFO';
  try {
    const anyIdCell = await page.locator('tbody tr td:nth-child(2)').first().innerText();
    const needle = anyIdCell.trim().slice(0, 6);
    const inputs = await page.locator('main input').all();
    for (const i of inputs) {
      const ph = await i.getAttribute('placeholder');
      const ty = await i.getAttribute('type');
      if (ph && ph.length > 3 && ty !== 'datetime-local') { await i.fill(needle); break; }
    }
    await page.waitForTimeout(400);
    const rows1 = await page.locator('tbody tr').count();
    filtered = `rows ${rows0} -> ${rows1} (needle=${needle})`;
  } catch (e) { filtered = 'filter probe failed: ' + e.message; }
  rec('a-checks-filter', 'INFO', filtered);
  // check detail page
  tag = 'a-checks-detail';
  const { json: checksList } = await apiFetch('/checks?limit=1');
  const checkId = checksList?.data?.[0]?.check_id;
  if (checkId) {
    await visit(page, `${BASE}/checks/${checkId}`, 'a-checks-detail');
  } else rec('a-checks-detail', 'INFO', 'no checks to open');

  // --- /monitoring -----------------------------------------------------------
  const monRes = await visit(page, `${BASE}/monitoring`, 'a-monitoring');
  tag = 'a-monitoring-run';
  const runBtn = page.locator('tbody button').first();
  if (await runBtn.count()) {
    const before = apiCalls.length;
    await runBtn.click();
    await page.waitForTimeout(2500);
    const posts = apiCalls.slice(before).filter((c) => /\/monitors\/.+\/run/.test(c.url));
    rec('a-monitoring run-now', posts.length && posts[0].status < 400 ? 'PASS' : 'CHECK',
      `POSTs=${JSON.stringify(posts.map((p) => `${p.status} ${p.url}`))}`);
    await page.screenshot({ path: join(SHOTS, 'a-monitoring-after-run.png') });
  } else rec('a-monitoring run-now', 'INFO', 'no monitor rows');

  // --- /risks ---------------------------------------------------------------
  await visit(page, `${BASE}/risks`, 'a-risks');

  // --- /reports -------------------------------------------------------------
  await visit(page, `${BASE}/reports`, 'a-reports');
  tag = 'a-reports-dialog';
  const genBtn = page.locator('main button:has([class*="i-lucide:file-plus"])').first();
  if (await genBtn.count()) {
    const dlg3 = await dialogEmptySubmit(page, genBtn, 'reports-generate', 'a-reports');
    if (dlg3) {
      // count options in the two selects
      const combos = dlg3.locator('[role="combobox"], select, button:has([class*="chevron"])');
      const nCombos = await combos.count();
      let optCounts = [];
      for (let i = 0; i < Math.min(nCombos, 2); i++) {
        try {
          await combos.nth(i).click();
          await page.waitForTimeout(500);
          const opts = await page.locator('[role="option"], [role="listbox"] [data-highlighted], [role="listbox"] li').count();
          optCounts.push(opts);
          await page.keyboard.press('Escape');
          await page.waitForTimeout(300);
        } catch (e) { optCounts.push('err'); }
      }
      rec('reports dialog selects', 'INFO', `combos=${nCombos} optionCounts=${JSON.stringify(optCounts)}`);
      await page.screenshot({ path: join(SHOTS, 'a-reports-dialog-open.png') });
      await closeDialog(page, dlg3);
    }
  } else rec('a-reports-dialog', 'INFO', 'no generate button (report.run cap missing?)');
  // download gating
  tag = 'a-reports-download';
  const dlBtn = page.locator('tbody button').first();
  if (await dlBtn.count()) {
    const [popup] = await Promise.all([
      ctx.waitForEvent('page', { timeout: 5000 }).catch(() => null),
      dlBtn.click(),
    ]);
    rec('a-reports download', popup ? 'PASS' : 'CHECK',
      `popup=${popup ? popup.url().slice(0, 140) : 'none'} | ${errSummary('a-reports-download')}`);
    if (popup) await popup.close();
  } else rec('a-reports download', 'INFO', 'no ready/downloadable report row');

  // --- /admin + subpages ------------------------------------------------------
  await visit(page, `${BASE}/admin`, 'a-admin');
  const cards = await page.locator('a[data-testid^="admin-card-"]').count();
  rec('a-admin cards', cards >= 5 ? 'PASS' : 'CHECK', `cards=${cards}`);
  for (const sub of ['types', 'catalogs', 'predicates', 'users', 'dump', 'schema', 'permissions']) {
    await visit(page, `${BASE}/admin/${sub}`, `a-admin-${sub}`);
  }

  // --- entity tabs ------------------------------------------------------------
  await visit(page, `${BASE}/entities/${ENT_RU}`, 'a-entity-overview');
  const tabNames = ['relations', 'graph', 'checks', 'risks', 'tasks', 'documents', 'history'];
  const tabs = page.locator('[role="tab"]');
  const nTabs = await tabs.count();
  for (let i = 1; i < nTabs && i - 1 < tabNames.length; i++) {
    const tn = tabNames[i - 1];
    tag = `a-enttab-${tn}`;
    await tabs.nth(i).click().catch(() => {});
    await settle(page, 1400);
    const lint = await bodyLint(page);
    const panelTxt = await page.evaluate(() => {
      const p = document.querySelector('[role="tabpanel"]:not([hidden])') ??
        document.querySelector('[data-state="active"][role="tabpanel"]');
      return (p?.innerText ?? '').replace(/\s+/g, ' ').slice(0, 180);
    });
    const errs = errsFor(tag);
    const bad = callsFor(tag);
    const problems = [];
    if (errs.console.length) problems.push(`consoleErr=${errs.console.length}`);
    if (errs.page.length) problems.push(`pageErr=${errs.page.length}`);
    if (bad.length) problems.push(`api>=400=${bad.length}`);
    if (lint.i18nKeys.length) problems.push(`i18n:${lint.i18nKeys.slice(0, 4).join(',')}`);
    if (lint.objObj) problems.push('[object Object]');
    await page.screenshot({ path: join(SHOTS, `a-enttab-${tn}.png`) });
    rec(`a-enttab-${tn}`, problems.length ? 'CHECK' : 'PASS',
      `panel="${panelTxt.slice(0, 160)}" | ${problems.join(' | ') || 'clean'} | ISO=${lint.isoCount}\n    ${errSummary(tag)}`);
  }
  // risks tab: resolve + new assessment dialog
  tag = 'a-enttab-risks-actions';
  {
    // make sure we're on the risks tab
    const rIdx = tabNames.indexOf('risks') + 1;
    await tabs.nth(rIdx).click().catch(() => {});
    await settle(page, 1000);
    const panel = page.locator('[role="tabpanel"]:not([hidden]), [data-state="active"][role="tabpanel"]').first();
    // resolve button: panel button without the clipboard-check icon and not inside the header row
    const resv = panel.locator('button:not(:has([class*="i-lucide:clipboard-check"]))').filter({ hasText: /\S/ }).first();
    if (await resv.count()) {
      const label = await resv.innerText().catch(() => '?');
      const before = apiCalls.length;
      await resv.click().catch(() => {});
      await page.waitForTimeout(2000);
      const posts = apiCalls.slice(before).filter((c) => /factors\/.+\/resolve/.test(c.url));
      rec('a-entity risk resolve', posts.length ? (posts[0].status < 400 ? 'PASS' : 'CHECK') : 'INFO',
        `btn="${label.slice(0, 40)}" POSTs=${JSON.stringify(posts.map((p) => `${p.status} ${p.url}`))} | ${errSummary('a-enttab-risks-actions')}`);
    } else rec('a-entity risk resolve', 'INFO', 'no resolvable factor button');
    // new assessment dialog: open, verify it renders fields, close (submit is
    // intentionally ungated — selects have defaults)
    const assessBtn = panel.locator('button:has([class*="i-lucide:clipboard-check"])').first();
    if (await assessBtn.count()) {
      await assessBtn.click();
      await page.waitForTimeout(700);
      const dlg = page.locator('[role="dialog"], [data-slot="content"]').last();
      const visible = await dlg.isVisible().catch(() => false);
      const fields = await dlg.locator('[role="combobox"], select, input, textarea').count().catch(() => 0);
      await page.screenshot({ path: join(SHOTS, 'a-enttab-risks-dialog.png') });
      rec('risks-assessment dialog', visible && fields >= 2 ? 'PASS' : 'CHECK',
        `visible=${visible} fields=${fields}`);
      await closeDialog(page, dlg);
    } else rec('risks-assessment dialog', 'INFO', 'no new-assessment button');
  }

  // --- /search -----------------------------------------------------------------
  tag = 'a-search';
  await visit(page, `${BASE}/search`, 'a-search-empty');
  const searchInput = page.locator('input[placeholder], main input').first();
  await searchInput.fill('Ромашка');
  await page.keyboard.press('Enter');
  await settle(page, 1500);
  const hits = await page.locator('a[href^="/entities/"]').count();
  const sLint = await bodyLint(page);
  await page.screenshot({ path: join(SHOTS, 'a-search-results.png') });
  rec('a-search query', hits > 0 ? 'PASS' : 'CHECK',
    `hits=${hits} url=${page.url()} | i18n=${sLint.i18nKeys.join(',') || 'none'} | ${errSummary('a-search')}`);
  // click-through
  if (hits > 0) {
    tag = 'a-search-click';
    await page.locator('a[href^="/entities/"]').first().click();
    await settle(page);
    rec('a-search click-through', page.url().includes('/entities/') ? 'PASS' : 'FAIL', `landed=${page.url()}`);
  }
  // nonsense query → empty state
  tag = 'a-search-noresults';
  await page.goto(`${BASE}/search?q=zzzzqqq__no_match__`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1500);
  const emptyTxt = await page.locator('body').innerText();
  const noHits = await page.locator('a[href^="/entities/"]').count();
  rec('a-search empty state', noHits === 0 ? 'PASS' : 'CHECK',
    `hits=${noHits} | ${errSummary('a-search-noresults')}`);
  await page.screenshot({ path: join(SHOTS, 'a-search-noresults.png') });

  // --- /audit -------------------------------------------------------------------
  await visit(page, `${BASE}/audit`, 'a-audit');
  tag = 'a-audit-verify';
  const verifyBtn = page.locator('main button:has([class*="i-lucide:shield-check"])').first();
  if (await verifyBtn.count()) {
    const before = apiCalls.length;
    await verifyBtn.click();
    await page.waitForTimeout(3000);
    const posts = apiCalls.slice(before).filter((c) => /audit\/verify/.test(c.url));
    const alertVisible = await page.locator('[role="alert"], .rounded-lg:has(svg)').first().isVisible().catch(() => false);
    rec('a-audit verify chain', posts.length && posts[0].status < 400 ? 'PASS' : 'CHECK',
      `POST=${JSON.stringify(posts.map((p) => `${p.status}`))} alertVisible=${alertVisible}`);
    await page.screenshot({ path: join(SHOTS, 'a-audit-verify.png') });
  } else rec('a-audit verify chain', 'FAIL', 'verify button not found');
  // expand a row
  tag = 'a-audit-expand';
  const firstRow = page.locator('tbody tr').first();
  await firstRow.click().catch(() => {});
  await page.waitForTimeout(600);
  const expanded = await page.locator('tbody tr td[colspan]').count();
  rec('a-audit row expand', expanded > 0 ? 'PASS' : 'CHECK', `detailCells=${expanded} | ${errSummary('a-audit-expand')}`);
  await page.screenshot({ path: join(SHOTS, 'a-audit-expanded.png') });
  // load more
  tag = 'a-audit-loadmore';
  const rowsBefore = await page.locator('tbody tr').count();
  // locate the load-more button by translated text across supported locales
  const lmBtn = page.locator('main button:not([type="submit"])').filter({ hasText: /ещё|еще|more|mehr|más|plus|更多|المزيد|עוד/i }).first();
  if (await lmBtn.count()) {
    await lmBtn.click();
    await page.waitForTimeout(2500);
    const rowsAfter = await page.locator('tbody tr').count();
    rec('a-audit load more', rowsAfter > rowsBefore ? 'PASS' : 'CHECK',
      `rows ${rowsBefore} -> ${rowsAfter} | ${errSummary('a-audit-loadmore')}`);
  } else rec('a-audit load more', 'INFO', 'no load-more button (<100 entries)');
  // actor filter
  tag = 'a-audit-filter';
  const actorInputs = await page.locator('main input[type="text"], main input:not([type])').all();
  if (actorInputs.length >= 2) {
    await actorInputs[0].fill('admin');
    await page.locator('main button:has([class*="i-lucide:search"])').first().click();
    await settle(page, 1500);
    rec('a-audit actor filter', 'INFO', `rows=${await page.locator('tbody tr').count()} | ${errSummary('a-audit-filter')}`);
  }

  // --- 400px overflow pass ------------------------------------------------------
  tag = 'a-mobile';
  await page.setViewportSize({ width: 400, height: 850 });
  const mobilePages = [
    ['/requests', 'requests'], ['/checks', 'checks'], ['/monitoring', 'monitoring'],
    ['/risks', 'risks'], ['/reports', 'reports'], ['/admin', 'admin'],
    ['/admin/types', 'admin-types'], ['/admin/users', 'admin-users'],
    [`/entities/${ENT_RU}`, 'entity'], ['/search', 'search'], ['/audit', 'audit'],
  ];
  for (const [u, n] of mobilePages) {
    tag = `m-${n}`;
    await page.goto(`${BASE}${u}`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await settle(page, 900);
    const ov = await overflowInfo(page);
    const errs = errsFor(tag);
    if (ov.overflow || errs.page.length) {
      await page.screenshot({ path: join(SHOTS, `m-${n}-400.png`) });
    }
    rec(`400px ${n}`, !ov.overflow && errs.page.length === 0 ? 'PASS' : 'FAIL',
      `scrollW=${ov.scrollW} innerW=${ov.innerW} pageErr=${errs.page.length}${errs.page.length ? ' ' + errs.page[0].text.slice(0, 160) : ''} consoleErr=${errs.console.length}`);
  }

  await ctx.close();
}

// ===========================================================================
// PART B — risk_analyst persona: sidebar gating + 403 deep links
// ===========================================================================
{
  const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, ignoreHTTPSErrors: true });
  const page = await ctx.newPage();
  wire(page);
  tag = 'b-login';
  await login(page, 'risk_analyst');
  await settle(page, 1200);

  tag = 'b-sidebar';
  const hrefs = await page.locator('aside a[href], nav a[href]').evaluateAll((els) =>
    els.map((a) => a.getAttribute('href')));
  const uniq = [...new Set(hrefs)];
  const leaks = uniq.filter((h) => /\/admin|\/audit|\/monitoring|\/integrations/.test(h));
  rec('B sidebar gating (risk_analyst)', leaks.length === 0 ? 'PASS' : 'FAIL',
    `links=${JSON.stringify(uniq)} | leaked=${JSON.stringify(leaks)}`);
  await page.screenshot({ path: join(SHOTS, 'b-sidebar.png') });

  // deep links — all should render a friendly 403 (not crash, not leak data)
  for (const sub of ['', '/permissions', '/types', '/catalogs', '/predicates', '/users', '/dump', '/schema']) {
    tag = `b-deny-admin${sub || '-index'}`;
    await page.goto(`${BASE}/admin${sub}`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await settle(page, 1200);
    const txt = await page.locator('body').innerText().catch(() => '');
    const got403 = /\b403\b/.test(txt) || /denied|forbidden|запрещ|доступ/i.test(txt);
    const hasTableData = (await page.locator('tbody tr').count()) > 0;
    const errs = errsFor(tag);
    const dataOk = callsFor(tag, 0).filter((c) => c.status === 200 && /users|schemas|dump|catalogs/.test(c.url));
    await page.screenshot({ path: join(SHOTS, `b-deny-admin${sub.replace(/\//g, '_') || '_index'}.png`) });
    rec(`B deep-link /admin${sub || ''}`, got403 && !hasTableData && dataOk.length === 0 ? 'PASS' : 'FAIL',
      `403ui=${got403} rows=${hasTableData ? 'LEAK' : 0} 200-data-calls=${dataOk.length} | ${errSummary(tag).slice(0, 300)}`);
  }
  for (const p of ['/monitoring', '/audit']) {
    tag = `b-deny${p.replace(/\//g, '-')}`;
    await page.goto(`${BASE}${p}`, { waitUntil: 'domcontentloaded' }).catch(() => {});
    await settle(page, 1200);
    const txt = await page.locator('body').innerText().catch(() => '');
    const got403 = /\b403\b/.test(txt) || /denied|forbidden|запрещ|доступ/i.test(txt);
    await page.screenshot({ path: join(SHOTS, `b-deny${p.replace(/\//g, '_')}.png`) });
    rec(`B deep-link ${p}`, got403 ? 'PASS' : 'FAIL', `403ui=${got403} body="${txt.slice(0, 140).replace(/\n/g, ' | ')}"`);
  }
  // allowed pages still render
  for (const p of ['/requests', '/checks', '/risks', '/reports', '/search']) {
    await visit(page, `${BASE}${p}`, `b-allow${p.replace(/\//g, '-')}`);
  }

  await ctx.close();
}

await browser.close();

const fails = results.filter((r) => r.verdict === 'FAIL');
const checks = results.filter((r) => r.verdict === 'CHECK');
console.log(`\n===== ${results.length} checks: ${results.length - fails.length - checks.length} PASS, ${checks.length} CHECK, ${fails.length} FAIL =====`);
for (const f of fails) console.log(`FAIL: ${f.name} — ${f.evidence.split('\n')[0]}`);
process.exit(fails.length ? 1 : 0);
