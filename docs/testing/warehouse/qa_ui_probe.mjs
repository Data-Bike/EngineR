/**
 * qa_ui_probe.mjs — visual/behavioral QA of the EngineR Nuxt admin UI
 * from a low-privilege account (risk_analyst), using playwright-core
 * resolved from nuxt/node_modules.
 *
 * Run:  node docs/testing/warehouse/qa_ui_probe.mjs
 */

const pw = await import(
  'file:///C:/Users/gorod/RustroverProjects/EngineR/nuxt/node_modules/playwright-core/index.mjs'
);
const { chromium } = pw;

const BASE = 'http://localhost:3000';
const SHOTS =
  'C:/Users/gorod/RustroverProjects/EngineR/docs/testing/warehouse/qa_shots';

const ENTITIES = {
  xss1: '01a15bc0-134e-7782-bc0c-4356b91ab5db', // <img onerror> + <script> payloads
  xss2: '01a15bc0-2c97-71b0-835f-4fe201f38e03', // attribute-breakout payload
  denied: '01a15bc1-2900-7c70-be27-7f17e1ddb39e', // qa_ui_deny rule on risk_analyst
  visible: '01a15652-35d0-7b23-8337-2555007510ed', // ordinary wh_warehouse
  bogus: '00000000-0000-0000-0000-00000000dead',
};

const results = [];
const apiCalls = [];
const consoleErrs = [];
const pageErrs = [];
const dialogs = [];
let tag = 'boot';

function rec(name, verdict, evidence) {
  results.push({ name, verdict, evidence });
  console.log(`\n### ${name}\n    ${verdict}\n    ${evidence}`);
}

const browser = await chromium.launch();
const ctx = await browser.newContext({
  viewport: { width: 1400, height: 900 },
  ignoreHTTPSErrors: true,
});
const page = await ctx.newPage();

page.on('console', (m) => {
  if (m.type() === 'error') consoleErrs.push({ tag, text: m.text() });
});
page.on('pageerror', (e) => pageErrs.push({ tag, text: String(e) }));
page.on('response', (r) => {
  const u = r.url();
  if (u.includes('/api/')) {
    apiCalls.push({
      tag,
      method: r.request().method(),
      url: u.replace(BASE, ''),
      status: r.status(),
    });
  }
});
page.on('dialog', async (d) => {
  dialogs.push({ tag, message: d.message() });
  await d.dismiss();
});

async function snap() {
  const body = await page
    .locator('body')
    .innerText()
    .catch(() => '');
  return { url: page.url(), body: body.slice(0, 1200) };
}

// ---------------------------------------------------------------------
// 0. Unauthenticated direct navigation to an admin URL
// ---------------------------------------------------------------------
tag = 'unauth-/admin/users';
await page.goto(`${BASE}/admin/users`, { waitUntil: 'networkidle' });
let s = await snap();
await page.screenshot({ path: `${SHOTS}/00-unauth-admin-users.png` });
const unauthApi = apiCalls.filter(
  (c) => c.tag === tag && /users|security|permissions/.test(c.url),
);
rec(
  'unauth /admin/users',
  s.url.includes('/login') ? 'PASS' : 'FAIL',
  `final=${s.url} | admin-data api calls fired: ${JSON.stringify(unauthApi)} | body="${s.body.slice(0, 160).replace(/\n/g, ' | ')}"`,
);

// ---------------------------------------------------------------------
// 1. Login as risk_analyst
// ---------------------------------------------------------------------
tag = 'login';
await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
await page.locator('input[autocomplete="username"]').fill('risk_analyst');
await page.locator('input[type="password"]').fill('enginer-demo');
await page.locator('button[type="submit"]').click();
await page.waitForURL((u) => !u.pathname.startsWith('/login'), {
  timeout: 15000,
});
await page.waitForLoadState('networkidle');
s = await snap();
rec(
  'login risk_analyst',
  !s.url.includes('/login') ? 'PASS' : 'FAIL',
  `landed on ${s.url}`,
);

// Sidebar: admin section must not be rendered for this capability set
const sideText = await page
  .locator('nav, aside, [class*=sidebar]')
  .first()
  .innerText()
  .catch(() => '');
const adminLinks = await page
  .locator('a[href^="/admin"]')
  .count()
  .catch(() => -1);
