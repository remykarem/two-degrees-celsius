// Usage: node playwright/causal-stage3-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. Causality graph stage 3 on the real simulation:
// How it works > Agents > Weather, Firms and Nature: the generated "Feedback loops" section (R / B / ? badges, sentences) and the per-node loop summary line.
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
await page.getByTestId('how-it-works').click();
await page.getByTestId('tab-agents').click();
for (const [label, slug] of [['Weather', 'weather'], ['Firms', 'firms'], ['Nature', 'nature']]) {
  await page.getByRole('button', { name: new RegExp(`^${label}`) }).first().click();
  await page.waitForSelector('[data-testid=agent-title]');
  const loops = page.getByTestId('agent-loops');
  const n = await page.getByTestId('agent-loop').count();
  console.log(`${slug}: ${n} loop lines`, (await page.getByTestId('agent-loop-summary').count()), 'summary lines');
  if (await loops.count()) {
    await loops.scrollIntoViewIfNeeded();
    const first = page.getByTestId('agent-loop').first();
    await first.locator('summary').click();
    for (const l of (await page.getByTestId('agent-loop').allInnerTexts()).slice(0, 3)) console.log('  ', l.replace(/\n+/g, ' | ').slice(0, 300));
  }
  await page.screenshot({ path: `${out}/3-loops-${slug}.png` });
}
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
