// Usage: node playwright/causal-stage1-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. Causality graph stage 1 on the real simulation after a few turns:
// How it works > Agents > Nature, Weather, Firms and Households: the generated Mechanisms section (plain code, not choices) with its Reads / Writes lines.
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
await page.getByTestId('how-it-works').click();
await page.getByTestId('tab-agents').click();
for (const [label, slug] of [['Nature', 'nature'], ['Weather', 'weather'], ['Firms', 'firms'], ['Households', 'households']]) {
  await page.getByRole('button', { name: new RegExp(`^${label}`) }).first().click();
  await page.waitForSelector('[data-testid=agent-title]');
  const lines = await page.getByTestId('agent-effects').allInnerTexts();
  console.log(`${slug}: ${lines.length} mechanisms`);
  for (const l of lines.slice(0, 3)) console.log('  ', l.slice(0, 200));
  const sec = page.getByTestId('agent-mechanisms');
  if (await sec.count()) { await sec.scrollIntoViewIfNeeded(); await page.getByTestId('agent-effects').first().locator('summary').first().click(); await page.getByTestId('agent-effect').first().locator('summary').click(); }
  await page.screenshot({ path: `${out}/1-agents-${slug}.png` });
}
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