rec(
  'sidebar hides admin links',
  adminLinks === 0 ? 'PASS' : 'FAIL',
  `a[href^="/admin"] count=${adminLinks} | sidebar text sample="${sideText.slice(0, 200).replace(/\n/g, ' | ')}"`,
);

// ---------------------------------------------------------------------
// 2. Admin pages as risk_analyst — expect clean 403, no data
// ---------------------------------------------------------------------
for (const p of ['/admin', '/admin/permissions', '/admin/users', '/admin/schema', '/admin/catalogs', '/audit']) {
  tag = `denied-${p}`;
  apiCalls.length = 0;
  await page.goto(`${BASE}${p}`, { waitUntil: 'networkidle' });
  s = await snap();
  await page.screenshot({ path: `${SHOTS}/denied-${p.replace(/\//g, '_')}.png` });
  const fired = apiCalls.filter((c) => c.status < 400);
  const has403 = /403/.test(s.body);
  // crude leak check: table rows / user emails rendered?
  const leak = /@demo\.local|principal_id|csrf/i.test(s.body);
  rec(
    `admin page ${p}`,
    has403 && !leak ? 'PASS' : 'CHECK',
    `final=${s.url} | body-mentions-403=${has403} | leak-suspect=${leak} | api<400: ${JSON.stringify(fired)} | body="${s.body.slice(0, 200).replace(/\n/g, ' | ')}"`,
  );
}

// ---------------------------------------------------------------------
// 3. /reports — download button must be hidden (no report.export)
// ---------------------------------------------------------------------
tag = 'reports';
await page.goto(`${BASE}/reports`, { waitUntil: 'networkidle' });
await page.waitForTimeout(1500);
s = await snap();
await page.screenshot({ path: `${SHOTS}/reports.png` });
const dlButtons = await page
  .locator('button:has-text("Скачать"), button:has-text("Download"), a:has-text("Скачать"), a:has-text("Download")')
  .count();
const dlIcons = await page.locator('.i-lucide-download').count();
rec(
  'reports: download hidden w/o report.export',
  dlButtons === 0 && dlIcons === 0 ? 'PASS' : 'FAIL',
  `download buttons=${dlButtons} download icons=${dlIcons} | body="${s.body.slice(0, 200).replace(/\n/g, ' | ')}"`,
);

