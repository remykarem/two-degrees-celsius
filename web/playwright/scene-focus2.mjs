// Usage: node playwright/scene-focus2.mjs <outDir> <replay|real>. Close-ups: idle people, mid-commute, smoke, camp, pending road, keyboard cursor.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const sim = process.argv[3] ?? 'real';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(`http://localhost:5180/?seed=42${sim === 'replay' ? '&sim=replay' : ''}`);
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
for (const name of ['Skip guide', 'Dismiss']) { const b = page.getByRole('button', { name }); if (await b.count()) await b.first().click().catch(() => {}); }
const shot = (n) => page.screenshot({ path: `${out}/${sim}-${n}.png`, clip: { x: 180, y: 50, width: 950, height: 745 } });
const g = (fn, arg) => page.evaluate(`(${fn})(window.__game, ${JSON.stringify(arg ?? null)})`);
const bus = () => page.evaluate(async () => { const b = (await import('/src/store/scene-bus.ts')).sceneBus; let v; b.stats.subscribe((x) => (v = x))(); return v; });
console.log(sim, await page.evaluate(() => document.documentElement.dataset.sim));
const zoom = async (n) => { await page.locator('canvas').hover(); for (let i = 0; i < n; i++) await page.mouse.wheel(0, -150); await page.waitForTimeout(800); };
await shot('idle-fit');
await zoom(3);
await shot('idle-closeup');
// frame time under software GL: count rAF frames over 3 s while playing
await page.evaluate(async () => { (await import('/src/store/scene-bus.ts')).sceneBus.requestReset(); });
await g('(game) => game.setLens("class")');
await page.waitForTimeout(500);
await shot('people-class-lens');
await g('(game) => game.setLens("none")');
await g('(game) => game.endTurn()');
await page.waitForTimeout(400);
const frames = await page.evaluate(() => new Promise((res) => { let n = 0; const t0 = performance.now(); const f = () => { n++; if (performance.now() - t0 < 2500) requestAnimationFrame(f); else res({ n, ms: performance.now() - t0 }); }; requestAnimationFrame(f); }));
console.log('rAF during playback', JSON.stringify(frames), 'frame ms', (frames.ms / frames.n).toFixed(1));
console.log('stats', JSON.stringify(await bus()));
await zoom(2);
await shot('mid-commute-a');
await page.waitForTimeout(900);
await shot('mid-commute-b');
await g('(game) => game.finishPlayback()');
await page.waitForTimeout(500);
await g('(game) => game.continueFromResults()');
await page.waitForTimeout(500);
await shot('after-turn');
console.log('errors', JSON.stringify(errors));
await browser.close();
