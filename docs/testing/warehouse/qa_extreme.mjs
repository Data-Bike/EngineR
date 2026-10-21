/**
 * qa_extreme.mjs — extreme-scenario UI probe of the EngineR Nuxt admin UI.
 * Emulates a stressed/confused user: session loss mid-work, rapid tab
 * clicking, back/forward with dirty forms, multi-tab edit races, long
 * data, empty states, catalog switching mid-flow, network failure, and a
 * console/network audit across every page.
 *
 * playwright-core resolved from nuxt/node_modules.
 * Run:  node ../docs/testing/warehouse/qa_extreme.mjs   (from nuxt/)
 *   or: node docs/testing/warehouse/qa_extreme.mjs      (repo root)
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
const SHOTS = join(HERE, 'qa_shots_extreme');
mkdirSync(SHOTS, { recursive: true });

const TOKENS = JSON.parse(
  readFileSync(join(HERE, '.wh_tokens.json'), 'utf8'),
);
const ADMIN_TOKEN = TOKENS.admin;

// --- fixtures -------------------------------------------------------------
const WH_ZONE_ENTITY = '01a15bc3-3c23-70c0-905f-783a75e3bd19'; // wh / wh_zone
const LONG_ENTITY = '01a15bde-9709-7ae2-9e1f-776c727af50a'; // qa_t / qa_item, 303-char name

// --- collectors -------------------------------------------------------------
const results = [];
const apiCalls = []; // {tag, method, url, status, ts}
const consoleErrs = [];
const pageErrs = [];
const dialogs = [];
let tag = 'boot';
let dialogAction = 'dismiss'; // consumed by page.on('dialog')

function rec(name, verdict, evidence) {
  results.push({ name, verdict, evidence });
  console.log(`\n### ${name}\n    ${verdict}\n    ${evidence}`);
}

function callsSince(marker) {
  return apiCalls.filter((c) => c.tag === marker);
}

function floodReport(t) {
  const calls = callsSince(t);
  const byKey = {};
  for (const c of calls) {
    const k = `${c.method} ${c.url.split('?')[0]}`;
    byKey[k] = (byKey[k] ?? 0) + 1;
  }
  return Object.entries(byKey)
    .filter(([, n]) => n >= 5)
    .map(([k, n]) => `${k} x${n}`);
}

function errsFor(t) {
  return {
    console: consoleErrs.filter((e) => e.tag === t),
    page: pageErrs.filter((e) => e.tag === t),
  };
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
  return r.json();
}

async function login(page, user, pass = 'enginer-demo') {
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await page.locator('input[autocomplete="username"]').fill(user);
  await page.locator('input[type="password"]').fill(pass);
  await page.locator('button[type="submit"]').click();
  await page.waitForURL((u) => !u.pathname.endsWith('/login'), {
    timeout: 20000,
  });
  await page.waitForLoadState('networkidle');
}

async function snap(page) {
  const body = await page
    .locator('body')
    .innerText()
    .catch(() => '');
  return { url: page.url(), body: body.slice(0, 1500) };
}

async function crumbs(page) {
  return page.evaluate(() => {
    const nav =
      document.querySelector('nav[aria-label="Breadcrumb"]') ??
      [...document.querySelectorAll('nav')].find((n) =>
        n.querySelector('ol, ul'),
      );
    if (!nav) return null;
    return [...nav.querySelectorAll('li')].map((li) => ({
      text: li.innerText.trim().replace(/\n/g, ' '),
      href: li.querySelector('a')?.getAttribute('href') ?? null,
    }));
  });
}

function wire(page, ctxTag) {
  page.on('console', (m) => {
    if (m.type() === 'error') consoleErrs.push({ tag: ctxTag ?? tag, text: m.text() });
  });
  page.on('pageerror', (e) => pageErrs.push({ tag: ctxTag ?? tag, text: String(e) }));
  page.on('response', (r) => {
    const u = r.url();
    if (u.includes('/api/')) {
      apiCalls.push({
        tag: ctxTag ?? tag,
        method: r.request().method(),
        url: u.replace(BASE, ''),
        status: r.status(),
        ts: Date.now(),
      });
    }
  });
  page.on('requestfailed', (r) => {
    if (r.url().includes('/api/')) {
      apiCalls.push({
        tag: ctxTag ?? tag,
        method: r.method(),
        url: r.url().replace(BASE, '') + ` [FAILED:${r.failure()?.errorText}]`,
        status: 0,
        ts: Date.now(),
      });
    }
  });
  page.on('dialog', async (d) => {
    dialogs.push({ tag: ctxTag ?? tag, type: d.type(), message: d.message() });
    if (dialogAction === 'accept') await d.accept();
    else await d.dismiss();
  });
}

/** Header select helper: open the Nth header combobox, pick option by exact name. */
async function pickHeaderSelect(page, index, optionText) {
  const combo = page.locator('header [role="combobox"]').nth(index);
  await combo.click();
  const opt = page.getByRole('option', { name: optionText, exact: true }).first();
  await opt.waitFor({ state: 'visible', timeout: 5000 });
  await opt.click();
  await page.waitForTimeout(400);
}

