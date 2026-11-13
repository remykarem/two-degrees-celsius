// Usage: node playwright/causal-stage0-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. Causality graph stage 0 on the real simulation after a few turns:
// How it works > Agents (live values beside the upstream causes), a metric "how?" breakdown, the "What changed this turn" label and window, and the Markets view.
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
await page.getByRole('button', { name: 'Start game' }).click({ timeout: 1500 }).catch(() => {});
for (let i = 0; i < 4; i++) {
  await page.evaluate(() => { const g = window.__game; g.endTurn(); g.finishPlayback(); g.continueFromResults(); });
}
await page.waitForTimeout(400);
if (await page.locator('.scrim').count()) await page.keyboard.press('Escape');
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
await page.getByRole('tab', { name: 'Economy' }).click();
await page.waitForTimeout(300);
await shot('1-economy-what-changed');
await page.getByRole('button', { name: 'What changed this turn' }).first().click();
await page.waitForTimeout(300);
await shot('2-what-changed-window');
await page.keyboard.press('Escape');
await page.getByTestId('how-gdp').first().click().catch(() => console.log('no how-gdp'));
await page.waitForTimeout(400);
await shot('3-how-breakdown');
await page.keyboard.press('Escape');
await page.getByTestId('economy-markets').click();
await page.waitForTimeout(400);
await shot('4-markets');
await page.keyboard.press('Escape');
await page.getByTestId('how-it-works').click();
await page.getByTestId('tab-agents').click();
for (const [label, slug] of [['Government', 'government'], ['Firms', 'firms']]) {
  await page.getByRole('button', { name: label, exact: true }).first().click();
  await page.waitForSelector('[data-testid=agent-title]');
  console.log(`${slug} upstream:`, await page.getByTestId('agent-upstream').allInnerTexts());
  await page.getByTestId('agent-decisions').scrollIntoViewIfNeeded();
  await shot(`5-agents-${slug}`);
}
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
