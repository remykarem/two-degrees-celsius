// Usage: node playwright/yield-shots.mjs <outDir> [url] [harvest]. Needs `npm run dev -- --port 5190`. The Ecology sustainable-yield view and the Markets food card
// (with its link to Ecology) on the real simulation. `harvest` raises the commons_harvest lever from turn 1 (the overconsumption run). Pass `?sim=replay` for the recorded demo.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5190/?sim=wasm&seed=42';
const harvest = process.argv[4] ? Number(process.argv[4]) : null;
const turns = Number(process.argv[5] ?? (harvest ? 14 : 8));
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
await page.getByRole('button', { name: 'Start game' }).click({ timeout: 1500 }).catch(() => {});
await page.waitForTimeout(500);
console.log('sim:', await page.evaluate(() => document.documentElement.dataset.sim));
const play = async (n, cmds = []) => {
  for (let i = 0; i < n; i++) {
    const ok = await page.evaluate((c) => {
      const g = window.__game; if (g.state.get().snapshot.gameOver) return false;
      for (const x of c) g.draftCommand(x);
      g.endTurn(); g.finishPlayback(); g.continueFromResults(); return true;
    }, i === 0 && harvest ? [{ type: 'set_policy', lever: 'commons_harvest', value: harvest }] : []);
    if (!ok) break;
  }
  await page.waitForTimeout(300);
};
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
await play(turns);
console.log('turn', await page.evaluate(() => window.__game.state.get().snapshot.turn));
if (await page.locator('.scrim').count()) { await page.keyboard.press('Escape'); await page.waitForTimeout(300); }
await page.getByRole('tab', { name: 'Economy' }).click();
await page.getByTestId('economy-markets').click();
await page.waitForTimeout(400);
const aside = page.locator('aside[aria-label="Metrics"]');
await page.getByTestId('market-food').scrollIntoViewIfNeeded();
await shot('1-markets-food');
await page.getByTestId('market-link-food').click();
await page.waitForTimeout(500);
await shot('2-ecology-top');
await page.getByTestId('yield-panel').scrollIntoViewIfNeeded();
await page.waitForTimeout(200);
await shot('3-yield-panel');
await aside.evaluate((el) => el.scrollBy(0, 600));
await page.waitForTimeout(200);
await shot('4-yield-panel-2');
await aside.evaluate((el) => el.scrollBy(0, 600));
await page.waitForTimeout(200);
await shot('5-yield-panel-3');
console.log(JSON.stringify(await page.evaluate(() => window.__game.state.get().snapshot.natureYield.map((v) => ({ id: v.id, head: v.headline, over: v.overLine, months: v.monthsToCollapse, verdict: v.verdict })))));
console.log(JSON.stringify(await page.evaluate(() => window.__game.state.get().snapshot.markets.filter((m) => m.id === 'food').map((m) => ({ gap: m.gapWords, short: m.short.map((c) => c.label) })))));
await page.getByTestId('howlink-ecology:commons').first().click().catch(() => console.log('no how'));
await page.waitForTimeout(500);
await shot('6-explorer');
console.log('errors:', JSON.stringify(errors));
await browser.close();
