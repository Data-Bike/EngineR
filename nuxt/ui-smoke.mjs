/**
 * UI smoke test — walks through all major pages, takes screenshots.
 */
import { chromium } from 'playwright';
import { mkdirSync } from 'node:fs';

const BASE = 'http://localhost:3000';
const SHOTS = './ui-screenshots';

mkdirSync(SHOTS, { recursive: true });

const pages = [
  { path: '/', name: 'home' },
  { path: '/audit', name: 'audit' },
  { path: '/checks', name: 'checks' },
  { path: '/integrations', name: 'integrations' },
  { path: '/admin/schema', name: 'admin-schema' },
  { path: '/monitoring', name: 'monitoring' },
  { path: '/reports', name: 'reports' },
  { path: '/objects/person', name: 'objects-person' },
  { path: '/objects/legal_entity', name: 'objects-legal-entity' },
  { path: '/requests', name: 'requests' },
  { path: '/risks', name: 'risks' },
  { path: '/docs', name: 'docs' },
  { path: '/admin', name: 'admin' },
];

async function run() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    locale: 'ru-RU',
  });
  const page = await context.newPage();

  const consoleErrors = [];
  page.on('console', (msg) => {
    if (msg.type() === 'error') {
      consoleErrors.push(`[console.error] ${msg.text()}`);
    }
  });
  page.on('pageerror', (err) => {
    consoleErrors.push(`[pageerror] ${err.message}`);
  });

  // 1. Go to site — mock auth auto-logs in
  console.log('=== Step 1: Navigate to site ===');
  await page.goto(BASE, { waitUntil: 'networkidle', timeout: 20000 });
  await page.waitForTimeout(3000);
  await page.screenshot({ path: `${SHOTS}/01-home.png`, fullPage: true });
  console.log('URL:', page.url());
  console.log('Title:', await page.title());

  // Check if on login page or already logged in
  const usernameInput = page.locator('input[autocomplete="username"]');
  const isLogin = await usernameInput.isVisible().catch(() => false);
  console.log('On login page:', isLogin);

  if (isLogin) {
    console.log('\n=== Login admin/admin ===');
    await usernameInput.fill('admin');
    await page.locator('input[autocomplete="current-password"]').fill('admin');
    await page.locator('button[type="submit"]').click();
    await page.waitForTimeout(3000);
    await page.screenshot({ path: `${SHOTS}/02-after-login.png`, fullPage: true });
    console.log('URL after login:', page.url());
  }

  // 2. Walk through all pages
  console.log('\n=== Step 2: Walk through pages ===');
  for (const p of pages) {
    console.log(`\n--- ${p.name} (${p.path}) ---`);
    try {
      await page.goto(`${BASE}${p.path}`, { waitUntil: 'networkidle', timeout: 20000 });
      await page.waitForTimeout(2500);
      console.log('URL:', page.url());

      // Check if redirected to login
      if (page.url().includes('/login')) {
        console.log('  REDIRECTED to login!');
        await page.screenshot({ path: `${SHOTS}/03-${p.name}-login.png`, fullPage: true });
        continue;
      }

      // H1
      const h1 = page.locator('h1').first();
      if (await h1.isVisible().catch(() => false)) {
        console.log('H1:', (await h1.textContent())?.trim());
      }

      // Tables
      const tables = page.locator('table');
      const tableCount = await tables.count();
      if (tableCount > 0) console.log('Tables:', tableCount);

      // Cards
      const cards = page.locator('[class*="card"], [class*="Card"]');
      const cardCount = await cards.count();
      if (cardCount > 0) console.log('Cards:', cardCount);

      // Buttons
      const buttons = page.locator('button');
      const buttonCount = await buttons.count();
      console.log('Buttons:', buttonCount);

      // Table rows
      if (tableCount > 0) {
        const rows = page.locator('table tbody tr');
        const rowCount = await rows.count();
        console.log('Table rows:', rowCount);
      }

      await page.screenshot({ path: `${SHOTS}/03-${p.name}.png`, fullPage: true });
    } catch (err) {
      console.log(`  ERROR: ${err.message.substring(0, 100)}`);
    }
  }

  // 3. Sidebar navigation
  console.log('\n=== Step 3: Sidebar ===');
  await page.goto(`${BASE}/`, { waitUntil: 'networkidle', timeout: 20000 });
  await page.waitForTimeout(2000);
  const navLinks = page.locator('nav a, aside a, [class*="sidebar"] a');
  const navCount = await navLinks.count();
  console.log('Nav links:', navCount);
  for (let i = 0; i < Math.min(navCount, 15); i++) {
    const text = (await navLinks.nth(i).textContent())?.trim();
    const href = await navLinks.nth(i).getAttribute('href');
    if (text) console.log(`  "${text}" → ${href}`);
  }

  // 4. Console errors
  console.log('\n=== Console Errors ===');
  if (consoleErrors.length === 0) {
    console.log('No console errors!');
  } else {
    console.log(`${consoleErrors.length} errors:`);
    for (const err of consoleErrors.slice(0, 10)) console.log(err);
  }

  await browser.close();
  console.log('\n=== Done ===');
}

run().catch((err) => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