const browser = await chromium.launch();

// ===========================================================================
// Scenario 1 — session expiry mid-work
// ===========================================================================
{
  tag = 's1-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'manager');
  rec('S1 login manager', 'PASS', `landed ${page.url()}`);

  // open an entity card (mid-work state)
  tag = 's1-entity-open';
  await page.goto(`${BASE}/entities/${WH_ZONE_ENTITY}`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(1200);
  let s = await snap(page);
  const tabsBefore = await page.locator('[role="tab"]').count();
  rec(
    'S1 entity open before expiry',
    tabsBefore > 0 ? 'PASS' : 'CHECK',
    `tabs=${tabsBefore} url=${s.url}`,
  );
  await page.screenshot({ path: join(SHOTS, 's1-before-expiry.png') });

  // kill the session cookie out from under the SPA
  tag = 's1-expired-action';
  await ctx.clearCookies();
  const remaining = await ctx.cookies(BASE);
  rec('S1 cookie cleared', remaining.length === 0 ? 'PASS' : 'FAIL',
    `remaining=${JSON.stringify(remaining.map((c) => c.name))}`);

  // trigger an in-page API action: switch to a data-loading tab
  apiCalls.length = 0;
  const relationsTab = page.locator('[role="tab"]').nth(1);
  await relationsTab.click().catch(() => {});
  // then a client-side navigation as well (store still populated -> page
  // renders, API calls 401 -> expected redirect to /login, no spinner)
  await page.locator('a[href="/objects"], a[href$="/objects"]').first().click().catch(() => {});
  await page.waitForTimeout(4000);
  s = await snap(page);
  const landed401 = apiCalls.filter((c) => c.status === 401);
  const spinnerStuck = /Загрузка|Loading|loading/i.test(s.body) && !s.url.includes('/login');
  await page.screenshot({ path: join(SHOTS, 's1-after-expiry.png') });
  rec(
    'S1 expired session -> action + nav',
    s.url.includes('/login') && !spinnerStuck ? 'PASS' : 'FAIL',
    `final=${s.url} | 401s=${landed401.length} | spinner-stuck=${spinnerStuck} | body="${s.body.slice(0, 200).replace(/\n/g, ' | ')}"`,
  );

  // hard reload while unauthenticated
  tag = 's1-expired-reload';
  await page.reload({ waitUntil: 'networkidle' }).catch(() => {});
  await page.waitForTimeout(1500);
  s = await snap(page);
  rec(
    'S1 reload after expiry',
    s.url.includes('/login') ? 'PASS' : 'FAIL',
    `final=${s.url}`,
  );
  const floods = floodReport('s1-expired-action').concat(floodReport('s1-expired-reload'));
  if (floods.length) rec('S1 request flood on 401', 'FAIL', floods.join('; '));
  await ctx.close();
}

// ===========================================================================
// Scenario 2 — breadcrumbs + locale (en then ar, RTL stress)
// ===========================================================================
{
  tag = 's2-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');

  // switch to English via header locale select (last combobox in header)
  tag = 's2-locale-en';
  await page.waitForTimeout(800);
  const comboCount = await page.locator('header [role="combobox"]').count();
  try {
    await pickHeaderSelect(page, comboCount - 1, 'English');
  } catch (e) {
    rec('S2 locale switch en', 'CHECK', `select interaction failed: ${e}`);
  }
  await page.waitForTimeout(800);
  const cookie = (await ctx.cookies(BASE)).find((c) => c.name === 'enginer_locale');
  rec(
    'S2 locale switch en',
    'INFO',
    `url=${page.url()} localeCookie=${cookie?.value} combos=${comboCount}`,
  );

  // deep navigation: /objects -> wh_zone -> entity -> edit
  tag = 's2-nav-objects';
  await page.goto(`${BASE}/objects`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1500);
  const crumbsObjects = await crumbs(page);
  const cardLink = page.locator('a:has-text("wh_zone")').first();
  await cardLink.click().catch(() => {});
  await page.waitForTimeout(1500);
  tag = 's2-nav-list';
  const crumbsList = await crumbs(page);
  const urlList = page.url();
  const firstRow = page.locator('tbody tr td a').first();
  await firstRow.click().catch(() => {});
  await page.waitForTimeout(1800);
  tag = 's2-nav-entity';
  const crumbsEntity = await crumbs(page);
  const urlEntity = page.url();
  const editBtn = page.locator('a:has-text("Edit"), a:has-text("Редактировать"), button:has-text("Edit")').first();
  await editBtn.click().catch(() => {});
  await page.waitForTimeout(1800);
  tag = 's2-nav-edit';
  const crumbsEdit = await crumbs(page);
  const urlEdit = page.url();
  rec(
    'S2 breadcrumbs deep nav (en)',
    'INFO',
    `objects=${JSON.stringify(crumbsObjects)} | list=${JSON.stringify(crumbsList)} @${urlList} | entity=${JSON.stringify(crumbsEntity)} @${urlEntity} | edit=${JSON.stringify(crumbsEdit)} @${urlEdit}`,
  );
  await page.screenshot({ path: join(SHOTS, 's2-en-edit.png') });

  // switch to Arabic — RTL stress
  tag = 's2-locale-ar';
  const comboCount2 = await page.locator('header [role="combobox"]').count();
  try {
    await pickHeaderSelect(page, comboCount2 - 1, 'العربية');
  } catch (e) {
    rec('S2 locale switch ar', 'CHECK', `select failed: ${e}`);
  }
  await page.waitForTimeout(1500);
  const rtl = await page.evaluate(() => ({
    dir: document.documentElement.dir,
    lang: document.documentElement.lang,
    scrollW: document.documentElement.scrollWidth,
    clientW: document.documentElement.clientWidth,
  }));
  rec(
    'S2 ar RTL at 1440px',
    rtl.dir === 'rtl' && rtl.scrollW <= rtl.clientW + 2 ? 'PASS' : 'CHECK',
    JSON.stringify(rtl) + ` url=${page.url()}`,
  );
  await page.screenshot({ path: join(SHOTS, 's2-ar-1440.png'), fullPage: false });

  await page.setViewportSize({ width: 400, height: 850 });
  await page.waitForTimeout(900);
  const rtl400 = await page.evaluate(() => ({
    dir: document.documentElement.dir,
    scrollW: document.documentElement.scrollWidth,
    clientW: document.documentElement.clientWidth,
    headerH: document.querySelector('header')?.getBoundingClientRect().height,
    sidebarVisible: !!document.querySelector('aside, nav[class*="side"]'),
  }));
  rec(
    'S2 ar RTL at 400px',
    rtl400.dir === 'rtl' && rtl400.scrollW <= rtl400.clientW + 2 ? 'PASS' : 'FAIL',
    JSON.stringify(rtl400),
  );
  await page.screenshot({ path: join(SHOTS, 's2-ar-400.png'), fullPage: true });

  // back to default locale for later scenarios — leave ar; each new
  // context starts fresh anyway.
  await ctx.close();
}

