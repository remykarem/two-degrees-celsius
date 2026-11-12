// Usage: node playwright/corn-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. The commons as a corn field on the real wasm sim:
// a default run, an overconsumption run (commons_harvest raised), stock poked to 0.8/0.5/0.25/0.08 in the live snapshot (dev poke: the snapshot object is
// mutated in place and a draft change re-renders), all 4 rotations, the lens selector, and a draw-call / frame-time report.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5190/?sim=wasm&seed=42';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 2 })).newPage();
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(url);
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
await page.getByRole('button', { name: 'Start game' }).click({ timeout: 1500 }).catch(() => {});
await page.waitForTimeout(800);
console.log('sim:', await page.evaluate(() => document.documentElement.dataset.sim));
const shot = async (n) => { await page.screenshot({ path: `${out}/${n}.png`, scale: 'css' }); await page.screenshot({ path: `${out}/${n}-zoom.png`, clip: { x: 500, y: 420, width: 360, height: 240 } }); };
const poke = async (v, access) => {
  await page.evaluate(([v, access]) => {
    const g = window.__game; const n = g.state.get().snapshot.nature; n.commons = v; if (access) n.commonsAccess = access;
    g.draftCommand({ type: 'set_policy', lever: 'commons_harvest', value: 1.01 }); g.cancelPlan?.();
  }, [v, access]);
  await page.waitForTimeout(1800);
};
const stats = () => page.evaluate(async () => { const { sceneBus } = await import('/src/store/scene-bus.ts'); return sceneBus.stats.get(); });
const frames = () => page.evaluate(() => new Promise((res) => { let n = 0; const t0 = performance.now(); const f = () => { n++; performance.now() - t0 < 2000 ? requestAnimationFrame(f) : res(n / 2); }; requestAnimationFrame(f); }));
console.log('stats default', JSON.stringify(await stats()), 'fps', await frames());
console.log('commons now', await page.evaluate(() => window.__game.state.get().snapshot.nature.commons));
await shot('01-default');
const mode = process.argv[4] ?? 'poke';
if (mode === 'poke') {
  for (const [n, v] of [['02-high', 0.8], ['03-middle', 0.5], ['04-low', 0.25], ['05-verylow', 0.13], ['06-collapsed', 0.05]]) { await poke(v); await shot(n); }
  await poke(0.8, 'community'); await shot('07-community-high'); await poke(0.35, 'community'); await shot('08-community-mid');
  await poke(0.8, 'enclosed'); await shot('09-enclosed-high'); await poke(0.35, 'enclosed'); await shot('10-enclosed-mid');
  await poke(0.5, 'open');
  for (const a of [0, 1, 2, 3]) { await page.evaluate((a) => { const g = window.__game; while (g.state.get().camera !== a) g.rotate(1); }, a); await page.waitForTimeout(900); await shot(`11-rot${a}`); }
  await page.evaluate(() => { const g = window.__game; while (g.state.get().camera !== 0) g.rotate(1); });
  for (const lens of ['wealth', 'emissions', 'fire']) { await page.evaluate((l) => window.__game.setLens?.(l), lens); await page.waitForTimeout(700); await shot(`12-lens-${lens}`); }
  await page.evaluate(() => window.__game.setLens?.('none'));
  console.log('stats after', JSON.stringify(await stats()), 'fps', await frames());
} else {
  await page.evaluate(() => window.__game.draftCommand({ type: 'set_policy', lever: 'commons_harvest', value: 3 }));
  for (let i = 0; i < 30; i++) {
    const r = await page.evaluate(() => { const g = window.__game; if (g.state.get().snapshot.gameOver) return null; g.endTurn(); g.finishPlayback(); g.continueFromResults(); const s = g.state.get().snapshot; return [s.turn, s.nature.commons]; });
    if (!r) break;
    if (i % 4 === 3) { console.log('turn', r[0], 'commons', r[1].toFixed(3)); await page.waitForTimeout(1500); await shot(`20-over-t${r[0]}`); }
  }
}
console.log('errors', JSON.stringify(errors));
await browser.close();
