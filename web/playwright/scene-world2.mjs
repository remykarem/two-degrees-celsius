// Usage: node playwright/scene-world2.mjs <outDir> <mock|real>. Builds a road network and a renewable plant over 5 turns, then shoots rotations, lenses, zoom. Dev server on 5180.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
const out = process.argv[2], sim = process.argv[3] ?? 'mock';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(`http://localhost:5180/?seed=42${sim === 'mock' ? '&sim=mock' : ''}`);
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
for (const name of ['Skip guide', 'Dismiss']) { const b = page.getByRole('button', { name }); if (await b.count()) await b.first().click().catch(() => {}); }
const shot = (n, full = false) => page.screenshot({ path: `${out}/${sim}-${n}.png`, ...(full ? {} : { clip: { x: 180, y: 50, width: 950, height: 745 } }) });
const ev = (fn) => page.evaluate(fn);
const reset = async () => { await page.evaluate(async () => { (await import('/src/store/scene-bus.ts')).sceneBus.requestReset(); }); await page.waitForTimeout(1200); };

// plan: a road network (plus, T and corner) and a renewable plant
const plan = await ev(() => {
  const g = window.__game;
  g.enterPlan();
  const valid = new Set(g.state.get().validTiles?.keys ?? []);
  g.setBuildKind('road');
  const v = new Set(g.state.get().validTiles.keys);
  const ok = (x, y) => v.has(`${x},${y}`);
  let c = null;
  for (let y = 2; y < 10 && !c; y++) for (let x = 2; x < 9 && !c; x++) if ([[0, 0], [1, 0], [-1, 0], [0, 1], [0, -1], [2, 0], [2, 1], [3, 1], [-2, 0]].every(([dx, dy]) => ok(x + dx, y + dy))) c = { x, y };
  const placed = [];
  if (c) for (const [dx, dy] of [[0, 0], [1, 0], [-1, 0], [0, 1], [0, -1], [2, 0], [2, 1], [3, 1], [-2, 0]]) { const e = g.clickTile({ x: c.x + dx, y: c.y + dy }); if (!e) placed.push([c.x + dx, c.y + dy]); }
  g.setBuildKind('renewable_plant');
  let r = null;
  for (const k of v) { const [x, y] = k.split(',').map(Number); if (!placed.some(([px, py]) => Math.abs(px - x) + Math.abs(py - y) < 2) && x > 6 && y < 4 && !r) r = { x, y }; }
  if (r) g.clickTile(r);
  g.setBuildKind(null);
  return { c, placed: placed.length, r };
});
console.log(sim, 'plan', JSON.stringify(plan));
await reset();
await shot('pending-roads-plant');
const turnOnce = async () => {
  await ev(() => window.__game.endTurn());
  await page.waitForTimeout(300);
  await ev(() => window.__game.finishPlayback());
  await page.waitForTimeout(200);
  await ev(() => window.__game.continueFromResults());
  await page.waitForTimeout(300);
  const ph = await ev(() => window.__game.state.get().phase);
  if (ph === 'review') return;
  if (ph !== 'plan') { await page.waitForTimeout(500); }
};
await turnOnce();
await page.waitForTimeout(500);
await shot('after-turn-1');
for (let i = 0; i < 4; i++) await turnOnce();
await ev(() => { const g = window.__game; if (g.state.get().phase === 'review') g.enterPlan(); });
await reset();
console.log('built', JSON.stringify(await ev(() => { const s = window.__game.state.get().snapshot; return { turn: s.turn, roads: s.map.tiles.filter((t) => t.kind === 'road').length, plants: s.plants.map((p) => [p.kind, p.emissionsMonthly]), pending: s.pending.length }; })));
for (let i = 0; i < 4; i++) { await shot(`world-rot${i}`); await ev(() => window.__game.rotate(1)); await page.waitForTimeout(900); }
for (const lens of ['emissions', 'class', 'wealth', 'employment', 'flows', 'commute']) {
  await page.evaluate(`window.__game.setLens("${lens}")`);
  await page.waitForTimeout(500);
  await shot(`world-lens-${lens}`);
}
await ev(() => window.__game.setLens('none'));
// zoom on the roads and plants
await page.locator('canvas').hover();
for (let i = 0; i < 5; i++) await page.mouse.wheel(0, -180);
await page.waitForTimeout(900);
await shot('world-zoom-in');
for (let i = 0; i < 16; i++) await page.mouse.wheel(0, 300);
await page.waitForTimeout(900);
await shot('world-zoom-out');
await reset();
console.log('stats', JSON.stringify(await ev(async () => { const b = (await import('/src/store/scene-bus.ts')).sceneBus; let v; b.stats.subscribe((x) => (v = x))(); return v; })));
console.log('errors', JSON.stringify(errors));
await browser.close();
