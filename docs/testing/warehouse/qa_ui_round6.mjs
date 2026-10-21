/**
 * qa_ui_round6.mjs — browser QA round 6: uncovered pages & flows.
 *
 *   /admin/dump         list, run, restore dialog, delete (confirm?), upload error
 *   /admin/schema       draft CRUD, validation findings, impact dialog, publish gating
 *   /admin/types        list, create dialog, invalid-alias feedback, edit form
 *   /admin/predicates   list, filters, create dialog, groups dialog, invalid alias
 *   /admin/catalogs     list, create, header-selector interplay, retire
 *   /integrations       list, enable/disable, health check, new-connector dialog, a11y
 *   /docs               renders hardcoded help content
 *   /search             unicode/RTL/special chars/emoji/XSS, empty query, click-through
 *   /objects/wh_zone/create  required-field errors, dedup dialog, draft save/restore
 *   /entities/[id]/edit      edit + save + revision bump
 *   keyboard nav        sidebar tab order, focus-visible, aria-labels on icon buttons
 *   viewports           400px + 2560px overflow on all pages above
 *   RTL                 locale he → dir=rtl on admin pages
 *
 * Cleanup: entities created here are retired (UI + API), drafts deleted,
 * the qa_r6 catalog retired, the qa_r6 dump deleted, connector re-enabled.
 *
 * Run:  node docs/testing/warehouse/qa_ui_round6.mjs   (repo root)
 */

import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..', '..');
const { chromium } = await import(
  'file:///' + join(ROOT, 'nuxt/node_modules/playwright-core/index.mjs').replace(/\\/g, '/')
);

const BASE = 'http://localhost:3000';
const API = 'http://localhost:8010/api/v1';
const SHOTS = join(HERE, 'qa_shots_round6');
mkdirSync(SHOTS, { recursive: true });

const TOKENS = JSON.parse(readFileSync(join(HERE, '.wh_tokens.json'), 'utf8'));
const ADMIN_TOKEN = TOKENS.admin;

// --- collectors ------------------------------------------------------------
const results = [];
const apiCalls = [];
const consoleErrs = [];
const pageErrs = [];
const dialogs = []; // {tag, message}
let tag = 'boot';

function rec(name, verdict, evidence = '') {
  results.push({ name, verdict, evidence });
  console.log(`${verdict.padEnd(5)} ${name} — ${evidence.split('\n')[0].slice(0, 220)}`);
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
    `consoleErr=${e.console.length}${e.console.length ? ' ' + JSON.stringify(e.console.slice(0, 2).map((x) => x.text.slice(0, 140))) : ''}` +
    ` pageErr=${e.page.length}${e.page.length ? ' ' + JSON.stringify(e.page.slice(0, 2).map((x) => x.text.slice(0, 160))) : ''}` +
    ` api>=400=${c.length}${c.length ? ' ' + JSON.stringify(c.slice(0, 5).map((x) => `${x.status} ${x.method} ${x.url}`)) : ''}`
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
  return { status: r.status, json: await r.json().catch(() => null), headers: r.headers };
}

async function apiRetire(publicId, reason = 'qa round6 cleanup') {
  return apiFetch(`/entities/${publicId}/retire`, {
    method: 'POST',
    body: JSON.stringify({ reason }),
  });
}

async function login(page, user, pass = 'enginer-demo') {
  await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
  await page.locator('input[name="username"], input[autocomplete="username"]').first().fill(user);
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
  page.on('dialog', (d) => {
    dialogs.push({ tag, message: d.message().slice(0, 120) });
    d.accept().catch(() => {});
  });
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

async function bodyLint(page) {
  const text = await page.locator('body').innerText().catch(() => '');
  const i18nKey = (text.match(/\b(?:nav|entity|requests|checks|risks|reports|audit|admin|catalog|users|schema|dump|common|search|errors|monitoring|integrations|docs|workflows|tasks|security|login|permissions|sections|fields|types)\.[a-zA-Z_][a-zA-Z0-9_.]*\b/g) ?? [])
    .filter((k) => !/\.(xz|log|json|csv|zip|gz|ndjson|ru|ts)$/i.test(k));
  const objObj = (text.match(/\[object Object\]/g) ?? []).length;
  const iso = text.match(/\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2})?(\.\d+)?([+-]\d{2}:?\d{2}|Z)?\b/g) ?? [];
  return {
    i18nKeys: [...new Set(i18nKey)],
    objObj,
    isoCount: iso.length,
    text,
  };
}

async function overflowInfo(page) {
  return page.evaluate(() => ({
    scrollW: Math.max(document.documentElement.scrollWidth, document.body?.scrollWidth ?? 0),
    innerW: window.innerWidth,
    overflow: Math.max(document.documentElement.scrollWidth, document.body?.scrollWidth ?? 0) > window.innerWidth + 1,
  }));
}

/** Icon-only buttons lacking any accessible name. */
async function unlabeledIconButtons(page) {
  return page.evaluate(() => {
    const out = [];
    for (const b of document.querySelectorAll('button')) {
      const txt = (b.textContent ?? '').replace(/\s+/g, ' ').trim();
      if (txt) continue;
      const name =
        b.getAttribute('aria-label') ||
        b.getAttribute('aria-labelledby') ||
        b.getAttribute('title') ||
        b.querySelector('svg,[class*="icon"]')?.getAttribute('aria-label');
      if (!name) {
        const cls = (b.querySelector('[class*="i-"]')?.getAttribute('class') ?? b.className ?? '').slice(0, 60);
        out.push(cls);
      }
    }
    return out;
  });
}

async function visit(page, url, name, { shot = true } = {}) {
  tag = name;
  await page.goto(url, { waitUntil: 'domcontentloaded' }).catch((e) => rec(name + ' goto', 'FAIL', String(e)));
  await settle(page);
  const lint = await bodyLint(page);
  const ov = await overflowInfo(page);
  const errs = errsFor(name);
  const bad = callsFor(name);
  const problems = [];
  if (errs.console.length) problems.push(`consoleErr=${errs.console.length}`);
  if (errs.page.length) problems.push(`pageErr=${errs.page.length}`);
  if (bad.length) problems.push(`api>=400=${bad.length}`);
  if (ov.overflow) problems.push(`h-overflow ${ov.scrollW}>${ov.innerW}`);
  if (lint.i18nKeys.length) problems.push(`i18n-keys:${lint.i18nKeys.slice(0, 5).join(',')}`);
  if (lint.objObj) problems.push(`[object Object]x${lint.objObj}`);
  if (shot) await page.screenshot({ path: join(SHOTS, `${name}.png`) });
  rec(name, problems.length ? 'CHECK' : 'PASS',
    `${url} | ${problems.join(' | ') || 'clean'} | ISO=${lint.isoCount}\n    ${errSummary(name)}`);
  return { lint, ov, errs, bad };
}

async function clickComboboxOption(page, combobox, optionText) {
  await combobox.click();
  await page.waitForTimeout(500);
  const opt = page.locator('[role="option"], [role="listbox"] [data-highlighted], [role="listbox"] li')
    .filter({ hasText: optionText }).first();
  const n = await opt.count();
  if (n) {
    await opt.click();
    await page.waitForTimeout(400);
  } else {
    await page.keyboard.press('Escape');
  }
  return n > 0;
}

const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, ignoreHTTPSErrors: true });
const page = await ctx.newPage();
wire(page);

tag = 'login';
await login(page, 'admin');
rec('login admin', 'PASS', `landed ${page.url()}`);

// track created artifacts for cleanup
let createdEntityPid = null;
let createdDistinctPid = null;
let qaDraftIds = [];

