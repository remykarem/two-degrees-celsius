// Usage: node playwright/v6-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. REAL simulation (wasm): screenshots the views Town v6 changed
// (Ecology tab, system switch modal, policy drawer, metrics, weather lens, end screen) and reports console errors.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5190/?sim=wasm&seed=7';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const page = await (await browser.newContext({ viewport: { width: 1440, height: 900 } })).newPage();
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(url);
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
console.log('sim:', await page.evaluate(() => document.documentElement.dataset.sim));
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
await page.getByRole('tab', { name: 'Ecology' }).click();
await shot('1-ecology');
await page.getByRole('tab', { name: 'Economy' }).click();
await shot('2-metrics');
await page.getByRole('button', { name: /Policies/ }).first().click().catch(() => {});
await page.waitForTimeout(400);
await shot('3-policies');
await page.keyboard.press('Escape');
await page.evaluate(() => { window.__game.state.set; });
// system switch modal
const sys = page.getByRole('button', { name: /system/i }).first();
await sys.click().catch(() => console.log('no system button'));
await page.waitForTimeout(400);
await shot('4-system-modal');
await page.keyboard.press('Escape');
// weather lens
await page.evaluate(() => window.__game.setLens('fire'));
await page.waitForTimeout(600);
await shot('5-fire-lens');
await page.evaluate(() => window.__game.setLens('none'));
// play to the end
for (let i = 0; i < 40; i++) {
  const over = await page.evaluate(() => { const g = window.__game; if (g.state.get().snapshot.gameOver) return true; g.endTurn(); g.finishPlayback(); g.continueFromResults(); return !!g.state.get().snapshot.gameOver; });
  if (over) break;
  if (i === 14) { await page.waitForTimeout(300); await page.getByRole('tab', { name: 'Ecology' }).click(); await shot('6-ecology-mid'); await page.evaluate(() => window.__game.setLens('fire')); await page.waitForTimeout(500); await shot('7-fire-lens-mid'); await page.evaluate(() => window.__game.setLens('none')); }
}
await page.waitForTimeout(800);
await shot('8-end');
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
