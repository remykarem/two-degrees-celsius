// Usage: node playwright/v52-shots.mjs <outDir> [url]. Needs a dev server (port 5186). Screenshots of the v5.2 transparency views at 1440x900 and 1024x768.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5186/?seed=42&sim=replay';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
for (const [w, h] of [[1440, 900], [1024, 768]]) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  const page = await ctx.newPage();
  page.on('console', (m) => m.type() === 'error' && errors.push(`[${w}] ${m.text()}`));
  page.on('pageerror', (e) => errors.push(`[${w}] pageerror ${String(e)}`));
  await page.goto(url);
  await page.waitForSelector('canvas');
  await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
  await page.reload();
  await page.waitForSelector('canvas');
  await page.waitForTimeout(700);
  await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});
  const shot = (n) => page.screenshot({ path: `${out}/${n}-${w}.png` });
  // play turns (one with a policy change queued, so the Policies tab has content)
  await page.evaluate(() => {
    const g = window.__game;
    const turn = (cmds) => { cmds.forEach((c) => g.draftCommand(c)); g.endTurn(); g.finishPlayback(); g.continueFromResults(); };
    turn([{ type: 'set_policy', lever: 'tax_wage', value: 0.4 }]);
    turn([]); turn([]);
    g.draftCommand({ type: 'set_policy', lever: 'carbon_price', value: 60 });
  });
  await page.waitForTimeout(500);
  const tab = async (name) => { await page.getByRole('tab', { name, exact: true }).click(); await page.waitForTimeout(200); };
  const modalShot = async (n, open) => { await open(); await page.waitForTimeout(250); await shot(n); await page.keyboard.press('Escape'); await page.waitForTimeout(150); };
  await tab('Economy'); await shot('economy');
  await modalShot('breakdown-gdp', () => page.getByTestId('how-gdp').first().click());
  await modalShot('breakdown-debt', () => page.getByTestId('how-debt').first().click());
  await tab('Ecology'); await shot('ecology');
  await page.getByTestId('emissions-panel').scrollIntoViewIfNeeded().catch(() => {});
  await shot('ecology-emissions');
  await tab('Society'); await shot('society');
  await modalShot('gini', () => page.getByTestId('how-gini').first().click());
  await modalShot('legitimacy', () => page.getByTestId('how-legitimacy').first().click());
  await tab('Finance'); await shot('finance');
  await page.getByTestId('taxes-table').scrollIntoViewIfNeeded().catch(() => {});
  await shot('finance-taxes');
  await modalShot('capital-ratio', () => page.getByTestId('how-bank-cr').first().click());
  await tab('People'); await shot('people');
  await tab('Policies'); await shot('policies');
  await tab('Industry'); await shot('industry');
  await modalShot('explorer', () => page.getByTestId('how-it-works').click());
  await modalShot('explorer-link', () => page.getByTestId('how-tax_wage').first().click().catch(() => {}));
  await ctx.close();
}
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