// ===========================================================================
// Scenario 3 — rapid tab clicking (race / stale tab content)
// ===========================================================================
{
  tag = 's3-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');
  tag = 's3-tabs';
  await page.goto(`${BASE}/entities/${WH_ZONE_ENTITY}`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(1500);
  const tabs = page.locator('[role="tab"]');
  const tabCount = await tabs.count();
  const tabLabels = await tabs.allInnerTexts();
  // spam: alternate between distant tabs 10x with no settle time
  for (let i = 0; i < 10; i++) {
    const idx = i % tabCount;
    await tabs.nth(idx).click({ timeout: 2000 }).catch(() => {});
    await page.waitForTimeout(40);
  }
  // finish on last tab (history), then settle
  await tabs.nth(tabCount - 1).click().catch(() => {});
  await page.waitForTimeout(2500);
  const selected = await page.evaluate(() => {
    const el = document.querySelector('[role="tab"][aria-selected="true"], [role="tab"][data-state="active"]');
    return el?.textContent?.trim() ?? null;
  });
  const panelText = await page.evaluate(() => {
    const p =
      document.querySelector('[role="tabpanel"]:not([hidden])') ??
      document.querySelector('[data-state="active"][role="tabpanel"]');
    return (p?.innerText ?? '').slice(0, 400);
  });
  const errs = errsFor('s3-tabs');
  await page.screenshot({ path: join(SHOTS, 's3-after-spam.png') });
  rec(
    'S3 rapid tab switching x10',
    errs.console.length === 0 && errs.page.length === 0 ? 'PASS' : 'FAIL',
    `tabs=${tabCount} [${tabLabels.join(', ').slice(0, 160)}] | selected="${selected}" | panel="${panelText.replace(/\n/g, ' | ').slice(0, 300)}" | consoleErrs=${JSON.stringify(errs.console.slice(0, 5))} pageErrs=${JSON.stringify(errs.page.slice(0, 3))}`,
  );
  const floods = floodReport('s3-tabs');
  rec('S3 request flood from tab spam', floods.length === 0 ? 'PASS' : 'CHECK',
    floods.join('; ') || 'none');
  await ctx.close();
}

