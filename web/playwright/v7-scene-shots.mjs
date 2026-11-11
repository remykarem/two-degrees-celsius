// Usage: node playwright/v7-scene-shots.mjs <outDir> [mock|real]. Needs `npm run dev -- --port 5180`. Town v7 estate corner: 4 rotations, lenses, a jet flight.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const sim = process.argv[3] ?? 'real';
const url = `http://localhost:5180/?seed=${process.argv[4] ?? 42}${sim === 'replay' ? '&sim=replay' : ''}`;
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
await page.waitForTimeout(400);
console.log(sim, await page.evaluate(() => document.documentElement.dataset.sim));
const estateAt = () => page.evaluate(() => {
  const s = window.__game.state.get().snapshot;
  const hh = s.households.find((h) => h.id === s.estate.household);
  return window.__scene.peopleScreen().find((p) => p.id === hh.members[0]);
});
const zoomToEstate = async (clicks = 5) => {
  const p = await estateAt();
  await page.mouse.move(p.x, p.y);
  for (let i = 0; i < clicks; i++) await page.mouse.wheel(0, -200);
  await page.waitForTimeout(900);
  // pan the estate (house + runway midpoint) to the middle of the map area, then move the pointer away so no hover card covers it
  const q = await estateAt();
  const cx = 655, cy = 435;
  await page.mouse.move(q.x, q.y); await page.mouse.down(); await page.mouse.move((q.x + cx) / 2, (q.y + cy) / 2, { steps: 5 }); await page.mouse.move(cx, cy + 20, { steps: 5 }); await page.mouse.up();
  await page.mouse.move(1300, 700);
  await page.waitForTimeout(700);
};
await shot('overview');
await zoomToEstate();
for (let i = 0; i < 4; i++) { await shot(`estate-rot${i}`); await g('(game) => game.rotate(1)'); await page.waitForTimeout(1100); }
// the jet at fixed month times (debug hook), then lenses are already shot above
await page.getByRole('button', { name: 'Reset view' }).click();
await page.waitForTimeout(900);
await zoomToEstate(4);
for (const lens of ['emissions', 'class', 'wealth', 'employment', 'flows', 'commute', 'fire']) {
  await g('(game, l) => game.setLens(l)', lens);
  await page.waitForTimeout(500);
  await shot(`estate-lens-${lens}`);
}
await g('(game, l) => game.setLens(l)', 'none');
await page.waitForTimeout(300);
for (const [n, t] of [['roll', 0.2], ['climb', 0.32], ['leaving', 0.38], ['approach', 0.68], ['rollout', 0.8], ['taxi', 0.92]]) {
  await page.evaluate((t) => window.__scene.debugFlight(t), t);
  await page.waitForTimeout(500);
  await shot(`jet-${n}`);
}
await page.evaluate(() => window.__scene.debugFlight(0.3));
await g('(game, l) => game.setLens(l)', 'emissions');
await page.waitForTimeout(500);
await shot('jet-lens-emissions');
await g('(game, l) => game.setLens(l)', 'none');
await page.evaluate(() => window.__scene.debugFlight(null));
await page.waitForTimeout(300);
// play turns until one has a private jet flight, then take frames through it
let found = false;
for (let turn = 0; turn < 8 && !found; turn++) {
  await g('(game) => game.endTurn()');
  await page.waitForFunction(() => window.__game.state.get().phase === 'playback' || window.__game.state.get().phase === 'plan', null, { timeout: 20000 }).catch(() => {});
  const months = await page.evaluate(() => (window.__game.state.get().lastResult?.months ?? []).map((m) => m.events.some((e) => e.type === 'private_jet_flight')));
  const fm = months.indexOf(true);
  console.log('turn', turn, 'flight months', JSON.stringify(months));
  if (fm >= 0) {
    found = true;
    for (let i = 0; i < 40; i++) {
      const st = await page.evaluate(() => ({ m: window.__game.state.get().playback.month, ph: window.__game.state.get().phase }));
      if (st.ph !== 'playback') break;
      if (st.m === fm) { await shot(`flight-a`); await page.waitForTimeout(250); await shot('flight-b'); await page.waitForTimeout(250); await shot('flight-c'); await page.waitForTimeout(250); await shot('flight-d'); await page.waitForTimeout(250); await shot('flight-e'); break; }
      await page.waitForTimeout(100);
    }
  }
  await g('(game) => game.finishPlayback()');
  await page.waitForTimeout(500);
}
await shot('after-flights');
console.log('errors', JSON.stringify(errors));
await browser.close();
