// Usage: node playwright/panels-shots.mjs <outDir> [url]. Needs a dev server (npm run dev -- --port 5175).
// Takes screenshots of the panels at 1440x900 and 1024x768 and prints console errors.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5175/?seed=42';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];

async function open(w, h, tag, query = '') {
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  const page = await ctx.newPage();
  page.on('console', (m) => m.type() === 'error' && errors.push(`[${tag}] ${m.text()}`));
  page.on('pageerror', (e) => errors.push(`[${tag}] ${String(e)}`));
  await page.goto(url + query);
  await page.waitForSelector('canvas');
  await page.waitForTimeout(800);
  return { page, shot: (n) => page.screenshot({ path: `${out}/${tag}-${n}.png` }) };
}
const play = (page, turns, setup) => page.evaluate(([n, setup]) => {
  const g = window.__game;
  for (let i = 0; i < n && !g.state.get().snapshot.gameOver; i++) {
    if (setup) for (const c of setup) g.draftCommand(c);
    g.endTurn(); g.finishPlayback(); g.continueFromResults();
  }
}, [turns, setup]);

for (const [w, h] of [[1440, 900], [1024, 768]]) {
  const tag = `${w}`;
  // 1. first turn with coach, then drawer, then plan
  let { page, shot } = await open(w, h, tag);
  await shot('01-review-coach');
  for (let i = 0; i < 2; i++) await page.getByRole('button', { name: 'Next' }).click();
  await page.waitForTimeout(300);
  await shot('02-coach-drawer');
  await page.getByRole('button', { name: 'Skip guide' }).click();
  await page.locator('#lever-carbon_price').fill('60');
  await page.locator('#lever-tax_wage').fill('0.3');
  await page.getByRole('button', { name: 'Renewable plant' }).first().click();
  await page.evaluate(() => { window.__game.clickTile({ x: 7, y: 9 }); window.__game.setBuildKind(null); });
  await page.evaluate(() => window.__game.draftCommand({ type: 'remove', kind: 'fossil_plant', count: 1 }));
  await page.waitForTimeout(400);
  await shot('03-plan-drafts');
  await page.getByRole('button', { name: 'Compare systems…' }).click();
  await page.waitForTimeout(200);
  await shot('04-system-modal');
  await page.getByRole('button', { name: /Switch to Regulated/ }).click();
  await shot('05-system-confirm');
  await page.keyboard.press('Escape'); await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Policies ▸' }).click();
  // 2. inspector for several entities
  const picks = [['home', { x: 2, y: 4 }], ['firm', { x: 4, y: 3 }], ['bank', { x: 6, y: 4 }], ['fossil', { x: 10, y: 1 }]];
  const lay = await page.evaluate(() => window.__game.state.get().layout.scenery.filter((s) => ['firm', 'government', 'forest', 'commons'].includes(s.kind)).map((s) => [s.kind, s.x, s.y]));
  const firm = lay.find((l) => l[0] === 'firm'), gov = lay.find((l) => l[0] === 'government');
  for (const [name, t] of [['home', picks[0][1]], ['firm', { x: firm[1], y: firm[2] }], ['bank', picks[2][1]], ['government', { x: gov[1], y: gov[2] }], ['fossil', picks[3][1]]]) {
    await page.evaluate((t) => window.__game.select(t), t);
    await page.waitForTimeout(150);
    await shot(`06-inspector-${name}`);
  }
  await page.evaluate(() => window.__game.select(null));
  // 3. end turn -> results with why
  await page.getByRole('button', { name: 'End turn ▸' }).first().click();
  await page.waitForTimeout(500);
  await shot('07-playback');
  await page.evaluate(() => window.__game.finishPlayback());
  await page.waitForTimeout(300);
  await shot('08-results');
  await page.getByRole('button', { name: /Why did Emissions/ }).click();
  await shot('09-results-why');
  await page.getByRole('button', { name: /Continue/ }).click();
  await play(page, 3);
  await page.getByRole('tab', { name: 'Ecology' }).click();
  await shot('10-metrics-ecology');
  await page.getByRole('tab', { name: 'Society' }).click();
  await page.getByRole('button', { name: /Why is Legitimacy/ }).click();
  await shot('11-why-modal');
  await page.keyboard.press('Escape');
  await page.context().close();

  // 4. crisis: blocked policy + low budget, then end screens
  ({ page, shot } = await open(w, h, tag, '&coachDone=1'));
  await page.evaluate(() => { try { localStorage.setItem('panels.coach.done', '1'); } catch {} });
  await page.reload(); await page.waitForSelector('canvas'); await page.waitForTimeout(600);
  await page.evaluate(() => { const g = window.__game; g.draftCommand({ type: 'switch_system', to: 'coop' }); g.draftCommand({ type: 'switch_system', to: 'regulated' }); });
  await page.evaluate(() => { const g = window.__game; for (let i = 0; i < 30; i++) g.draftCommand({ type: 'place', kind: 'renewable_plant', count: 1, tile: { x: i % 12, y: 11 - Math.floor(i / 12) } }); g.endTurn(); g.finishPlayback(); });
  await page.waitForTimeout(300);
  await page.getByRole('button', { name: /Continue/ }).click();
  await shot('12-crisis-blocked');
  const info = await page.evaluate(() => {
    const g = window.__game; let n = 0;
    while (!g.state.get().snapshot.gameOver && g.state.get().snapshot.indicators.budgetShare > 0.14) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); n++; }
    return { n, share: g.state.get().snapshot.indicators.budgetShare, over: g.state.get().snapshot.gameOver };
  });
  console.log(tag, 'low-budget after', info);
  await page.waitForTimeout(300);
  await shot('13-crisis-low-budget');
  await page.evaluate(() => { const g = window.__game; while (!g.state.get().snapshot.gameOver) { g.endTurn(); g.finishPlayback(); if (g.state.get().snapshot.gameOver) break; g.continueFromResults(); } });
  await page.waitForTimeout(300);
  await page.getByRole('button', { name: /See the ending/ }).click();
  await page.waitForTimeout(300);
  await shot('14-end-lose');
  await page.context().close();

  // 5. win: strong policy from turn 1
  ({ page, shot } = await open(w, h, tag));
  await page.evaluate(() => { try { localStorage.setItem('panels.coach.done', '1'); } catch {} });
  await page.reload(); await page.waitForSelector('canvas'); await page.waitForTimeout(600);
  const win = await page.evaluate(() => {
    const g = window.__game;
    const set = [{ type: 'set_policy', lever: 'carbon_price', value: 200 }, { type: 'set_policy', lever: 'green_subsidy', value: 0.8 }, { type: 'place', kind: 'renewable_plant', count: 1, tile: { x: 7, y: 9 } }];
    let n = 0;
    while (!g.state.get().snapshot.gameOver) {
      if (n % 6 === 0) for (const c of set) g.draftCommand(c.type === 'place' ? { ...c, tile: { x: (n / 6) % 12, y: 10 } } : c);
      g.endTurn(); g.finishPlayback(); if (g.state.get().snapshot.gameOver) break; g.continueFromResults(); n++;
    }
    return g.state.get().snapshot.gameOver;
  });
  console.log(tag, 'win run ended:', JSON.stringify(win));
  await page.getByRole('button', { name: /See the ending/ }).click();
  await page.waitForTimeout(300);
  await shot('15-end-win-or-lose');
  await page.context().close();
}
console.log('console errors:', errors.length ? errors : 'none');
await browser.close();