// ===========================================================================
// Scenario 4 — back/forward with unsaved form
// ===========================================================================
{
  tag = 's4-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');
  tag = 's4-form';
  dialogs.length = 0;

  await page.goto(`${BASE}/objects/wh_zone?catalog=wh`, {
    waitUntil: 'networkidle',
  });
  await page.goto(`${BASE}/objects/wh_zone/create?catalog=wh`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(1500);
  const codeInput = page.locator('form input').first();
  await codeInput.waitFor({ state: 'visible', timeout: 10000 });
  await codeInput.fill('GUARD-TEST-01');
  await page.waitForTimeout(300);

  // 1st Back — dismiss the confirm -> must stay on page with data
  dialogAction = 'dismiss';
  await page.goBack().catch(() => {});
  await page.waitForTimeout(1200);
  let s = await snap(page);
  const stayed = s.url.includes('/create');
  const valAfterDismiss = await codeInput.inputValue().catch(() => '<gone>');
  rec(
    'S4 Back w/ unsaved (dismiss guard)',
    stayed ? 'PASS' : 'FAIL',
    `url=${s.url} | dialogs=${JSON.stringify(dialogs)} | field="${valAfterDismiss}"`,
  );

  // 2nd Back — accept the confirm -> navigate away
  dialogAction = 'accept';
  await page.goBack().catch(() => {});
  await page.waitForTimeout(1500);
  s = await snap(page);
  const left = !s.url.includes('/create');
  rec(
    'S4 Back w/ unsaved (accept guard)',
    left ? 'PASS' : 'FAIL',
    `url=${s.url} | dialogs=${JSON.stringify(dialogs.slice(-3))}`,
  );

  // Forward — is form state restored?
  await page.goForward().catch(() => {});
  await page.waitForTimeout(2000);
  s = await snap(page);
  const valAfterForward = await page
    .locator('form input')
    .first()
    .inputValue()
    .catch(() => '<no-form>');
  rec(
    'S4 Forward after leaving',
    'INFO',
    `url=${s.url} | field-after-forward="${valAfterForward}" (empty = state lost on remount)`,
  );
  await page.screenshot({ path: join(SHOTS, 's4-forward.png') });

  // draft restore check: fill + click "save draft" then reload
  tag = 's4-draft';
  const draftInput = page.locator('form input').first();
  if (await draftInput.count()) {
    await draftInput.fill('DRAFT-TEST-77');
    const draftBtn = page.locator('button:has-text("черновик"), button:has-text("draft"), button:has-text("Draft")').first();
    const hasDraft = (await draftBtn.count()) > 0;
    if (hasDraft) {
      await draftBtn.click();
      await page.waitForTimeout(400);
      const stored = await page.evaluate(() =>
        localStorage.getItem('entity_draft_wh_zone'),
      );
      await page.reload({ waitUntil: 'networkidle' });
      await page.waitForTimeout(1500);
      const restored = await page
        .locator('form input')
        .first()
        .inputValue()
        .catch(() => '<no-form>');
      rec(
        'S4 draft save+reload restore',
        stored && restored === 'DRAFT-TEST-77' ? 'PASS' : 'CHECK',
        `localStorage=${stored} | restored="${restored}"`,
      );
    } else {
      rec('S4 draft button', 'INFO', 'no save-draft button found');
    }
  }
  await ctx.close();
}