// ---------------------------------------------------------------------
// 4. Entity pages: bogus id / denied id / visible id / edit+retire buttons
// ---------------------------------------------------------------------
tag = 'entity-bogus';
await page.goto(`${BASE}/entities/${ENTITIES.bogus}`, {
  waitUntil: 'networkidle',
});
await page.waitForTimeout(1200);
s = await snap();
await page.screenshot({ path: `${SHOTS}/entity-bogus.png` });
const bogusHasTabs = (await page.locator('[role="tab"]').count()) > 0;
rec(
  'entity bogus id',
  !bogusHasTabs ? 'PASS' : 'FAIL',
  `final=${s.url} tabs-rendered=${bogusHasTabs} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
);

tag = 'entity-denied';
apiCalls.length = 0;
await page.goto(`${BASE}/entities/${ENTITIES.denied}`, {
  waitUntil: 'networkidle',
});
await page.waitForTimeout(1200);
s = await snap();
await page.screenshot({ path: `${SHOTS}/entity-denied.png` });
const deniedHasTabs = (await page.locator('[role="tab"]').count()) > 0;
const deniedLeak = s.body.includes('UI QA DENIED SECRET');
const deniedGet = apiCalls.find((c) =>
  c.url.includes(`/entities/${ENTITIES.denied}`),
);
rec(
  'entity denied (per-entity deny rule)',
  !deniedHasTabs && !deniedLeak ? 'PASS' : 'FAIL',
  `api=${JSON.stringify(deniedGet)} tabs=${deniedHasTabs} secretLeaked=${deniedLeak} | body="${s.body.slice(0, 300).replace(/\n/g, ' | ')}"`,
);

tag = 'entity-visible';
await page.goto(`${BASE}/entities/${ENTITIES.visible}`, {
  waitUntil: 'networkidle',
});
await page.waitForTimeout(1200);
s = await snap();
await page.screenshot({ path: `${SHOTS}/entity-visible.png` });
const btnTexts = await page
  .locator('button')
  .allInnerTexts()
  .catch(() => []);
const editShown = btnTexts.some((b) => /ред|edit/i.test(b));
const retireShown = btnTexts.some((b) => /архив|retire|удал/i.test(b));
rec(
  'entity visible renders',
  s.body.includes('Shared-Predicate-Test') ? 'PASS' : 'FAIL',
  `body has name=${s.body.includes('Shared-Predicate-Test')} | buttons: editShown=${editShown} retireShown=${retireShown} (analyst lacks entity.edit/entity.retire -> UI bug if shown)`,
);

// ---------------------------------------------------------------------
// 5. XSS probes — entity detail + list page
// ---------------------------------------------------------------------
for (const [k, pid] of [
  ['xss1', ENTITIES.xss1],
  ['xss2', ENTITIES.xss2],
]) {
  tag = `entity-${k}`;
  await page.goto(`${BASE}/entities/${pid}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${SHOTS}/entity-${k}.png` });
  const dom = await page.evaluate(() => ({
    rawImg: !!document.querySelector('img[src="x"]'),
    scriptTags: [...document.querySelectorAll('script')]
      .map((x) => x.textContent || '')
      .filter((x) => x.includes('alert(2)')).length,
    htmlHasRawImgTag: document.body.innerHTML.includes(
      '<img src=x onerror=alert(1)>',
    ),
    htmlHasRawScript: document.body.innerHTML.includes(
      '<script>alert(2)</script>',
    ),
    escapedShown: document.body.innerText.includes(
      '<img src=x onerror=alert(1)>',
    ),
    focusEl: !!document.querySelector('[onfocus]'),
    text: document.body.innerText.slice(0, 400),
  }));
  rec(
    `XSS detail ${k}`,
    !dom.rawImg && !dom.scriptTags && !dom.focusEl && dom.escapedShown
      ? 'PASS'
      : 'FAIL',
    JSON.stringify(dom).slice(0, 500),
  );
}

// list page with xss row
tag = 'objects-wh_product';
await page.goto(`${BASE}/objects/wh_product?catalog=wh`, {
  waitUntil: 'networkidle',
});
await page.waitForTimeout(1500);
await page.screenshot({ path: `${SHOTS}/objects-wh_product.png` });
const listDom = await page.evaluate(() => ({
  rawImg: !!document.querySelector('img[src="x"]'),
  htmlHasRawImgTag: document.body.innerHTML.includes(
    '<img src=x onerror=alert(1)>',
  ),
  escapedShown: document.body.innerText.includes(
    '<img src=x onerror=alert(1)>',
  ),
}));
rec(
  'XSS list page',
  !listDom.rawImg && listDom.escapedShown ? 'PASS' : 'CHECK',
  JSON.stringify(listDom),
);

// ---------------------------------------------------------------------
// 6. Storage inspection
// ---------------------------------------------------------------------
tag = 'storage';
await page.goto(`${BASE}/`, { waitUntil: 'networkidle' });
const storage = await page.evaluate(() => {
  const grab = (st) => {
    const o = {};
    for (let i = 0; i < st.length; i++) {
      const k = st.key(i);
      o[k] = (st.getItem(k) || '').slice(0, 300);
    }
    return o;
  };
  return { local: grab(localStorage), session: grab(sessionStorage) };
});
const cookies = await ctx.cookies(BASE);
const cookieInfo = cookies.map((c) => ({
  name: c.name,
  httpOnly: c.httpOnly,
  secure: c.secure,
  sameSite: c.sameSite,
  valueLen: c.value.length,
}));
const storageStr = JSON.stringify(storage);
const secretsInStorage =
  /egr_|enginer-demo|csrf|bearer|password/i.test(storageStr);
rec(
  'storage hygiene',
  !secretsInStorage ? 'PASS' : 'CHECK',
  `local=${JSON.stringify(storage.local)} session=${JSON.stringify(storage.session)} cookies=${JSON.stringify(cookieInfo)}`,
);

// ---------------------------------------------------------------------
// 7. Console / page errors summary
// ---------------------------------------------------------------------
rec(
  'console errors',
  consoleErrs.length === 0 && pageErrs.length === 0 ? 'PASS' : 'CHECK',
  `console: ${JSON.stringify(consoleErrs.slice(0, 15))} page: ${JSON.stringify(pageErrs.slice(0, 10))}`,
);
rec(
  'dialogs (alerts) fired',
  dialogs.length === 0 ? 'PASS' : 'FAIL',
  JSON.stringify(dialogs),
);

console.log('\n==================== ALL API CALLS ====================');
console.log(JSON.stringify(apiCalls, null, 0));

await browser.close();
console.log('\nDONE');
