// Usage: node playwright/trades-shots.mjs <outDir> [url]. Needs `npm run dev`. The "See all trades" drill-down of the Markets view on the real simulation after several turns
// (Food, Labour, Housing, Credit, a sorted column, a household's own trades in the inspector); pass `?sim=replay` for the recorded demo (the button explains the list needs the live sim).
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5173/?sim=wasm&seed=7';
const demo = url.includes('sim=replay');
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
for (let i = 0; i < (demo ? 3 : 9); i++) {
  const ok = await page.evaluate(() => { const g = window.__game; if (g.state.get().snapshot.gameOver) return false; g.endTurn(); g.finishPlayback(); g.continueFromResults(); return true; });
  if (!ok) break;
}
await page.waitForTimeout(300);
console.log('turn', await page.evaluate(() => window.__game.state.get().snapshot.turn));
if (await page.locator('.scrim').count()) { await page.keyboard.press('Escape'); await page.waitForTimeout(300); }
await page.getByRole('tab', { name: 'Economy' }).click();
await page.getByTestId('economy-markets').click();
await page.waitForTimeout(400);
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
const aside = page.locator('aside[aria-label="Metrics"]');
await page.getByTestId('trades-open-food').scrollIntoViewIfNeeded();
await shot('1-markets-card-with-button');
for (const id of demo ? ['food'] : ['food', 'labour', 'housing', 'credit']) {
  await page.getByTestId(`trades-open-${id}`).click();
  await page.waitForTimeout(400);
  await shot(`2-${id}`);
  if (id === 'food' && !demo) {
    await page.getByTestId('sort-units').click(); await page.getByTestId('sort-units').click();
    await page.waitForTimeout(200); await shot('3-food-sorted-units-desc');
    await page.getByTestId('trades-period').selectOption({ index: 0 }); await page.waitForTimeout(300); await shot('4-food-whole-turn');
    await page.getByTestId('trades-period').selectOption({ index: await page.locator('[data-testid=trades-period] option').count() - 1 }); await page.waitForTimeout(300); await shot('4b-food-oldest-rolled-up');
  }
  console.log(id, JSON.stringify(await page.evaluate(() => ({ rows: document.querySelectorAll('[data-testid=trade-row]').length, rec: document.querySelector('[data-testid=trades-reconcile]')?.textContent?.trim(), tot: document.querySelector('[data-testid=trades-total-text]')?.textContent?.trim(), err: document.querySelector('[data-testid=trades-error]')?.textContent?.trim() }))));
  await page.keyboard.press('Escape');
  await page.waitForTimeout(250);
}
if (!demo) {
  await page.getByRole('button', { name: 'Dismiss' }).click({ timeout: 1000 }).catch(() => {});
  await page.evaluate(() => window.__game.select({ kind: 'household', id: 0 }));
  await page.waitForTimeout(400);
  await page.getByTestId('entity-trades').scrollIntoViewIfNeeded();
  await shot('5-inspector-household-trades');
  console.log('inspector', await page.evaluate(() => document.querySelector('[data-testid=entity-trades]')?.textContent?.slice(0, 400)));
  await page.getByTestId('entity-trades-food').getByRole('button').first().click();
  await page.waitForTimeout(400); await shot('6-food-filtered-to-household');
}
console.log('console errors:', JSON.stringify(errors));
await browser.close();
