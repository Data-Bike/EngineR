// Visual check for the force-directed GraphView.
// Logs in, opens a dense entity's graph tab, waits for the layout to settle,
// screenshots, then programmatically verifies no node bubbles overlap.
import { chromium } from 'playwright-core';

const BASE = process.env.PLAYWRIGHT_BASE_URL ?? 'http://localhost:3000';
const ENTITY = '01a150bc-baf3-7fc2-935a-1416ac671d3d'; // wh_stock, ~42 statements

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, locale: 'ru-RU' });
page.on('console', (m) => { if (m.type() === 'error') console.log('CONSOLE ERR:', m.text()); });
page.on('pageerror', (e) => console.log('PAGE ERR:', e.message));

await page.goto(`${BASE}/login`, { waitUntil: 'networkidle' });
await page.locator('input[autocomplete="username"]').fill('admin');
await page.locator('input[autocomplete="current-password"]').fill('enginer-demo');
await page.locator('button[type="submit"]').click();
await page.waitForURL((u) => !u.pathname.includes('/login'), { timeout: 15000 });
console.log('logged in:', page.url());

await page.goto(`${BASE}/entities/${ENTITY}`, { waitUntil: 'networkidle' });
// Graph tab — click the tab labelled "Graph"/"Граф".
await page.getByRole('button', { name: /граф|graph/i }).last().click().catch(async () => {
  // fallback: UTabs renders items as buttons inside the tablist
  await page.locator('[role="tab"], [role="tablist"] button, button').filter({ hasText: /граф|graph/i }).last().click();
});
await page.waitForSelector('svg[role="img"]', { timeout: 15000 });
console.log('graph svg rendered');

// Let the simulation settle.
await page.waitForTimeout(6000);

const stats = await page.evaluate(() => {
  const nodes = [...document.querySelectorAll('g.nodes > g')];
  const circles = nodes.map((g) => {
    const t = g.getAttribute('transform') ?? '';
    const m = t.match(/translate\(([-\d.e]+)[ ,]+([-\d.e]+)\)/);
    const r = Number(g.querySelector('circle')?.getAttribute('r') ?? 34);
    return { x: Number(m?.[1] ?? 0), y: Number(m?.[2] ?? 0), r };
  });
  let overlaps = 0;
  let worst = null;
  for (let i = 0; i < circles.length; i++) {
    for (let j = i + 1; j < circles.length; j++) {
      const a = circles[i], b = circles[j];
      const d = Math.hypot(a.x - b.x, a.y - b.y);
      if (d < a.r + b.r - 2) {
        overlaps++;
        worst = { d: Math.round(d), need: a.r + b.r };
      }
    }
  }
  const edges = document.querySelectorAll('g.edges path').length;
  return { nodes: circles.length, edges, overlaps, worst };
});
console.log('graph stats:', JSON.stringify(stats));

await page.screenshot({ path: 'graph_check_1.png', fullPage: false });

// Expand a node if a "+" exists.
const plus = page.locator('g.nodes circle.cursor-pointer').first();
if (await plus.count()) {
  await plus.dispatchEvent('click');
  await page.waitForTimeout(5000);
  const after = await page.evaluate(() => document.querySelectorAll('g.nodes > g').length);
  console.log('nodes after expand:', after);
  const stats2 = await page.evaluate(() => {
    const nodes = [...document.querySelectorAll('g.nodes > g')];
    const circles = nodes.map((g) => {
      const t = g.getAttribute('transform') ?? '';
      const m = t.match(/translate\(([-\d.e]+)[ ,]+([-\d.e]+)\)/);
      const r = Number(g.querySelector('circle')?.getAttribute('r') ?? 34);
      return { x: Number(m?.[1] ?? 0), y: Number(m?.[2] ?? 0), r };
    });
    let overlaps = 0;
    for (let i = 0; i < circles.length; i++) {
      for (let j = i + 1; j < circles.length; j++) {
        const a = circles[i], b = circles[j];
        if (Math.hypot(a.x - b.x, a.y - b.y) < a.r + b.r - 2) overlaps++;
      }
    }
    return { nodes: circles.length, overlaps };
  });
  console.log('after expand stats:', JSON.stringify(stats2));
  await page.screenshot({ path: 'graph_check_2.png' });
}

// Depth=2 for a denser graph.
await page.locator('input[type="number"]').fill('2');
await page.locator('input[type="number"]').dispatchEvent('change');
await page.waitForTimeout(7000);
const stats3 = await page.evaluate(() => {
  const nodes = [...document.querySelectorAll('g.nodes > g')];
  const circles = nodes.map((g) => {
    const t = g.getAttribute('transform') ?? '';
    const m = t.match(/translate\(([-\d.e]+)[ ,]+([-\d.e]+)\)/);
    const r = Number(g.querySelector('circle')?.getAttribute('r') ?? 34);
    return { x: Number(m?.[1] ?? 0), y: Number(m?.[2] ?? 0), r };
  });
  let overlaps = 0;
  for (let i = 0; i < circles.length; i++) {
    for (let j = i + 1; j < circles.length; j++) {
      const a = circles[i], b = circles[j];
      if (Math.hypot(a.x - b.x, a.y - b.y) < a.r + b.r - 2) overlaps++;
    }
  }
  const edges = document.querySelectorAll('g.edges path').length;
  return { nodes: circles.length, edges, overlaps };
});
console.log('depth2 stats:', JSON.stringify(stats3));
await page.screenshot({ path: 'graph_check_3.png' });

await browser.close();
console.log('DONE');
