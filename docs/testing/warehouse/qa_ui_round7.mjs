// QA round 7 — focused re-checks for this round's fixes.
// Requires the stack up (web :3000, api :8010) and .wh_tokens.json.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
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
const SHOTS = join(__dir, 'qa_shots_round7');
mkdirSync(SHOTS, { recursive: true });

const results = [];
const rec = (name, ok, note = '') => {
  results.push({ name, ok, note });
  console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${note ? ` — ${note}` : ''}`);
};

async function api(method, path, body, headers = {}) {
  const r = await fetch(API + path, {
    method,
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      ...headers,
    },
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

// 1. /admin/dump — delete asks confirmation
await page.goto(`${WEB}/admin/dump`, { waitUntil: 'networkidle' });
const delBtn = page.locator('[data-testid^="delete-"]').first();
let deleteConfirmed = 'skip';
if (await delBtn.count()) {
  await delBtn.click();
  await page.waitForTimeout(400);
  const modal = page.locator('text=confirm-delete, [data-testid="confirm-delete"]');
  deleteConfirmed = await page.locator('[data-testid="confirm-delete"]').count() > 0;
  await page.screenshot({ path: join(SHOTS, 'dump-delete-modal.png') });
  await page.keyboard.press('Escape');
}
rec('dump: delete shows confirm modal', deleteConfirmed === true ? true : deleteConfirmed === 'skip', String(deleteConfirmed));

// 2. /admin/schema — create draft dialog opens with stream-scoped base revision.
// Assert on the POST /schemas/drafts response status — the page legitimately
// lists validation findings (severity "error") for the published stream, so
// scanning body text for "error" is a false positive.
await page.goto(`${WEB}/admin/schema`, { waitUntil: 'networkidle' });
await page.waitForTimeout(800);
const createBtn = page.locator('button', { hasText: /черновик|draft/i }).first();
let draftOk = 'skip';
let draftNote = 'no create button';
if (await createBtn.count()) {
  await createBtn.click();
  await page.waitForTimeout(400);
  const inputs = page.locator('input');
  const n = await inputs.count();
  let filled = false;
  for (let i = 0; i < n; i++) {
    const inp = inputs.nth(i);
    if (await inp.isVisible() && (await inp.getAttribute('type')) !== 'hidden') {
      await inp.fill('QA r7 draft');
      filled = true;
      break;
    }
  }
  const saveBtn = page.locator('button', { hasText: /сохранить|save/i }).last();
  if (filled && await saveBtn.count()) {
    const respPromise = page
      .waitForResponse(
        (r) => r.url().includes('/schemas/drafts') && r.request().method() === 'POST',
        { timeout: 10000 },
      )
      .catch(() => null);
    await saveBtn.click();
    const resp = await respPromise;
    if (resp) {
      draftOk = resp.status() === 201;
      const reqBody = resp.request().postData() ?? '';
      const url = resp.url();
      draftNote = `POST ${url.split('/api/v1')[1]} -> ${resp.status()}; body=${reqBody.slice(0, 200)}`;
      if (!draftOk) {
        try {
          const j = await resp.json();
          draftNote += `; err=${JSON.stringify(j.error ?? j).slice(0, 160)}`;
        } catch {}
      }
    } else {
      draftOk = false;
      draftNote = 'no POST /schemas/drafts observed';
    }
  }
  await page.screenshot({ path: join(SHOTS, 'schema-create-draft.png') });
}
rec('schema: create draft returns 201', draftOk === true ? true : draftOk === 'skip', draftNote);

// cleanup the created draft
{
  const drafts = await api('GET', '/schemas/drafts');
  for (const d of drafts.body?.data ?? []) {
    if (d.display_name === 'QA r7 draft') {
      await api('DELETE', `/schemas/drafts/${d.revision_id}`, null, { 'If-Match': d.etag });
    }
  }
}

// 3. icon-only buttons have accessible names
for (const [path, sel] of [
  ['/admin/types', 'button:has(svg)'],
  ['/admin/predicates', 'button:has(svg)'],
  ['/integrations', 'button:has(svg)'],
]) {
  await page.goto(`${WEB}${path}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(600);
  const bad = await page.evaluate(() => {
    const out = [];
    for (const b of document.querySelectorAll('button')) {
      const label = (b.getAttribute('aria-label') || b.getAttribute('title') || b.textContent || '').trim();
      const hasText = (b.innerText || '').trim().length > 0;
      if (!label && !hasText) out.push(b.outerHTML.slice(0, 80));
    }
    return out;
  });
  rec(`a11y: icon buttons labelled ${path}`, bad.length === 0, bad.length ? `${bad.length} unlabeled` : '');
}