// ===========================================================================
// Scenario 5 — multi-tab edit race (stale revision -> expect 409)
// ===========================================================================
{
  tag = 's5-setup';
  // fresh entity to mutate safely
  const code = `QA-MT-${Date.now().toString(36)}`;
  const created = await apiFetch('/entities?catalog=qa_t', {
    method: 'POST',
    body: JSON.stringify({
      type_id: 'qa_item',
      identity_mode: 'return_existing',
      fields: { qa_code: code, name: 'Multitab-Original' },
    }),
  });
  const pid = created?.data?.public_id;
  rec('S5 seed entity', pid ? 'PASS' : 'FAIL', JSON.stringify(created).slice(0, 300));

  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page1 = await ctx.newPage();
  wire(page1, 's5-tab1');
  await login(page1, 'admin');

  const page2 = await ctx.newPage();
  wire(page2, 's5-tab2');
  await page2.goto(`${BASE}/entities/${pid}/edit`, { waitUntil: 'networkidle' });
  tag = 's5-tab1';
  await page1.goto(`${BASE}/entities/${pid}/edit`, { waitUntil: 'networkidle' });
  await page1.waitForTimeout(1500);
  tag = 's5-tab2';
  await page2.waitForTimeout(1500);

  // tab1 edits name and saves
  const name1 = page1.locator('form input').nth(1);
  await name1.fill('Multitab-Tab1');
  apiCalls.length = 0;
  await page1.locator('button[type="submit"]').click();
  await page1.waitForTimeout(2500);
  const t1calls = apiCalls.filter((c) => c.tag === 's5-tab1' && c.method === 'PUT');
  const tab1Url = page1.url();
  rec(
    'S5 tab1 save',
    !tab1Url.includes('/edit') ? 'PASS' : 'CHECK',
    `url=${tab1Url} PUTs=${JSON.stringify(t1calls)}`,
  );

  // tab2 edits (stale revision) and saves
  const name2 = page2.locator('form input').nth(1);
  await name2.fill('Multitab-Tab2');
  apiCalls.length = 0;
  await page2.locator('button[type="submit"]').click();
  await page2.waitForTimeout(2500);
  const t2calls = apiCalls.filter((c) => c.tag === 's5-tab2' && c.method === 'PUT');
  const tab2Url = page2.url();
  const tab2Body = (await page2.locator('body').innerText().catch(() => '')).slice(0, 600);
  await page2.screenshot({ path: join(SHOTS, 's5-tab2-after-save.png') });

  // ground truth from the API
  const after = await apiFetch(`/entities/${pid}`);
  const finalName = after?.data?.display_name;
  const finalRev = after?.data?.revision;
  const silentOverwrite = finalName === 'Multitab-Tab2';
  rec(
    'S5 tab2 stale save',
    !silentOverwrite && t2calls.some((c) => c.status === 409) ? 'PASS'
    : silentOverwrite ? 'FAIL-SILENT-OVERWRITE' : 'CHECK',
    `tab2 PUTs=${JSON.stringify(t2calls)} url=${tab2Url} | finalName="${finalName}" rev=${finalRev} | body="${tab2Body.replace(/\n/g, ' | ').slice(0, 300)}"`,
  );
  await ctx.close();
}

