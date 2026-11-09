// Usage: node playwright/scene-shots2.mjs <outDir> [mock|real]. Needs `npm run dev -- --port 5180`. Scene visual check at 1440x900 on one sim.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const sim = process.argv[3] ?? 'mock';
const url = `http://localhost:5180/?seed=42${sim === 'mock' ? '&sim=mock' : ''}`;
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(url);
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
const shot = (n) => page.screenshot({ path: `${out}/${sim}-${n}.png` });
const g = (fn, arg) => page.evaluate(`(${fn})(window.__game, ${JSON.stringify(arg ?? null)})`);
for (const name of ['Skip guide', 'Dismiss']) { const b = page.getByRole('button', { name }); if (await b.count()) await b.first().click().catch(() => {}); }
await page.waitForTimeout(300);
console.log(sim, await page.evaluate(() => document.documentElement.dataset.sim));

for (let i = 0; i < 4; i++) { await shot(`rot${i}`); await g('(game) => game.rotate(1)'); await page.waitForTimeout(900); }
for (const lens of ['emissions', 'class', 'wealth', 'employment', 'flows', 'commute']) {
  await g('(game, l) => game.setLens(l)', lens);
  await page.waitForTimeout(500);
  await shot(`lens-${lens}`);
}
await g('(game, l) => game.setLens(l)', 'none');
const canvas = page.locator('canvas');
await canvas.hover();
for (let i = 0; i < 6; i++) await page.mouse.wheel(0, -200);
await page.waitForTimeout(900); await shot('zoom-in');
for (let i = 0; i < 14; i++) await page.mouse.wheel(0, 300);
await page.waitForTimeout(900); await shot('zoom-out');
await page.evaluate(async () => { (await import('/src/store/scene-bus.ts')).sceneBus.requestReset(); });
await page.waitForTimeout(1200);
// roads: plan several
await g('(game) => game.enterPlan()');
await g('(game) => game.setBuildKind("road")');
const errs = await g('(game) => { const out = []; const tiles = game.state.get().snapshot.map.tiles.filter((t) => t.kind === "empty").slice(0, 400); for (const t of [[5,6],[5,7],[6,7],[7,7],[7,6],[7,8],[3,5],[4,5],[5,5]]) out.push(game.clickTile({ x: t[0], y: t[1] })); return out; }');
console.log('road clicks', JSON.stringify(errs));
await page.waitForTimeout(500); await shot('roads-drafted');
await g('(game) => { game.setBuildKind("fossil_plant"); game.clickTile({ x: 1, y: 1 }); game.setBuildKind(null); }');
await page.waitForTimeout(400); await shot('plan-all');
await g('(game) => game.endTurn()');
for (const [n, ms] of [['play-a', 900], ['play-b', 1000], ['play-c', 1000], ['play-d', 1000]]) { await page.waitForTimeout(ms); await shot(n); }
await g('(game) => game.finishPlayback()');
await page.waitForTimeout(700);
await shot('after-turn-1');
console.log('stats', JSON.stringify(await page.evaluate(() => window.__game.state.get().phase)));
console.log('errors', JSON.stringify(errors));
await browser.close();