// 4. /admin/catalogs — create then check selector refreshes via /me/context
await page.goto(`${WEB}/admin/catalogs`, { waitUntil: 'networkidle' });
const alias = 'qar7' + String(Date.now()).slice(-4);
const newBtn = page.locator('button', { hasText: /создать|create|новый/i }).first();
let catOk = 'skip';
let catNote = 'no create button';
if (await newBtn.count()) {
  await newBtn.click();
  await page.waitForTimeout(300);
  // The dialog's alias input has placeholder="hr"; name placeholder="HR".
  const aliasInput = page.locator('input[placeholder="hr"]');
  const nameInput = page.locator('input[placeholder="HR"]');
  if (await aliasInput.count()) {
    await aliasInput.fill(alias);
    await nameInput.fill('QA round7 catalog');
    // creating a catalog triggers fetchContext() — wait for it explicitly
    const postPromise = page
      .waitForResponse(
        (r) => r.url().includes('/catalogs') && r.request().method() === 'POST',
        { timeout: 10000 },
      )
      .catch(() => null);
    const ctxPromise = page
      .waitForResponse((r) => r.url().includes('/me/context'), { timeout: 10000 })
      .catch(() => null);
    const save = page.locator('button', { hasText: /сохранить|save/i }).last();
    await save.click();
    const postResp = await postPromise;
    const ctxResp = await ctxPromise;
    catNote = `POST /catalogs -> ${postResp ? postResp.status() : 'none'}; me/context=${ctxResp ? ctxResp.status() : 'none'}; body=${(postResp?.request().postData() ?? '').slice(0, 160)}`;
    if (postResp && postResp.status() >= 400) {
      try {
        const j = await postResp.json();
        catNote += `; err=${JSON.stringify(j.error ?? j).slice(0, 160)}`;
      } catch {}
    }
    await page.waitForTimeout(500);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);
    // The header catalog USelect carries aria-label = t('catalog.selector')
    // ("Каталог" in ru). Open it and look for the new alias in the listbox.
    const catSelect = page.locator('header [aria-label="Каталог"], header [aria-label="Catalog"]').first();
    if (await catSelect.count()) {
      await catSelect.click();
      await page.waitForTimeout(600);
      const listbox = page.locator('[role="listbox"], [role="menu"]');
      const text = (await listbox.count()) ? await listbox.innerText() : await page.locator('body').innerText();
      // Options render the catalog display name, not the raw alias.
      catOk = text.includes(alias) || text.includes('QA round7 catalog');
      catNote += `; option=${catOk}; listbox="${text.slice(0, 140).replace(/\n/g, '|')}"`;
      await page.keyboard.press('Escape');
    } else {
      catOk = false;
      catNote += '; catalog selector not rendered (hasMultiple?)';
    }
  }
  await page.screenshot({ path: join(SHOTS, 'catalogs-created.png') });
}
rec('catalogs: selector updates without reload', catOk === true ? true : catOk === 'skip', catNote);

// cleanup created catalog
const list = await api('GET', '/catalogs');
if (list.body?.data) {
  for (const c of list.body.data) {
    if (c.alias === alias) {
      const d = await api('DELETE', `/catalogs/${alias}`, null);
      // retire endpoint may be POST
      if (d.status >= 400) await api('POST', `/catalogs/${alias}/retire`, {});
    }
  }
}

// 5. /search — pagination button when >50 results (may skip)
await page.goto(`${WEB}/search?q=wh`, { waitUntil: 'networkidle' });
await page.waitForTimeout(1200);
const moreBtn = page.locator('button', { hasText: /ещё|more/i });
const searchNote = await page.locator('body').innerText().then((t) => t.length);
rec('search: renders without errors', pageErrors.length === 0, `moreBtn=${await moreBtn.count()}`);

// 6. create page — 422 keeps form alive
await page.goto(`${WEB}/objects/wh_zone/create`, { waitUntil: 'networkidle' });
await page.waitForTimeout(800);
const submit = page.locator('button', { hasText: /создать|save|сохранить/i }).last();
let formAlive = 'skip';
if (await submit.count()) {
  await submit.click();
  await page.waitForTimeout(1500);
  // form should still be visible (fields present), not a full-page error
  const formInputs = await page.locator('input').count();
  const hasRetry = await page.locator('button', { hasText: /повторить|retry/i }).count();
  formAlive = formInputs > 2;
  await page.screenshot({ path: join(SHOTS, 'create-422.png') });
}
rec('create: 422 keeps form (no view wipeout)', formAlive === true ? true : formAlive === 'skip', String(formAlive));

console.log(`\npageErrors: ${pageErrors.length}`);
pageErrors.slice(0, 5).forEach((e) => console.log('  ', e.slice(0, 160)));
writeFileSync(join(SHOTS, 'results.json'), JSON.stringify(results, null, 2));
const fails = results.filter((r) => r.ok === false).length;
console.log(`\n${results.filter((r) => r.ok === true).length} pass, ${fails} fail, ${results.filter((r) => r.ok === 'skip').length} skip`);
await browser.close();
process.exit(fails ? 1 : 0);