// ===========================================================================
// Scenario 6 — long data (303-char display_name)
// ===========================================================================
{
  tag = 's6-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');

  // entity card
  tag = 's6-card';
  await page.goto(`${BASE}/entities/${LONG_ENTITY}`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(1500);
  let m = await page.evaluate(() => ({
    scrollW: document.documentElement.scrollWidth,
    clientW: document.documentElement.clientWidth,
    h1W: document.querySelector('h1')?.getBoundingClientRect().width ?? 0,
  }));
  rec(
    'S6 long name entity card 1440px',
    m.scrollW <= m.clientW + 2 ? 'PASS' : 'FAIL',
    JSON.stringify(m),
  );
  await page.screenshot({ path: join(SHOTS, 's6-card-1440.png') });
  const crumbsLong = await crumbs(page);
  rec('S6 breadcrumb long name', 'INFO', JSON.stringify(crumbsLong));

  await page.setViewportSize({ width: 400, height: 850 });
  await page.waitForTimeout(800);
  m = await page.evaluate(() => ({
    scrollW: document.documentElement.scrollWidth,
    clientW: document.documentElement.clientWidth,
  }));
  rec(
    'S6 long name entity card 400px',
    m.scrollW <= m.clientW + 2 ? 'PASS' : 'FAIL',
    JSON.stringify(m),
  );
  await page.screenshot({ path: join(SHOTS, 's6-card-400.png'), fullPage: true });
  await page.setViewportSize({ width: 1440, height: 900 });

  // list row
  tag = 's6-list';
  await page.goto(`${BASE}/objects/qa_item?catalog=qa_t`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(1500);
  m = await page.evaluate(() => {
    const table = document.querySelector('table');
    return {
      scrollW: document.documentElement.scrollWidth,
      clientW: document.documentElement.clientWidth,
      tableW: table?.getBoundingClientRect().width ?? 0,
      cellHasEllipsis: (() => {
        const td = table?.querySelector('tbody td');
        if (!td) return null;
        const cs = getComputedStyle(td);
        return { overflow: cs.overflow, textOverflow: cs.textOverflow, whiteSpace: cs.whiteSpace, maxW: cs.maxWidth };
      })(),
    };
  });
  rec('S6 list row long value 1440px', 'INFO', JSON.stringify(m));
  await page.screenshot({ path: join(SHOTS, 's6-list-1440.png') });
  await page.setViewportSize({ width: 400, height: 850 });
  await page.waitForTimeout(800);
  m = await page.evaluate(() => ({
    scrollW: document.documentElement.scrollWidth,
    clientW: document.documentElement.clientWidth,
  }));
  rec(
    'S6 list 400px',
    m.scrollW <= m.clientW + 2 ? 'PASS' : 'FAIL',
    JSON.stringify(m) + ' (table inside .overflow-x-auto wrapper => page-level scroll is the bug)',
  );
  await page.screenshot({ path: join(SHOTS, 's6-list-400.png'), fullPage: true });
  await ctx.close();
}

// ===========================================================================
// Scenario 7 — empty states (impossible filter + empty catalog type)
// ===========================================================================
{
  tag = 's7-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');

  tag = 's7-impossible-search';
  await page.goto(`${BASE}/objects/wh_zone?catalog=wh`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(1500);
  await page.locator('input[placeholder], input[type="text"]').first().fill('zzzqqq_no_match_98765');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(2000);
  let s = await snap(page);
  const loadMoreVisible = await page.locator('button:has-text("ещё"), button:has-text("more"), button:has-text("Load"), button:has-text("Загрузить")').count();
  const rowCount = await page.locator('tbody tr').count();
  rec(
    'S7 impossible search empty state',
    rowCount === 0 && loadMoreVisible === 0 ? 'PASS' : 'CHECK',
    `rows=${rowCount} loadMoreBtns=${loadMoreVisible} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
  );
  await page.screenshot({ path: join(SHOTS, 's7-empty-search.png') });

  // catalog with zero types (qa_empty)
  tag = 's7-empty-catalog';
  await page.goto(`${BASE}/objects?catalog=qa_empty`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1500);
  s = await snap(page);
  const cardCount = await page.locator('main a, [role="main"] a').count().catch(() => -1);
  rec(
    'S7 empty catalog /objects',
    'INFO',
    `links=${cardCount} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
  );
  await page.screenshot({ path: join(SHOTS, 's7-empty-catalog.png') });

  // wh_zone scoped to catalog where it does not exist
  tag = 's7-type-wrong-catalog';
  apiCalls.length = 0;
  await page.goto(`${BASE}/objects/wh_zone?catalog=qa_empty`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(2000);
  s = await snap(page);
  const failedCalls = callsSince('s7-type-wrong-catalog').filter((c) => c.status >= 400);
  const rows2 = await page.locator('tbody tr').count();
  const whiteScreen = s.body.trim().length < 20;
  rec(
    'S7 type alias in wrong catalog',
    !whiteScreen ? 'PASS' : 'FAIL',
    `rows=${rows2} apiErrors=${JSON.stringify(failedCalls)} whiteScreen=${whiteScreen} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
  );
  await page.screenshot({ path: join(SHOTS, 's7-wrong-catalog.png') });
  await ctx.close();
}

// ===========================================================================
// Scenario 8 — catalog switch mid-flow
// ===========================================================================
{
  tag = 's8-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');

  // pin catalog=wh via the selector's localStorage key, then load /objects
  tag = 's8-objects-wh';
  await page.evaluate(() => localStorage.setItem('enginer.catalog', 'wh'));
  await page.goto(`${BASE}/objects`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1500);
  const whCardCount = await page.locator('main a').count();
  // click wh_zone type card
  await page.locator('a:has-text("wh_zone")').first().click();
  await page.waitForTimeout(1800);
  tag = 's8-list-wh';
  const urlWh = page.url();
  const rowsWh = await page.locator('tbody tr').count();
  rec('S8 wh catalog -> wh_zone list', 'INFO', `url=${urlWh} rows=${rowsWh} cards=${whCardCount}`);

  // now switch the header catalog to qa_empty mid-list
  tag = 's8-switch-empty';
  apiCalls.length = 0;
  try {
    await pickHeaderSelect(page, 0, 'QA Empty');
  } catch (e) {
    rec('S8 header catalog select', 'CHECK', `interaction failed: ${e}`);
  }
  await page.waitForTimeout(2500);
  let s = await snap(page);
  const rowsAfter = await page.locator('tbody tr').count();
  const apiErrs = callsSince('s8-switch-empty').filter((c) => c.status >= 400);
  const white = s.body.trim().length < 20;
  rec(
    'S8 switch catalog wh->qa_empty mid-list',
    !white ? 'PASS' : 'FAIL',
    `url=${s.url} rows=${rowsAfter} apiErrs=${JSON.stringify(apiErrs)} whiteScreen=${white} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
  );
  await page.screenshot({ path: join(SHOTS, 's8-after-switch.png') });

  // navigate Back — expect wh list restored
  tag = 's8-back';
  await page.goBack().catch(() => {});
  await page.waitForTimeout(2000);
  s = await snap(page);
  const rowsBack = await page.locator('tbody tr').count();
  rec(
    'S8 Back after catalog switch',
    'INFO',
    `url=${s.url} rows=${rowsBack} | body="${s.body.slice(0, 200).replace(/\n/g, ' | ')}"`,
  );
  await ctx.close();
}

// ===========================================================================
// Scenario 9 — network failure (route interception)
// ===========================================================================
{
  tag = 's9-login';
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  wire(page);
  await login(page, 'admin');

  // abort entity GET -> entity card must show error+retry, not a white screen
  tag = 's9-abort-entity';
  await ctx.route('**/api/v1/entities/*', (r) => {
    if (r.request().url().includes('/entities/query')) return r.continue();
    return r.abort('failed');
  });
  await page.goto(`${BASE}/entities/${WH_ZONE_ENTITY}`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(2500);
  let s = await snap(page);
  const retryBtn = await page.locator('button:has-text("Повторить"), button:has-text("Retry"), button:has-text("повтор")').count();
  const white1 = s.body.trim().length < 20;
  rec(
    'S9 aborted entity fetch',
    !white1 && retryBtn > 0 ? 'PASS' : 'CHECK',
    `whiteScreen=${white1} retryBtns=${retryBtn} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
  );
  await page.screenshot({ path: join(SHOTS, 's9-entity-aborted.png') });
  await ctx.unroute('**/api/v1/entities/*');

  // abort the query endpoint on the list page
  tag = 's9-abort-query';
  await ctx.route('**/api/v1/entities/query*', (r) => r.abort('failed'));
  await page.goto(`${BASE}/objects/wh_zone?catalog=wh`, {
    waitUntil: 'networkidle',
  });
  await page.waitForTimeout(2500);
  s = await snap(page);
  const white2 = s.body.trim().length < 20;
  const retryBtn2 = await page.locator('button:has-text("Повторить"), button:has-text("Retry")').count();
  rec(
    'S9 aborted list query',
    !white2 ? 'PASS' : 'FAIL',
    `whiteScreen=${white2} retryBtns=${retryBtn2} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
  );
  await page.screenshot({ path: join(SHOTS, 's9-list-aborted.png') });

  // does the retry button recover once the network is back?
  await ctx.unroute('**/api/v1/entities/query*');
  if (retryBtn2 > 0) {
    await page.locator('button:has-text("Повторить"), button:has-text("Retry")').first().click();
    await page.waitForTimeout(2000);
    const rows = await page.locator('tbody tr').count();
    rec('S9 retry after recovery', rows > 0 ? 'PASS' : 'CHECK', `rows=${rows}`);
  }
  await ctx.close();
}

// ===========================================================================
// Scenario 10 — audit: console errors, 5xx, request floods
// ===========================================================================
{
  const fiveHundreds = apiCalls.filter((c) => c.status >= 500);
  const fourOhFours = apiCalls.filter((c) => c.status === 404);
  const byEndpoint = {};
  for (const c of apiCalls) {
    const k = `${c.method} ${c.url.split('?')[0]}`;
    byEndpoint[k] = (byEndpoint[k] ?? 0) + 1;
  }
  const floods = Object.entries(byEndpoint)
    .filter(([, n]) => n >= 10)
    .sort((a, b) => b[1] - a[1]);
  rec(
    'S10 5xx responses',
    fiveHundreds.length === 0 ? 'PASS' : 'FAIL',
    JSON.stringify(fiveHundreds.slice(0, 10)),
  );
  rec(
    'S10 404s',
    'INFO',
    `${fourOhFours.length} total | ${JSON.stringify(fourOhFours.slice(0, 8))}`,
  );
  rec(
    'S10 request floods (>=10 same endpoint)',
    floods.length === 0 ? 'PASS' : 'CHECK',
    floods.map(([k, n]) => `${k} x${n}`).join('; ') || 'none',
  );
  rec(
    'S10 console errors (all scenarios)',
    consoleErrs.length === 0 ? 'PASS' : 'CHECK',
    `${consoleErrs.length} errors | ${JSON.stringify(consoleErrs.slice(0, 15))}`,
  );
  rec(
    'S10 page errors (uncaught)',
    pageErrs.length === 0 ? 'PASS' : 'FAIL',
    JSON.stringify(pageErrs.slice(0, 10)),
  );
  rec(
    'S10 dialogs seen',
    'INFO',
    JSON.stringify(dialogs),
  );
}

console.log('\n==================== SUMMARY ====================');
for (const r of results) console.log(`${r.verdict.padEnd(22)} ${r.name}`);

await browser.close();
console.log('\nDONE');
