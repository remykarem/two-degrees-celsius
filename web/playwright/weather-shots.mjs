// Usage: node playwright/weather-shots.mjs <outDir> [url]. Needs a dev server (npm run dev -- --port 5183). Town v4 weather screenshots.
// Builds weather situations by editing the exported save (mock only: the blob is the mock's own JSON); on the real sim it only plays and looks.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5183/?sim=replay&seed=7';
const real = !url.includes('sim=replay');
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const page = await ctx.newPage();
page.on('console', (m) => (m.type() === 'error') && errors.push(`[${m.type()}] ${m.text()}`));
page.on('pageerror', (e) => errors.push(`[pageerror] ${String(e)}`));
await page.goto(url);
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(1200);
const prefix = real ? 'real-' : '';
const shot = (n, clip) => page.screenshot({ path: `${out}/${prefix}${n}.png`, ...(clip ? { clip } : {}) });
const check = (name, ok, extra = '') => console.log(ok ? 'PASS' : 'FAIL', name, extra);
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});

/** Edit the mock's state through the save format. */
async function setState(edit) {
  const err = await page.evaluate((src) => {
    const g = window.__game;
    const file = JSON.parse(g.exportSave());
    const blob = JSON.parse(file.simState);
    const forestIdx = (st) => st.grid.map((c, i) => (c.kind === 'forest' ? i : -1)).filter((i) => i >= 0);
    new Function('s', 'forestIdx', `return (${src})(s)`)(blob, forestIdx);
    file.simState = JSON.stringify(blob);
    return g.importSave(JSON.stringify(file));
  }, edit.toString());
  if (err) throw new Error(err);
  await page.waitForTimeout(700);
}

// 1. calm: top bar cluster, Ecology tab
await page.getByRole('tab', { name: 'Ecology' }).click();
await shot('1-calm-ecology');
check('health meters shown', (await page.getByTestId('health-meters').count()) >= 2);
check('stress gauge shown', (await page.getByTestId('stress-gauge').count()) === 1);

if (!real) {
  // 2. stress rising, drought active
  await setState((s) => { s.cumulativeTonnes = 600 * 0.7; s.drought = { severity: 0.8, monthsLeft: 3 }; });
  await shot('2-drought-look');
  await page.getByRole('tab', { name: 'Ecology' }).click();
  await shot('2b-drought-ecology');
  check('drought banner', (await page.getByText(/drought is under way/).count()) >= 1);

  // 3. an active fire with smoke
  await setState((s) => {
    s.cumulativeTonnes = 600 * 0.8; s.drought = { severity: 0.5, monthsLeft: 2 };
    const f = forestIdx(s);
    for (const i of f.slice(0, 2)) { s.grid[i].state = 'burning'; s.grid[i].left = 2; }
    s.grid[f[2]].state = 'regrowing'; s.grid[f[2]].left = 6;
  });
  await shot('3-fire-smoke');
  check('fire banner', (await page.getByText(/Forest fire near/).count()) >= 1);
  await page.getByRole('tab', { name: 'Ecology' }).click();
  await shot('3b-fire-ecology-forecast');
  // zoom on the map
  await page.mouse.move(670, 320);
  for (let i = 0; i < 5; i++) await page.mouse.wheel(0, -300);
  await page.waitForTimeout(500);
  await shot('3c-fire-zoom');
  await page.getByRole('button', { name: 'Reset view' }).click();

  // 4. burnt and regrowing tiles
  await setState((s) => {
    s.cumulativeTonnes = 600 * 0.85;
    const f = forestIdx(s);
    for (const i of f.slice(0, 3)) { s.grid[i].state = 'regrowing'; s.grid[i].left = 8; }
    s.drought = null;
  });
  await shot('4-burnt-regrowing');

  // 5. fire station with coverage ring (select it)
  await setState((s) => {
    const e = s.grid.map((c, i) => (c.kind === 'empty' ? i : -1)).filter((i) => i >= 0);
    const f = forestIdx(s);
    const fx = f.map((i) => [i % 12, Math.floor(i / 12)]);
    const cx = Math.round(fx.reduce((a, p) => a + p[0], 0) / fx.length), cy = Math.round(fx.reduce((a, p) => a + p[1], 0) / fx.length);
    e.sort((a, b) => Math.hypot((a % 12) - cx, Math.floor(a / 12) - cy) - Math.hypot((b % 12) - cx, Math.floor(b / 12) - cy));
    s.grid[e[0]].kind = 'fire_station';
    s.cumulativeTonnes = 600 * 0.75;
  });
  const st = await page.evaluate(() => { const g = window.__game.state.get().snapshot; const t = g.map.tiles.find((x) => x.kind === 'fire_station'); window.__game.select({ kind: 'tile', x: t.x, y: t.y }); return t; });
  await page.waitForTimeout(600);
  await shot('5-fire-station-ring');
  check('station selected', !!st);

  // 6. build palette and Adapt shortcut
  await page.evaluate(() => window.__game.select(null));
  await page.getByRole('tab', { name: 'Ecology' }).click();
  await page.getByRole('button', { name: /Adapt/ }).first().click();
  await page.waitForTimeout(800);
  await shot('6-adapt-shortcut');
  check('fire station build selected', await page.evaluate(() => window.__game.state.get().buildKind === 'fire_station'));
  await page.mouse.move(700, 450);
  await page.waitForTimeout(500);
  await shot('6b-fire-station-hover-ring');
  await page.keyboard.press('Escape');

  // 7. fire lens
  await page.evaluate(() => { window.__game.setBuildKind(null); window.__game.setLens('fire'); });
  await page.waitForTimeout(600);
  await shot('7-fire-lens');
  await page.evaluate(() => window.__game.setLens('none'));
}

// 8. results with the new events: play turns deep in stress (mock only edits the budget; the real sim just plays)
if (!real) await setState((s) => { s.cumulativeTonnes = 600 * 0.72; s.drought = { severity: 0.6, monthsLeft: 2 }; const f = forestIdx(s); s.grid[f[3]].state = 'burning'; s.grid[f[3]].left = 2; });
for (let i = 0; i < 2; i++) {
  await page.evaluate(() => { const g = window.__game; g.endTurn(); g.finishPlayback(); g.continueFromResults(); });
  await page.waitForTimeout(200);
}
await page.evaluate(() => { const g = window.__game; g.endTurn(); g.finishPlayback(); });
await page.waitForTimeout(800);
await shot('8-results');
await page.getByRole('button', { name: /Why/i }).first().click().catch(() => {});
await page.waitForTimeout(400);
await shot('8b-results-why');

// 9. Pressure mode (quarters of about 13 s): weather cards stay non-blocking
const p2 = await ctx.newPage();
p2.on('pageerror', (e) => errors.push(`[pageerror] ${String(e)}`));
await p2.goto(url + '&mode=pressure');
await p2.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await p2.reload();
await p2.waitForSelector('canvas');
await p2.waitForTimeout(1500);
if (!real) {
  await p2.evaluate(() => {
    const g = window.__game, s = g.state.get();
    void s;
  });
}
await p2.getByRole('tab', { name: 'Ecology' }).click();
await p2.waitForTimeout(7000);
await p2.screenshot({ path: `${out}/${prefix}9-pressure.png` });
check('pressure: game keeps playing, no blocking dialog', (await p2.locator('[aria-modal="true"]').count()) === 0);
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
