// Usage: node playwright/markets-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. The Markets view (Economy tab, Markets segment) on the real simulation after several turns and
// a carbon price shock, in scroll steps; the system-switch modal's coordination note; and the Industry link. Pass `?sim=replay` for the recorded demo (no shock is possible there).
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5190/?sim=wasm&seed=7';
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
const play = async (n, cmds = []) => {
  for (let i = 0; i < n; i++) {
    const ok = await page.evaluate((c) => {
      const g = window.__game; if (g.state.get().snapshot.gameOver) return false;
      for (const x of c) g.draftCommand(x);
      g.endTurn(); g.finishPlayback(); g.continueFromResults(); return true;
    }, i === 0 ? cmds : []);
    if (!ok) break;
  }
  await page.waitForTimeout(300);
};
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
await play(3);
if (!demo) await play(1, [{ type: 'set_policy', lever: 'carbon_price', value: 300 }]);
await play(demo ? 2 : 6);
console.log('turn', await page.evaluate(() => window.__game.state.get().snapshot.turn));
if (await page.locator('.scrim').count()) { await shot('0-modal-in-the-way'); await page.keyboard.press('Escape'); await page.waitForTimeout(300); }
await page.getByRole('tab', { name: 'Economy' }).click();
await page.getByTestId('economy-markets').click();
await page.waitForTimeout(400);
await shot('1-markets-top');
const aside = page.locator('aside[aria-label="Metrics"]');
for (let i = 2; i <= 6; i++) {
  await aside.evaluate((el) => el.scrollBy(0, 700));
  await page.waitForTimeout(200);
  await shot(`${i}-markets-scroll`);
}
console.log(JSON.stringify(await page.evaluate(() => window.__game.state.get().snapshot.markets.map((m) => ({ id: m.id, d: m.demandText, s: m.supplyText, gap: m.gapWords, price: m.priceText, dir: m.priceDirection, rec: m.reconciles })))));
await page.getByTestId('how-markets:food').first().click().catch(() => console.log('no how'));
await page.waitForTimeout(500);
await shot('7-how-food');
await page.keyboard.press('Escape');
await page.getByRole('tab', { name: 'Industry' }).click();
await page.waitForTimeout(300);
await shot('8-industry-link');
await page.getByRole('tab', { name: 'Economy' }).click();
await page.getByRole('button', { name: /^Policies/ }).first().click().catch(() => console.log('no policies button'));
await page.getByRole('button', { name: /Compare systems/ }).click().catch(() => console.log('no system button'));
await page.waitForTimeout(400);
await shot('9-system-modal');
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
