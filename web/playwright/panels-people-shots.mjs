// Usage: node playwright/panels-people-shots.mjs <outDir> [baseUrl]. Needs a dev server on port 5181.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const base = process.argv[3] ?? 'http://localhost:5181/';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];

async function open(w, h, tag, sim) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  await ctx.addInitScript(() => localStorage.setItem('panels.coach.done', '1'));
  const page = await ctx.newPage();
  page.on('console', (m) => m.type() === 'error' && errors.push(`[${tag}] ${m.text()}`));
  page.on('pageerror', (e) => errors.push(`[${tag}] ${String(e)}`));
  await page.goto(`${base}?seed=7&sim=${sim}`);
  await page.waitForSelector('canvas');
  await page.waitForTimeout(1000);
  return { page, shot: (n) => page.screenshot({ path: `${out}/${tag}-${n}.png` }) };
}
const play = (page, n) => page.evaluate((n) => { const g = window.__game; for (let i = 0; i < n; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); } }, n);

for (const sim of ['real', 'wasm']) {
  for (const [w, h] of [[1440, 900], [1024, 768]]) {
    const tag = `${sim}-${w}`;
    const { page, shot } = await open(w, h, tag, sim);
    await play(page, 3);
    await page.getByRole('tab', { name: 'People' }).click();
    await page.getByRole('button', { name: 'Dismiss' }).first().click().catch(() => {});
    await shot('01-people-list');
    await page.locator('ul[aria-label="Households"] button.row').nth(1).click();
    await shot('02-household-inspector');
    await page.locator('div[aria-label="People"] button.row').first().click();
    await shot('03-person-inspector');
    const unhoused = page.locator('ul[aria-label="Households"] button.unhoused').first();
    if (await unhoused.count()) { await unhoused.click(); await shot('04-unhoused-household'); }
    for (const t of ['Economy', 'Society']) { await page.getByRole('tab', { name: t }).click(); await shot(`05-metrics-${t}`); }
    await page.evaluate(() => window.__game.select(null));
    await page.getByRole('button', { name: /fossil/i }).first().click();
    await shot('06-plan-costs');
    await page.evaluate(() => { const g = window.__game; g.setBuildKind(null); const t = g.state.get().snapshot.map.tiles.find((x) => x.kind === 'empty'); g.draftCommand({ type: 'place', kind: 'road', tile: { x: t.x, y: t.y } }); g.endTurn(); });
    await page.waitForTimeout(1500);
    await shot('07-playback');
    await page.evaluate(() => window.__game.finishPlayback());
    await page.waitForTimeout(500);
    await shot('08-results');
    await page.evaluate(() => { const g = window.__game; g.continueFromResults(); for (let i = 0; i < 40 && !g.state.get().snapshot.gameOver; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); } });
    await page.waitForTimeout(800);
    await shot('09-end');
  }
}
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