try {
// ===========================================================================
// 1. /admin/dump
// ===========================================================================
{
  await visit(page, `${BASE}/admin/dump`, 'dump-list');
  const rows = await page.locator('[data-testid="dumps-table"] tbody tr').count();
  const overflowGuard = await page.evaluate(() =>
    !!document.querySelector('[data-testid="dumps-table"]')?.closest('.overflow-x-auto'));
  rec('dump: table renders', rows >= 1 ? 'PASS' : 'CHECK', `rows=${rows} overflowXGuard=${overflowGuard}`);

  // restore confirmation dialog on a finished dump
  tag = 'dump-restore-dialog';
  const restoreBtn = page.locator('[data-testid^="restore-"]').first();
  if (await restoreBtn.count()) {
    await restoreBtn.click();
    await page.waitForTimeout(600);
    const dlg = page.locator('[role="dialog"]').last();
    const vis = await dlg.isVisible().catch(() => false);
    const dlgTxt = (await dlg.innerText().catch(() => '')).replace(/\s+/g, ' ');
    const hasConfirm = await dlg.locator('[data-testid="confirm-restore"]').count();
    await page.screenshot({ path: join(SHOTS, 'dump-restore-dialog.png') });
    rec('dump: restore confirmation dialog', vis && hasConfirm > 0 ? 'PASS' : 'FAIL',
      `visible=${vis} confirmBtn=${hasConfirm} warn="${dlgTxt.slice(0, 140)}"`);
    // cancel — never actually restore
    await dlg.locator('button').first().click().catch(() => {});
    await page.keyboard.press('Escape').catch(() => {});
    await page.waitForTimeout(400);
  } else rec('dump: restore confirmation dialog', 'INFO', 'no finished dump with payload');

  // upload wrong file type → error state
  tag = 'dump-upload-bad';
  await page.setInputFiles('input[type="file"]', {
    name: 'qa-notadump.txt', mimeType: 'text/plain', buffer: Buffer.from('definitely not a dump'),
  });
  await page.waitForTimeout(2500);
  const errAlert = await page.locator('[role="alert"], .text-error, [class*="error"]').first().isVisible().catch(() => false);
  const bodyT = await page.locator('body').innerText();
  const errShown = /error|ошиб|invalid|failed|not a/i.test(bodyT.slice(-2000)) || errAlert;
  rec('dump: bad-upload error state', errShown ? 'PASS' : 'CHECK',
    `alertVisible=${errAlert} | ${errSummary('dump-upload-bad')}`);
  await page.screenshot({ path: join(SHOTS, 'dump-upload-error.png') });

  // run a fresh dump, then delete it (no-confirm check) — also cleans up
  tag = 'dump-run';
  const idsBefore = new Set(((await apiFetch('/admin/dump')).json?.data ?? []).map((d) => d.dump_id));
  const apiBefore = apiCalls.length;
  await page.locator('[data-testid="run-dump"]').click();
  await page.waitForTimeout(1500);
  const runPost = apiCalls.slice(apiBefore).find((c) => c.method === 'POST' && /admin\/dump$/.test(c.url));
  rec('dump: run dump', runPost && runPost.status < 400 ? 'PASS' : 'CHECK',
    `POST status=${runPost?.status} | ${errSummary('dump-run')}`);
  // poll until a NEW dump id reaches a finished state
  let newDumpId = null;
  for (let i = 0; i < 45; i++) {
    const { json } = await apiFetch('/admin/dump');
    const fresh = (json?.data ?? []).find((d) => !idsBefore.has(d.dump_id));
    if (fresh && !['running', 'restoring'].includes(fresh.status)) { newDumpId = fresh.dump_id; break; }
    if (fresh?.status === 'failed') { newDumpId = fresh.dump_id; break; }
    await page.waitForTimeout(1500);
  }
  await page.reload({ waitUntil: 'domcontentloaded' });
  await settle(page, 1000);
  if (newDumpId) {
    tag = 'dump-delete';
    const delBtn = page.locator(`[data-testid="delete-${newDumpId}"]`);
    const dlgBefore = dialogs.length;
    if (await delBtn.count()) {
      await delBtn.click();
      await page.waitForTimeout(1500);
      const askedConfirm = dialogs.length > dlgBefore;
      const stillThere = await page.locator(`[data-testid="delete-${newDumpId}"]`).count();
      rec('dump: delete dump', stillThere === 0 ? 'PASS' : 'CHECK',
        `deleted=${stillThere === 0} confirmDialog=${askedConfirm} (delete has NO confirmation — UX inconsistency w/ restore) | ${errSummary('dump-delete')}`);
    } else {
      rec('dump: delete dump', 'CHECK', `delete button for ${newDumpId} not found`);
      await apiFetch(`/admin/dump/${newDumpId}`, { method: 'DELETE' }).catch(() => {});
    }
  } else {
    rec('dump: run->done poll', 'CHECK', 'no finished dump within 60s');
    await page.screenshot({ path: join(SHOTS, 'dump-run-pending.png') });
  }
  const unlabeled = await unlabeledIconButtons(page);
  rec('dump: icon-button a11y names', unlabeled.length === 0 ? 'PASS' : 'FAIL',
    `${unlabeled.length} icon-only buttons without name: ${JSON.stringify(unlabeled.slice(0, 4))}`);
}

// ===========================================================================
// 2. /admin/schema
// ===========================================================================
{
  await visit(page, `${BASE}/admin/schema`, 'schema-list');
  const draftArticles = await page.locator('article').count();
  rec('schema: drafts render', draftArticles >= 1 ? 'PASS' : 'CHECK', `articles=${draftArticles}`);

  // --- create dialog empty-submit gating ---
  tag = 'schema-create-gating';
  await page.locator('header button:has([class*="i-lucide:plus"]), main > div button:has([class*="i-lucide:plus"])').first().click();
  await page.waitForTimeout(600);
  let dlg = page.locator('[role="dialog"]').last();
  let vis = await dlg.isVisible().catch(() => false);
  if (!vis) {
    // header may be outside 'main'
    await page.locator('button:has([class*="i-lucide:plus"])').first().click();
    await page.waitForTimeout(600);
    dlg = page.locator('[role="dialog"]').last();
    vis = await dlg.isVisible().catch(() => false);
  }
  if (vis) {
    const submit = dlg.locator('button[type="submit"]').last();
    const disabled0 = await submit.evaluate((b) => b.disabled || b.getAttribute('aria-disabled') === 'true').catch(() => null);
    // invalid semver → still disabled
    await dlg.locator('input').nth(0).fill('QA round6 valid draft');
    const verInputs = await dlg.locator('input').all();
    // name is first input; version field — fill bad semver on the second grid input
    let badVer = false;
    for (const inp of verInputs) {
      const ph = (await inp.getAttribute('placeholder')) ?? '';
      const val = await inp.inputValue().catch(() => '');
      if (ph === '1.1.0' || /^\d+\.\d+\.\d+$/.test(val)) {
        await inp.fill('not.a.version');
        await page.waitForTimeout(200);
        badVer = await submit.evaluate((b) => b.disabled || b.getAttribute('aria-disabled') === 'true').catch(() => false);
        await inp.fill(`9.${Date.now() % 90 + 10}.0`);
        break;
      }
    }
    await page.screenshot({ path: join(SHOTS, 'schema-create-dialog.png') });
    rec('schema: create-draft gating', disabled0 === true && badVer === true ? 'PASS' : 'CHECK',
      `emptyDisabled=${disabled0} badSemverDisabled=${badVer}`);
    // create draft B via UI — NOTE: the dialog defaults base_revision to the
    // first PUBLISHED revision across ALL catalog streams; a base from
    // another catalog's stream is rejected by the server (400).
    const apiBefore = apiCalls.length;
    await submit.click();
    await page.waitForTimeout(2500);
    const post = apiCalls.slice(apiBefore).find((c) => c.method === 'POST' && /schemas\/drafts/.test(c.url));
    const alertTxt = (await dlg.locator('[role="alert"]').innerText().catch(() => '')).replace(/\s+/g, ' ');
    rec('schema: create draft via dialog (default base_revision)', post && post.status < 400 ? 'PASS' : 'FAIL',
      `POST=${post?.status} dialogError="${alertTxt.slice(0, 160)}" — dialog defaults base_revision to a revision from ANOTHER catalog stream → 400`);
    await page.screenshot({ path: join(SHOTS, 'schema-create-error.png') });
    await page.keyboard.press('Escape').catch(() => {});
    await page.waitForTimeout(400);
    if (await dlg.isVisible().catch(() => false)) {
      await dlg.locator('button').first().click().catch(() => {});
      await page.waitForTimeout(300);
    }
    // fallback: create draft B via API on the DEFAULT stream's published
    // revision (general catalog stream — revision 1, 'SKR Core 1.0')
    const { json: pubJ } = await apiFetch('/schemas/published?catalog=general');
    const genBase = (pubJ?.data ?? []).find((r) => r.status === 'published')
      ?? (pubJ?.data ?? [])[0];
    const cr = await apiFetch('/schemas/drafts', {
      method: 'POST',
      body: JSON.stringify({
        name: 'QA round6 valid draft', description: 'validate+impact probe',
        semantic_version: `9.${Date.now() % 80 + 10}.0`,
        ...(genBase ? { base_revision: genBase.revision_id } : {}),
      }),
    });
    rec('schema: create valid draft (API)', cr.status < 400 ? 'PASS' : 'FAIL',
      `status=${cr.status} base=${genBase?.revision_id} ${JSON.stringify(cr.json).slice(0, 160)}`);
    await page.waitForTimeout(500);
  } else {
    rec('schema: create dialog opens', 'FAIL', 'no [role=dialog]');
  }

  // remember created draft ids for cleanup
  {
    const { json } = await apiFetch('/schemas/drafts');
    qaDraftIds = (json?.data ?? [])
      .filter((d) => /QA round6/i.test(d.display_name ?? ''))
      .map((d) => d.revision_id);
  }

  // --- Draft A: invalid types → validation findings dialog ---
  tag = 'schema-invalid-draft';
  {
    // create draft A directly via API (same as UI path would)
    const cr = await apiFetch('/schemas/drafts', {
      method: 'POST',
      body: JSON.stringify({ name: 'QA round6 INVALID draft', description: 'validation findings probe', semantic_version: '0.9.1' }),
    });
    const draftA = cr.json?.data?.revision_id;
    if (draftA) {
      qaDraftIds.push(draftA);
      // corrupt it: types reference nonexistent predicates
      const full = await apiFetch(`/schemas/drafts/${draftA}`);
      const d = full.json?.data ?? {};
      await apiFetch(`/schemas/drafts/${draftA}`, {
        method: 'PUT',
        headers: { 'If-Match': d.etag ?? '*' },
        body: JSON.stringify({
          display_name: 'QA round6 INVALID draft',
          description: 'validation findings probe',
          semantic_version: d.semantic_version ?? '0.9.1',
          types: [
            { type_id: 'qa_r6_bogus', stable_alias: 'qa_r6_bogus', entity_class: 'data', label: 'QA bogus', fields: [{ alias: 'no_such_pred', kind: 'text', required: true }] },
            { type_id: 'qa_r6_bogus2', stable_alias: 'qa_r6_bogus', entity_class: 'data', label: 'QA dup alias', fields: [] },
          ],
          predicates: [
            { stable_alias: 'qa_r6_edge', label: 'QA broken edge', is_edge: true, domain: 'no_such_type_zzz', range: 'also_missing_type', direction: 'directed', min_cardinality: 0, max_cardinality: 1 },
          ],
          relation_groups: d.relation_groups ?? [],
          changes: [{ kind: 'add', resource_kind: 'type', resource_id: 'qa_r6_bogus', summary: 'bogus type', breaking: false }],
        }),
      });
      await page.reload({ waitUntil: 'domcontentloaded' });
      await settle(page, 1200);
      // find the article for our draft and click Validate
      const art = page.locator('article').filter({ hasText: 'QA round6 INVALID' }).first();
      if (await art.count()) {
        const apiBefore = apiCalls.length;
        await art.locator('button:has([class*="i-lucide:check-circle"])').first().click();
        await page.waitForTimeout(2500);
        const vdlg = page.locator('[role="dialog"]').last();
        const vvis = await vdlg.isVisible().catch(() => false);
        const vtxt = (await vdlg.innerText().catch(() => '')).replace(/\s+/g, ' ');
        const findings = await vdlg.locator('code').count();
        await page.screenshot({ path: join(SHOTS, 'schema-validation-findings.png') });
        const valPost = apiCalls.slice(apiBefore).find((c) => /validate/.test(c.url));
        const reportedInvalid = /ошиб|error|fail|не пройдена|непройдена|invalid/i.test(vtxt);
        rec('schema: validation findings dialog', vvis && reportedInvalid ? 'PASS' : 'FAIL',
          `dialog=${vvis} findings=${findings} POST=${valPost?.status} — draft contains unknown predicates, dup alias, broken edge domain; server said: "${vtxt.slice(0, 160)}"`);
        await vdlg.locator('button').last().click().catch(() => {});
        await page.keyboard.press('Escape').catch(() => {});
        await page.waitForTimeout(400);
        // publish button must be disabled for this draft
        await page.reload({ waitUntil: 'domcontentloaded' });
        await settle(page, 1000);
        const art2 = page.locator('article').filter({ hasText: 'QA round6 INVALID' }).first();
        const pubBtn = art2.locator('button:has([class*="i-lucide:rocket"])').first();
        const pubDisabled = await pubBtn.evaluate((b) => b.disabled || b.getAttribute('aria-disabled') === 'true').catch(() => null);
        rec('schema: publish gated for invalid draft', pubDisabled === true ? 'PASS' : 'FAIL',
          `publishDisabled=${pubDisabled}`);
      } else rec('schema: invalid draft visible', 'FAIL', 'article not found after reload');
    } else rec('schema: create invalid draft', 'FAIL', `POST /schemas/drafts → ${cr.status} ${JSON.stringify(cr.json).slice(0, 200)}`);
  }

  // --- Draft B: validate + impact dialogs on a clean draft ---
  tag = 'schema-valid-draft';
  {
    const { json } = await apiFetch('/schemas/drafts');
    const draftB = (json?.data ?? []).find((d) => /QA round6 valid draft/i.test(d.display_name ?? ''));
    if (draftB) {
      const art = page.locator('article').filter({ hasText: 'QA round6 valid draft' }).first();
      if (await art.count()) {
        await art.locator('button:has([class*="i-lucide:check-circle"])').first().click();
        await page.waitForTimeout(2500);
        let vdlg = page.locator('[role="dialog"]').last();
        let vtxt = (await vdlg.innerText().catch(() => '')).replace(/\s+/g, ' ');
        await page.screenshot({ path: join(SHOTS, 'schema-validate-clean.png') });
        const valid = /пройдена|passed|valid|успеш/i.test(vtxt) && !/не пройдена|failed/i.test(vtxt);
        rec('schema: clean draft validates', valid ? 'PASS' : 'CHECK', `"${vtxt.slice(0, 160)}"`);
        await vdlg.locator('button').last().click().catch(() => {});
        await page.waitForTimeout(400);
        // now run impact (status should be 'validating' + valid)
        await page.reload({ waitUntil: 'domcontentloaded' });
        await settle(page, 1000);
        const art2 = page.locator('article').filter({ hasText: 'QA round6 valid draft' }).first();
        const impBtn = art2.locator('button:has([class*="i-lucide:git-compare"])').first();
        const impDisabled = await impBtn.evaluate((b) => b.disabled).catch(() => true);
        if (!impDisabled) {
          await impBtn.click();
          await page.waitForTimeout(2500);
          const idlg = page.locator('[role="dialog"]').last();
          const ivis = await idlg.isVisible().catch(() => false);
          const itxt = (await idlg.innerText().catch(() => '')).replace(/\s+/g, ' ');
          await page.screenshot({ path: join(SHOTS, 'schema-impact-dialog.png') });
          rec('schema: impact report dialog', ivis && /\d/.test(itxt) ? 'PASS' : 'CHECK',
            `visible=${ivis} "${itxt.slice(0, 180)}"`);
          await idlg.locator('button').last().click().catch(() => {});
          await page.waitForTimeout(400);
        } else rec('schema: impact report dialog', 'CHECK', 'impact button still disabled after validate');
        // detail dialog (JSON "diff-ish" view)
        const viewBtn = art2.locator('button:has([class*="i-lucide:eye"])').first();
        await viewBtn.click().catch(() => {});
        await page.waitForTimeout(1200);
        const ddlg = page.locator('[role="dialog"]').last();
        const dvis = await ddlg.isVisible().catch(() => false);
        const hasJson = await ddlg.locator('details pre').count();
        await page.screenshot({ path: join(SHOTS, 'schema-detail-dialog.png') });
        rec('schema: detail dialog (types/predicates JSON)', dvis && hasJson >= 2 ? 'PASS' : 'CHECK',
          `visible=${dvis} jsonBlocks=${hasJson} (no dedicated diff view — JSON only)`);
        await page.keyboard.press('Escape');
        await page.waitForTimeout(400);
      } else rec('schema: valid draft visible', 'FAIL', 'article not found');
    } else rec('schema: valid draft exists', 'CHECK', 'draft B not found via API');
  }

  // --- delete QA drafts via UI ---
  tag = 'schema-delete-drafts';
  {
    const { json } = await apiFetch('/schemas/drafts');
    const mine = (json?.data ?? []).filter((d) => /QA round6/i.test(d.display_name ?? ''));
    for (const d of mine) {
      const art = page.locator('article').filter({ hasText: d.display_name }).first();
      if (await art.count()) {
        await art.locator('button:has([class*="i-lucide:trash-2"])').first().click();
        await page.waitForTimeout(700);
        const del = page.locator('[role="dialog"]').last();
        const delVis = await del.isVisible().catch(() => false);
        if (delVis) {
          await del.locator('button').last().click().catch(() => {});
          await page.waitForTimeout(1200);
        }
        const still = await page.locator('article').filter({ hasText: d.display_name }).count();
        rec(`schema: delete draft "${d.display_name}"`, still === 0 ? 'PASS' : 'FAIL',
          `confirmDialog=${delVis} stillListed=${still}`);
      } else {
        // fallback API delete
        const full = await apiFetch(`/schemas/drafts/${d.revision_id}`);
        await apiFetch(`/schemas/drafts/${d.revision_id}`, {
          method: 'DELETE', headers: { 'If-Match': full.json?.data?.etag ?? '*' },
        });
        rec(`schema: delete draft "${d.display_name}"`, 'CHECK', 'deleted via API fallback');
      }
      qaDraftIds = qaDraftIds.filter((x) => x !== d.revision_id);
    }
  }

  // published tab
  tag = 'schema-published-tab';
  {
    const tabsList = page.locator('[role="tab"]');
    const nTabs = await tabsList.count();
    if (nTabs >= 2) {
      await tabsList.nth(1).click();
      await page.waitForTimeout(1200);
      const revs = await page.locator('article').count();
      const txt = await page.locator('body').innerText();
      await page.screenshot({ path: join(SHOTS, 'schema-published.png') });
      rec('schema: published tab', revs >= 1 ? 'PASS' : 'CHECK', `revisions=${revs}`);
    } else rec('schema: published tab', 'FAIL', `tabs=${nTabs}`);
  }
}

// ===========================================================================
// 3. /admin/types + /admin/predicates
// ===========================================================================
{
  await visit(page, `${BASE}/admin/types`, 'types-list');
  const typeCards = await page.locator('main [class*="rounded"], main .divide-y, main > div > div').count();

  // create dialog — open, check fields, invalid alias feedback
  tag = 'types-create-dialog';
  await page.locator('button:has([class*="i-lucide:plus"])').first().click();
  await page.waitForTimeout(700);
  let dlg = page.locator('[role="dialog"]').last();
  const vis = await dlg.isVisible().catch(() => false);
  if (vis) {
    const fieldsN = await dlg.locator('input, textarea, [role="combobox"], [role="switch"]').count();
    await page.screenshot({ path: join(SHOTS, 'types-create-dialog.png') });
    rec('types: create dialog renders', fieldsN >= 4 ? 'PASS' : 'CHECK', `fields=${fieldsN}`);
    // invalid alias → save → expect error toast (schema.validation_failed path)
    await dlg.locator('input').first().fill('QA BAD!!');
    const apiBefore = apiCalls.length;
    const toastsBefore = await page.locator('[role="status"], [data-sonner-toast], section [class*="toast"]').count();
    await dlg.locator('button').last().click(); // Save
    await page.waitForTimeout(4000);
    const draftPosts = apiCalls.slice(apiBefore).filter((c) => /schemas\/drafts/.test(c.url));
    const bodyTxt = await page.locator('body').innerText();
    const m = bodyTxt.match(/.{0,60}(validation_failed|ошиб\w*|error).{0,80}/is);
    const errToast = !!m;
    const dlgStillOpen = await dlg.isVisible().catch(() => false);
    await page.screenshot({ path: join(SHOTS, 'types-invalid-alias.png') });
    rec('types: invalid alias feedback', errToast ? 'PASS' : 'CHECK',
      `dialogOpen=${dlgStillOpen} errToast=${errToast} toast="${(m?.[0] ?? '').replace(/\s+/g, ' ').slice(0, 120)}" schemaCalls=${draftPosts.length} | ${errSummary('types-create-dialog')}`);
    // close dialog
    await page.keyboard.press('Escape');
    await page.waitForTimeout(400);
    if (await dlg.isVisible().catch(() => false)) {
      await dlg.locator('button').first().click().catch(() => {});
      await page.waitForTimeout(300);
    }
    // cleanup residue draft from failed publish (also catches a publish of
    // the bogus type — that would surface as leftover evidence instead)
    const { json: djson } = await apiFetch('/schemas/drafts');
    for (const d of (djson?.data ?? []).filter((x) => /QA BAD/.test(x.display_name ?? ''))) {
      const full = await apiFetch(`/schemas/drafts/${d.revision_id}`);
      await apiFetch(`/schemas/drafts/${d.revision_id}`, {
        method: 'DELETE', headers: { 'If-Match': full.json?.data?.etag ?? '*' },
      });
      rec('types: cleanup bad-alias draft', 'PASS', `deleted draft ${d.revision_id}`);
    }
    // did the bogus alias actually get published? (that would be residue + a
    // missing server-side validation finding)
    const { json: tj } = await apiFetch('/schemas/types');
    const leaked = (tj?.data ?? []).filter((t) => /QA BAD/i.test(t.id ?? t.type_id ?? ''));
    if (leaked.length) {
      rec('types: invalid alias NOT validated server-side', 'FAIL',
        `type published: ${JSON.stringify(leaked.map((t) => t.id ?? t.type_id))} — needs manual cleanup`);
    }
  } else rec('types: create dialog opens', 'FAIL', 'no [role=dialog]');

  // edit dialog on existing type
  tag = 'types-edit-dialog';
  const editBtn = page.locator('main button').filter({ hasText: /Изменить|Редактировать|Edit|Bearbeiten|Modifier|Editar|编辑|تحرير|ערוך/ }).first();
  if (await editBtn.count()) {
    await editBtn.click();
    await page.waitForTimeout(700);
    const edlg = page.locator('[role="dialog"]').last();
    const evis = await edlg.isVisible().catch(() => false);
    const fields = await edlg.locator('input, [role="combobox"], [role="switch"]').count();
    const disabledAlias = await edlg.locator('input').first().evaluate((i) => i.disabled).catch(() => null);
    await page.screenshot({ path: join(SHOTS, 'types-edit-dialog.png') });
    rec('types: edit dialog renders fields', evis && fields >= 5 ? 'PASS' : 'CHECK',
      `visible=${evis} fields=${fields} aliasDisabled=${disabledAlias}`);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(400);
  } else rec('types: edit dialog', 'CHECK', 'no edit button found');
  rec('types: retired display', 'INFO', 'listTypes returns published-only; retired types never reach the list');

  // --- predicates ---
  await visit(page, `${BASE}/admin/predicates`, 'predicates-list');
  const predCards = await page.locator('main [class*="rounded-lg"], main section > div, main .space-y-2 > div').count();
  rec('predicates: list renders', 'INFO', `cards≈${predCards} | ${errSummary('predicates-list')}`);

  tag = 'predicates-groups';
  let gvis = false;
  for (let attempt = 0; attempt < 3 && !gvis; attempt++) {
    // scope to main — the header catalog select also carries i-lucide-layers
    await page.locator('main button:has([class*="i-lucide:layers"])').first().click().catch(() => {});
    await page.waitForTimeout(1200);
    gvis = await page.locator('[role="dialog"]').last().isVisible().catch(() => false);
  }
  const gdlg = page.locator('[role="dialog"]').last();
  const gRows = await gdlg.locator('input').count();
  await page.screenshot({ path: join(SHOTS, 'predicates-groups.png') });
  rec('predicates: groups dialog', gvis && gRows >= 3 ? 'PASS' : 'CHECK', `visible=${gvis} inputs=${gRows}`);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(400);

  tag = 'predicates-create-dialog';
  await page.locator('button:has([class*="i-lucide:plus"])').first().click();
  await page.waitForTimeout(700);
  const pdlg = page.locator('[role="dialog"]').last();
  const pvis = await pdlg.isVisible().catch(() => false);
  if (pvis) {
    const combos = await pdlg.locator('[role="combobox"], select').count();
    await page.screenshot({ path: join(SHOTS, 'predicates-create-dialog.png') });
    // invalid alias feedback
    await pdlg.locator('input').first().fill('BAD ALIAS!!');
    const apiBefore = apiCalls.length;
    await pdlg.locator('button').last().click();
    await page.waitForTimeout(4000);
    const draftPosts = apiCalls.slice(apiBefore).filter((c) => /schemas\/drafts/.test(c.url));
    const bodyTxt = await page.locator('body').innerText();
    const pm = bodyTxt.match(/.{0,60}(validation_failed|ошиб\w*|error).{0,80}/is);
    const errToast = !!pm;
    await page.screenshot({ path: join(SHOTS, 'predicates-invalid-alias.png') });
    rec('predicates: invalid alias feedback', errToast ? 'PASS' : 'CHECK',
      `errToast=${errToast} toast="${(pm?.[0] ?? '').replace(/\s+/g, ' ').slice(0, 120)}" schemaCalls=${draftPosts.length} combos=${combos}`);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(400);
    const { json: djson } = await apiFetch('/schemas/drafts');
    for (const d of (djson?.data ?? []).filter((x) => /BAD ALIAS/.test(x.display_name ?? ''))) {
      const full = await apiFetch(`/schemas/drafts/${d.revision_id}`);
      await apiFetch(`/schemas/drafts/${d.revision_id}`, {
        method: 'DELETE', headers: { 'If-Match': full.json?.data?.etag ?? '*' },
      });
      rec('predicates: cleanup bad-alias draft', 'PASS', `deleted draft ${d.revision_id}`);
    }
  } else rec('predicates: create dialog opens', 'FAIL', 'no [role=dialog]');
}

// ===========================================================================
// 4. /admin/catalogs
// ===========================================================================
const CAT_ALIAS = `qar6${Date.now() % 100000}`;
const CAT_NAME = `QA R6 Catalog ${CAT_ALIAS}`;
{
  await visit(page, `${BASE}/admin/catalogs`, 'catalogs-list');

  // create dialog
  tag = 'catalogs-create';
  await page.locator('button:has([class*="i-lucide:plus"])').first().click();
  await page.waitForTimeout(700);
  let cdlg = page.locator('[role="dialog"]').last();
  let cvis = await cdlg.isVisible().catch(() => false);
  if (cvis) {
    const ins = cdlg.locator('input');
    await ins.nth(0).fill(CAT_ALIAS);
    await ins.nth(1).fill(CAT_NAME);
    const apiBefore = apiCalls.length;
    await cdlg.locator('button').last().click();
    await page.waitForTimeout(2000);
    const post = apiCalls.slice(apiBefore).find((c) => c.method === 'POST' && /catalogs/.test(c.url));
    await page.screenshot({ path: join(SHOTS, 'catalogs-created.png') });
    rec('catalogs: create catalog', post && post.status < 400 ? 'PASS' : 'FAIL',
      `POST=${post?.status} | ${errSummary('catalogs-create')}`);
  } else rec('catalogs: create dialog opens', 'FAIL', 'no [role=dialog]');

  // header selector interplay — does the new catalog appear without reload?
  tag = 'catalogs-selector';
  const catCombo = page.locator('header [role="combobox"][aria-label]').first();
  if (await catCombo.count()) {
    const foundBefore = await clickComboboxOption(page, catCombo, CAT_NAME);
    if (!foundBefore) {
      await page.reload({ waitUntil: 'domcontentloaded' });
      await settle(page, 1000);
      const foundAfter = await clickComboboxOption(page, catCombo, CAT_NAME);
      rec('catalogs: header selector lists new catalog', foundAfter ? 'CHECK' : 'CHECK',
        `immediate=${foundBefore} afterReload=${foundAfter} (selector only refreshes via /me/context → needs reload)`);
      if (foundAfter) {
        // verify selection stuck: API calls carry ?catalog=<alias>
        const apiBefore = apiCalls.length;
        await page.goto(`${BASE}/objects`, { waitUntil: 'domcontentloaded' });
        await settle(page, 1200);
        const scoped = apiCalls.slice(apiBefore).filter((c) => c.url.includes(`catalog=${CAT_ALIAS}`));
        rec('catalogs: selector scopes API calls', scoped.length > 0 ? 'PASS' : 'CHECK',
          `callsWithCatalog=${scoped.length} of ${apiCalls.length - apiBefore}`);
        // back to all catalogs
        await page.goto(`${BASE}/admin/catalogs`, { waitUntil: 'domcontentloaded' });
        await settle(page, 800);
        const combo2 = page.locator('header [role="combobox"][aria-label]').first();
        await clickComboboxOption(page, combo2, /Все каталоги|All catalogs/);
      }
    } else {
      rec('catalogs: header selector lists new catalog', 'PASS', 'found without reload');
      await clickComboboxOption(page, catCombo, /Все каталоги|All catalogs/);
    }
  } else rec('catalogs: header selector', 'CHECK', 'no catalog combobox (single accessible catalog?)');

  // retire the QA catalog via UI (confirm() → auto-accepted + recorded)
  tag = 'catalogs-retire';
  await page.goto(`${BASE}/admin/catalogs`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1000);
  const qaCard = page.locator('main .grid > div').filter({ hasText: CAT_ALIAS }).first();
  const retBtn = qaCard.locator('button:has([class*="i-lucide:archive"])').first();
  if (await retBtn.count()) {
    const dlgBefore = dialogs.length;
    await retBtn.click();
    await page.waitForTimeout(2000);
    const asked = dialogs.length > dlgBefore;
    const badge = (await qaCard.innerText().catch(() => '')).replace(/\s+/g, ' ');
    const retired = /retired|погашен/i.test(badge) || !(await retBtn.count());
    await page.screenshot({ path: join(SHOTS, 'catalogs-retired.png') });
    rec('catalogs: retire flow (confirm + badge)', asked && retired ? 'PASS' : 'CHECK',
      `confirmDialog=${asked} retiredShown=${retired} badge="${badge.slice(0, 120)}"`);
    // selector should drop retired catalog
    await page.reload({ waitUntil: 'domcontentloaded' });
    await settle(page, 1000);
    const combo3 = page.locator('header [role="combobox"][aria-label]').first();
    const stillListed = await clickComboboxOption(page, combo3, CAT_NAME);
    rec('catalogs: retired catalog leaves selector', !stillListed ? 'PASS' : 'CHECK',
      `stillListed=${stillListed}`);
  } else rec('catalogs: retire flow', 'CHECK', `${CAT_ALIAS} card/retire button not found`);
  // make sure no catalog selection leaks into later sections
  await page.evaluate(() => localStorage.removeItem('enginer.catalog'));
}

// ===========================================================================
// 5. /integrations
// ===========================================================================
{
  await visit(page, `${BASE}/integrations`, 'integrations-list');
  const connRows = await page.locator('tbody tr').count();
  rec('integrations: connector list', connRows >= 1 ? 'PASS' : 'CHECK', `rows=${connRows}`);

  // health check on first connector
  tag = 'integrations-health';
  const rowBtns = page.locator('tbody tr').first().locator('button');
  const healthBtn = rowBtns.nth(0);
  if (await healthBtn.count()) {
    const apiBefore = apiCalls.length;
    await healthBtn.click();
    await page.waitForTimeout(3000);
    const healthPost = apiCalls.slice(apiBefore).find((c) => /connectors\/.+\/health/.test(c.url));
    const bodyTxt = await page.locator('body').innerText();
    const toast = /здоров|health|недоступ|unreachable|error|ошиб/i.test(bodyTxt.slice(-1500));
    await page.screenshot({ path: join(SHOTS, 'integrations-health.png') });
    rec('integrations: health check', healthPost ? 'PASS' : 'CHECK',
      `POST status=${healthPost?.status} toastShown=${toast} | ${errSummary('integrations-health')}`);
  } else rec('integrations: health check', 'CHECK', 'no row buttons');

  // disable then re-enable first connector
  tag = 'integrations-toggle';
  const toggleBtn = page.locator('tbody tr').first().locator('button').nth(1);
  if (await toggleBtn.count()) {
    const apiBefore = apiCalls.length;
    await toggleBtn.click();
    await page.waitForTimeout(2000);
    const disPost = apiCalls.slice(apiBefore).find((c) => /connectors\/.+\/(dis|en)able/.test(c.url));
    const badgeAfter = (await page.locator('tbody tr').first().innerText()).replace(/\s+/g, ' ');
    await page.screenshot({ path: join(SHOTS, 'integrations-disabled.png') });
    const flipped = /отключ|disabled/i.test(badgeAfter) || /enable/i.test(disPost?.url ?? '');
    rec('integrations: toggle off', disPost && disPost.status < 400 ? 'PASS' : 'CHECK',
      `${disPost?.method} ${disPost?.url} → ${disPost?.status} badge="${badgeAfter.slice(0, 120)}"`);
    // toggle back
    const apiBefore2 = apiCalls.length;
    await page.locator('tbody tr').first().locator('button').nth(1).click();
    await page.waitForTimeout(2000);
    const enPost = apiCalls.slice(apiBefore2).find((c) => /connectors\/.+\/(dis|en)able/.test(c.url));
    rec('integrations: toggle back on', enPost && enPost.status < 400 ? 'PASS' : 'CHECK',
      `${enPost?.url} → ${enPost?.status}`);
  } else rec('integrations: toggle', 'CHECK', 'no toggle button');

  // new connector dialog
  tag = 'integrations-dialog';
  await page.locator('button:has([class*="i-lucide:plus"])').first().click();
  await page.waitForTimeout(700);
  const idlg = page.locator('[role="dialog"]').last();
  const ivis = await idlg.isVisible().catch(() => false);
  const ifields = await idlg.locator('input, [role="combobox"]').count();
  await page.screenshot({ path: join(SHOTS, 'integrations-dialog.png') });
  rec('integrations: new-connector dialog', ivis && ifields >= 3 ? 'PASS' : 'CHECK',
    `visible=${ivis} fields=${ifields}`);
  await page.keyboard.press('Escape');
  await page.waitForTimeout(400);

  const unlabeled = await unlabeledIconButtons(page);
  rec('integrations: icon-button a11y names', unlabeled.length === 0 ? 'PASS' : 'FAIL',
    `${unlabeled.length} unnamed icon buttons ${JSON.stringify(unlabeled.slice(0, 4))}`);
}

// ===========================================================================
// 6. /docs
// ===========================================================================
{
  const { lint } = await visit(page, `${BASE}/docs`, 'docs');
  const is404 = /404|not found|не найден/i.test(lint.text.slice(0, 400));
  const sections = await page.locator('main ul button, main li button').count();
  rec('docs: renders (not 404)', !is404 && sections > 0 ? 'PASS' : 'FAIL', `docLinks=${sections}`);
  tag = 'docs-open';
  const firstDoc = page.locator('main ul button, main li button').first();
  if (await firstDoc.count()) {
    await firstDoc.click();
    await page.waitForTimeout(500);
    const contentLen = (await page.locator('main .prose, main [class*="prose"]').innerText().catch(() => '')).length;
    await page.screenshot({ path: join(SHOTS, 'docs-content.png') });
    rec('docs: item renders content', contentLen > 100 ? 'PASS' : 'CHECK', `contentLen=${contentLen}`);
  }
  const hebCheck = /Быстрый старт|Руководство/.test(lint.text);
  rec('docs: locale coverage', 'INFO', `hardcoded RU content (not localized): ${hebCheck}`);
}

// ===========================================================================
// 7. /search — unicode / special chars / XSS / empty
// ===========================================================================
{
  await visit(page, `${BASE}/search`, 'search-empty');
  const inp = page.locator('main input').first();

  const probes = [
    { q: 'Ромашка', name: 'cyrillic-hit' },
    { q: '%22', name: 'pct-encoded-quote' },
    { q: '<script>alert(1)</script>', name: 'xss' },
    { q: 'שלום עולם', name: 'hebrew-rtl' },
    { q: 'العربية نص', name: 'arabic-rtl' },
    { q: '🔥📦🏭', name: 'emoji' },
    { q: 'Ива́нов', name: 'combining-char' },
    { q: "O'Brien \"quoted\" \\backslash", name: 'quotes' },
  ];
  for (const p of probes) {
    tag = `search-${p.name}`;
    await page.goto(`${BASE}/search?q=${encodeURIComponent(p.q)}`, { waitUntil: 'domcontentloaded' });
    await settle(page, 1300);
    const hits = await page.locator('a[href^="/entities/"]').count();
    const errs = errsFor(tag);
    const bad = callsFor(tag);
    // XSS check: no injected script nodes / no dialog popped
    const injected = await page.evaluate(() => {
      const main = document.querySelector('main');
      return !!(main && main.querySelector('script'));
    });
    const problem = errs.page.length > 0 || injected;
    await page.screenshot({ path: join(SHOTS, `search-${p.name}.png`) });
    rec(`search: ${p.name}`, !problem ? 'PASS' : 'FAIL',
      `q=${JSON.stringify(p.q)} hits=${hits} injectedScript=${injected} apiErrs=${bad.length} pageErr=${errs.page.length} ${errs.page[0]?.text.slice(0, 120) ?? ''}`);
  }

  // empty query → no results call, no crash
  tag = 'search-empty-submit';
  await page.goto(`${BASE}/search`, { waitUntil: 'domcontentloaded' });
  await settle(page, 800);
  const apiBefore = apiCalls.length;
  await inp.fill('   ');
  await inp.press('Enter');
  await page.waitForTimeout(800);
  const entCalls = apiCalls.slice(apiBefore).filter((c) => /entities\?|entities\/query/.test(c.url));
  rec('search: empty query', entCalls.length === 0 && !page.url().includes('q=') ? 'PASS' : 'CHECK',
    `entityCalls=${entCalls.length} url=${page.url()}`);

  // click-through
  tag = 'search-click';
  await page.goto(`${BASE}/search?q=${encodeURIComponent('Ромашка')}`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1300);
  const first = page.locator('a[href^="/entities/"]').first();
  if (await first.count()) {
    await first.click();
    await settle(page);
    rec('search: result click-through', page.url().includes('/entities/') ? 'PASS' : 'FAIL', `landed=${page.url()}`);
  } else rec('search: result click-through', 'CHECK', 'no hits for Ромашка');
  rec('search: pagination', 'INFO', 'results capped at limit=50, no pagination UI');
}

// ===========================================================================
// 8. /objects/wh_zone/create — full flow
// ===========================================================================
const ZONE_CODE = `Z-QA-R6-${Date.now() % 100000}`;
{
  tag = 'create-form';
  await page.goto(`${BASE}/objects/wh_zone/create?catalog=wh`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1500);
  const formInputs = await page.locator('form input').count();
  await page.screenshot({ path: join(SHOTS, 'create-form.png') });
  rec('create: form renders', formInputs >= 2 ? 'PASS' : 'FAIL', `inputs=${formInputs}`);

  // required-field validation: submit empty
  tag = 'create-required';
  const apiBefore = apiCalls.length;
  await page.locator('form button[type="submit"]').last().click();
  await page.waitForTimeout(3000);
  const post = apiCalls.slice(apiBefore).find((c) => c.method === 'POST' && /\/entities(\?|$)/.test(c.url));
  const formStillThere = await page.locator('form').count();
  const errBlock = await page.locator('form [class*="red"], form [role="alert"], .border-red-300').first().isVisible().catch(() => false);
  const pageTxt = (await page.locator('body').innerText().catch(() => '')).replace(/\s+/g, ' ');
  const showsRequiredMsg = /required|обязател|wh_zone_code|missing/i.test(pageTxt);
  await page.screenshot({ path: join(SHOTS, 'create-required-errors.png') });
  // Expected: inline field errors, form intact. Actual: 422 without a
  // validation_errors map → viewError replaces the whole form (data lost).
  rec('create: required-field errors', formStillThere > 0 && errBlock ? 'PASS' : 'FAIL',
    `POST=${post?.status} formStillThere=${formStillThere} inlineErrBlock=${errBlock} msgShown=${showsRequiredMsg} — 422 has no validation_errors map → viewError destroys the form`);

  // fill code + create (reload — the previous error may have nuked the form)
  tag = 'create-submit';
  await page.goto(`${BASE}/objects/wh_zone/create?catalog=wh`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1500);
  await page.locator('form input').first().fill(ZONE_CODE);
  const capInput = page.locator('form input[type="number"]').first();
  if (await capInput.count()) await capInput.fill('555');
  const apiBefore2 = apiCalls.length;
  await page.locator('form button[type="submit"]').last().click();
  await page.waitForTimeout(3500);
  const createPost = apiCalls.slice(apiBefore2).find((c) => c.method === 'POST' && /\/entities(\?|$)/.test(c.url));
  const landed = /\/entities\/[0-9a-f-]{36}/.test(page.url());
  createdEntityPid = landed ? page.url().split('/entities/')[1]?.split('/')[0] : null;
  await page.screenshot({ path: join(SHOTS, 'create-landed.png') });
  rec('create: entity created → card', landed ? 'PASS' : 'FAIL',
    `POST=${createPost?.status} landed=${page.url()} pid=${createdEntityPid}`);

  // second identical → dedup dialog
  tag = 'create-dedup';
  await page.goto(`${BASE}/objects/wh_zone/create?catalog=wh`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1500);
  await page.locator('form input').first().fill(ZONE_CODE);
  await page.locator('form button[type="submit"]').last().click();
  await page.waitForTimeout(3500);
  const ddlg = page.locator('[role="dialog"]').last();
  const dvis = await ddlg.isVisible().catch(() => false);
  const dtxt = (await ddlg.innerText().catch(() => '')).replace(/\s+/g, ' ');
  await page.screenshot({ path: join(SHOTS, 'create-dedup-dialog.png') });
  const hasExact = /точн|exact|схож|similar|duplicate|дубли/i.test(dtxt);
  rec('create: dedup dialog appears', dvis && hasExact ? 'PASS' : 'CHECK',
    `visible=${dvis} "${dtxt.slice(0, 200)}"`);
  if (dvis) {
    // exercise "create distinct" → second entity (expected: 409 identity
    // collision on an exact match — check the error is surfaced gracefully)
    const distinctBtn = ddlg.locator('button').filter({ hasText: /отдельн|distinct|всё равно|создать/i }).first();
    if (await distinctBtn.count()) {
      const apiBefore3 = apiCalls.length;
      await distinctBtn.click();
      await page.waitForTimeout(3500);
      const post3 = apiCalls.slice(apiBefore3).find((c) => c.method === 'POST' && /\/entities(\?|$)/.test(c.url));
      const landed3 = /\/entities\/[0-9a-f-]{36}/.test(page.url());
      createdDistinctPid = landed3 ? page.url().split('/entities/')[1]?.split('/')[0] : null;
      const formAlive = await page.locator('form').count();
      const pageTxt3 = (await page.locator('body').innerText().catch(() => '')).replace(/\s+/g, ' ');
      const errShown = /collision|коллизи|conflict|ошиб/i.test(pageTxt3);
      await page.screenshot({ path: join(SHOTS, 'create-distinct-result.png') });
      rec('create: create-distinct on exact dup',
        landed3 ? 'PASS' : post3?.status === 409 && formAlive === 0 ? 'FAIL' : 'CHECK',
        `POST=${post3?.status} landed=${page.url()} formAlive=${formAlive} errShown=${errShown} — 409 identity collision → viewError destroys the form (same bug as required-field)`);
    } else {
      rec('create: create-distinct', 'CHECK', 'no distinct button — using cancel');
      await ddlg.locator('button').first().click().catch(() => {});
    }
  }

  // draft save/restore
  tag = 'create-draft';
  await page.goto(`${BASE}/objects/wh_zone/create?catalog=wh`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1500);
  const draftCode = `${ZONE_CODE}-DRAFT`;
  await page.locator('form input').first().fill(draftCode);
  // save-draft = second-to-last button in form footer (cancel, draft, submit)
  const formBtns = page.locator('form button');
  const nBtns = await formBtns.count();
  await formBtns.nth(nBtns - 2).click();
  await page.waitForTimeout(1200);
  const savedLS = await page.evaluate((k) => localStorage.getItem(k), `entity_draft_wh_zone`);
  await page.goto(`${BASE}/objects/wh_zone/create?catalog=wh`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1500);
  const restored = await page.locator('form input').first().inputValue();
  const bodyTxt = await page.locator('body').innerText();
  await page.screenshot({ path: join(SHOTS, 'create-draft-restored.png') });
  rec('create: draft save + restore', savedLS?.includes(draftCode) && restored === draftCode ? 'PASS' : 'FAIL',
    `localStorage=${!!savedLS} restoredValue="${restored}" toast=${/черновик восстановлен|draft restored/i.test(bodyTxt)}`);
  // clear the draft so it doesn't pollute later runs
  await page.evaluate((k) => localStorage.removeItem(k), `entity_draft_wh_zone`);
}

// ===========================================================================
// 9. /entities/[pid]/edit — revision bump
// ===========================================================================
{
  const editPid = createdDistinctPid ?? createdEntityPid;
  if (editPid) {
    tag = 'entity-edit';
    const before = await apiFetch(`/entities/${editPid}`);
    const rev0 = before.json?.data?.revision;
    await page.goto(`${BASE}/entities/${editPid}/edit`, { waitUntil: 'domcontentloaded' });
    await settle(page, 1500);
    const numInput = page.locator('form input[type="number"]').first();
    if (await numInput.count()) {
      await numInput.fill('777');
      const apiBefore = apiCalls.length;
      await page.locator('form button[type="submit"]').last().click();
      await page.waitForTimeout(3000);
      const put = apiCalls.slice(apiBefore).find((c) => c.method === 'PUT' && /entities\//.test(c.url));
      const landed = page.url().endsWith(`/entities/${editPid}`);
      const after = await apiFetch(`/entities/${editPid}`);
      const rev1 = after.json?.data?.revision;
      const cardTxt = await page.locator('body').innerText();
      const revShown = new RegExp(`rev\\.?\\s*${rev1}`).test(cardTxt) || cardTxt.includes(`${rev1}`);
      await page.screenshot({ path: join(SHOTS, 'entity-edit-saved.png') });
      rec('entity: edit + revision bump', put?.status < 400 && rev1 === rev0 + 1 ? 'PASS' : 'FAIL',
        `PUT=${put?.status} rev ${rev0}→${rev1} landed=${landed} revBadgeShown=${revShown}`);
    } else rec('entity: edit form', 'FAIL', 'no number input in form');
  } else rec('entity: edit', 'CHECK', 'no entity created to edit');
}

// ===========================================================================
// cleanup: retire created entities (pid1 via UI dialog, pid2 via API)
// ===========================================================================
{
  tag = 'cleanup-retire';
  if (createdEntityPid) {
    await page.goto(`${BASE}/entities/${createdEntityPid}`, { waitUntil: 'domcontentloaded' });
    await settle(page, 1200);
    const retBtn = page.locator('main button:has([class*="i-lucide:archive"])').first();
    if (await retBtn.count()) {
      await retBtn.click();
      await page.waitForTimeout(700);
      const rdlg = page.locator('[role="dialog"]').last();
      const rvis = await rdlg.isVisible().catch(() => false);
      if (rvis) {
        const rinput = rdlg.locator('input').first();
        if (await rinput.count()) await rinput.fill('qa round6 cleanup');
        const apiBefore = apiCalls.length;
        await rdlg.locator('button').last().click();
        await page.waitForTimeout(2500);
        const post = apiCalls.slice(apiBefore).find((c) => /retire/.test(c.url));
        const chk = await apiFetch(`/entities/${createdEntityPid}`);
        const st = chk.json?.data?.status;
        await page.screenshot({ path: join(SHOTS, 'cleanup-retire-ui.png') });
        rec('cleanup: retire entity via UI', st === 'retired' ? 'PASS' : 'FAIL',
          `dialog=${rvis} POST=${post?.status} status=${st}`);
      } else {
        const r = await apiRetire(createdEntityPid);
        rec('cleanup: retire entity via API', r.status < 400 ? 'PASS' : 'FAIL', `no dialog; API=${r.status}`);
      }
    } else {
      const r = await apiRetire(createdEntityPid);
      rec('cleanup: retire entity via API', r.status < 400 ? 'PASS' : 'FAIL', `no retire btn; API=${r.status}`);
    }
  }
  if (createdDistinctPid) {
    const r = await apiRetire(createdDistinctPid);
    rec('cleanup: retire distinct entity', r.status < 400 ? 'PASS' : 'FAIL', `API=${r.status}`);
  }
  // leftover QA drafts
  const { json: djson } = await apiFetch('/schemas/drafts');
  for (const d of (djson?.data ?? []).filter((x) => /QA round6|QA BAD|BAD ALIAS/.test(x.display_name ?? ''))) {
    const full = await apiFetch(`/schemas/drafts/${d.revision_id}`);
    const del = await apiFetch(`/schemas/drafts/${d.revision_id}`, {
      method: 'DELETE', headers: { 'If-Match': full.json?.data?.etag ?? '*' },
    });
    rec('cleanup: leftover draft', del.status < 400 ? 'PASS' : 'CHECK',
      `"${d.display_name}" → ${del.status}`);
  }
}

// ===========================================================================
// 10. keyboard nav / focus-visible / aria
// ===========================================================================
{
  tag = 'a11y-keyboard';
  await page.goto(`${BASE}/objects`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1200);
  // Tab order: collect first ~12 focused elements
  const seq = [];
  await page.locator('body').click({ position: { x: 5, y: 5 } }).catch(() => {});
  for (let i = 0; i < 14; i++) {
    await page.keyboard.press('Tab');
    const info = await page.evaluate(() => {
      const el = document.activeElement;
      if (!el) return null;
      const inAside = !!el.closest('aside');
      const inHeader = !!el.closest('header');
      const cs = getComputedStyle(el);
      const hasOutline =
        (cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) > 0) ||
        (cs.boxShadow !== 'none' && cs.boxShadow.includes('rgb'));
      return {
        tag: el.tagName,
        text: (el.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 30),
        href: el.getAttribute('href') ?? '',
        zone: inAside ? 'sidebar' : inHeader ? 'header' : 'main/other',
        focusVisible: hasOutline,
      };
    });
    if (info) seq.push(info);
  }
  const sidebarHits = seq.filter((s) => s.zone === 'sidebar').length;
  const focused = seq.filter((s) => s.focusVisible).length;
  await page.screenshot({ path: join(SHOTS, 'a11y-tab-order.png') });
  rec('a11y: tab order reaches sidebar', sidebarHits > 0 ? 'PASS' : 'CHECK',
    `sequence: ${seq.map((s) => `${s.zone}:${s.tag}${s.href ? '→' + s.href : ''}`).join(' | ').slice(0, 400)}`);
  rec('a11y: focus-visible indicator', focused === seq.length ? 'PASS' : focused > seq.length / 2 ? 'CHECK' : 'FAIL',
    `${focused}/${seq.length} focused elements show visible outline`);

  // icon-only buttons across scanned pages
  for (const [u, n] of [
    ['/admin/dump', 'dump'], ['/admin/schema', 'schema'], ['/admin/types', 'types'],
    ['/admin/predicates', 'predicates'], ['/admin/catalogs', 'catalogs'],
    ['/integrations', 'integrations'], ['/docs', 'docs'],
  ]) {
    await page.goto(`${BASE}${u}`, { waitUntil: 'domcontentloaded' });
    await settle(page, 900);
    const unl = await unlabeledIconButtons(page);
    if (unl.length) rec(`a11y: unlabeled icon buttons ${n}`, 'FAIL', `${unl.length}: ${JSON.stringify(unl.slice(0, 5))}`);
  }
}

// ===========================================================================
// 11. viewports 400px + 2560px
// ===========================================================================
{
  const pages = [
    ['/admin/dump', 'admin-dump'], ['/admin/schema', 'admin-schema'], ['/admin/types', 'admin-types'],
    ['/admin/predicates', 'admin-predicates'], ['/admin/catalogs', 'admin-catalogs'],
    ['/integrations', 'integrations'], ['/docs', 'docs'], ['/search', 'search'],
    ['/objects/wh_zone/create?catalog=wh', 'create-whzone'],
  ];
  for (const [w, wname] of [[400, '400'], [2560, '2560']]) {
    await page.setViewportSize({ width: w, height: 850 });
    for (const [u, n] of pages) {
      tag = `vp-${wname}-${n}`;
      await page.goto(`${BASE}${u}`, { waitUntil: 'domcontentloaded' }).catch(() => {});
      await settle(page, 800);
      const ov = await overflowInfo(page);
      const errs = errsFor(tag);
      if (ov.overflow || errs.page.length || w === 2560) {
        await page.screenshot({ path: join(SHOTS, `vp-${wname}-${n}.png`) });
      }
      rec(`${wname}px ${n}`, !ov.overflow && errs.page.length === 0 ? 'PASS' : 'FAIL',
        `scrollW=${ov.scrollW} innerW=${ov.innerW} pageErr=${errs.page.length}${errs.page.length ? ' ' + errs.page[0].text.slice(0, 140) : ''}`);
    }
  }
  await page.setViewportSize({ width: 1440, height: 900 });
}

// ===========================================================================
// 12. RTL locale (he) on admin pages
// ===========================================================================
{
  tag = 'rtl-he';
  await page.goto(`${BASE}/admin/dump`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1200);
  // locale select = header combobox WITHOUT aria-label
  const combos = await page.locator('header [role="combobox"]').all();
  let localeCombo = null;
  for (const c of combos) {
    if (!(await c.getAttribute('aria-label'))) { localeCombo = c; break; }
  }
  let switched = false;
  if (localeCombo) {
    switched = await clickComboboxOption(page, localeCombo, 'עברית');
    await page.waitForTimeout(1000);
  }
  const dir = await page.evaluate(() => document.documentElement.dir || document.documentElement.getAttribute('dir'));
  const lang = await page.evaluate(() => document.documentElement.lang);
  await page.screenshot({ path: join(SHOTS, 'rtl-he-admin-dump.png') });
  rec('rtl: he switches dir', switched && dir === 'rtl' ? 'PASS' : switched ? 'CHECK' : 'CHECK',
    `optionFound=${switched} dir=${dir} lang=${lang}`);

  // locale prefix strategy: non-default locales live under /<code>/ — use it
  await page.goto(`${BASE}/he/admin/schema`, { waitUntil: 'domcontentloaded' });
  await settle(page, 1200);
  const dir2 = await page.evaluate(() => document.documentElement.dir);
  const ov = await overflowInfo(page);
  const lint = await bodyLint(page);
  await page.screenshot({ path: join(SHOTS, 'rtl-he-admin-schema.png') });
  rec('rtl: /admin/schema layout flip', dir2 === 'rtl' && !ov.overflow ? 'PASS' : 'CHECK',
    `dir=${dir2} overflow=${ov.overflow} i18n=${lint.i18nKeys.slice(0, 4).join(',') || 'none'}`);

  // switch back to ru
  const combos2 = await page.locator('header [role="combobox"]').all();
  for (const c of combos2) {
    if (!(await c.getAttribute('aria-label'))) {
      await clickComboboxOption(page, c, 'Русский');
      break;
    }
  }
  await page.waitForTimeout(800);
}

} catch (e) {
  rec('SCRIPT ERROR', 'FAIL', `${e.stack?.slice(0, 500)}`);
}

// --- summary ---------------------------------------------------------------
const fails = results.filter((r) => r.verdict === 'FAIL');
const checks = results.filter((r) => r.verdict === 'CHECK');
console.log(`\n===== ${results.length} checks: ${results.length - fails.length - checks.length} PASS, ${checks.length} CHECK, ${fails.length} FAIL =====`);
for (const c of checks) console.log(`CHECK: ${c.name} — ${c.evidence.split('\n')[0].slice(0, 200)}`);
for (const f of fails) console.log(`FAIL: ${f.name} — ${f.evidence.split('\n')[0].slice(0, 200)}`);
writeFileSync(join(HERE, 'qa_round6_results.json'), JSON.stringify(results, null, 2));
await browser.close();
process.exit(fails.length ? 1 : 0);
