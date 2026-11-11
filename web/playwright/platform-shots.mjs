// Usage: node playwright/platform-shots.mjs <outDir> [url] [W]. Needs `npm run dev -- --port 5184`. Real-sim (wasm) screenshots of Ecology, Finance and Society tabs after a few turns.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5184/?sim=wasm&seed=42';
const W = Number(process.argv[4] ?? 1440);
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const page = await (await browser.newContext({ viewport: { width: W, height: 900 } })).newPage();
page.on('console', (m) => m.type() === 'error' && errors.push(`[console] ${m.text()}`));
page.on('pageerror', (e) => errors.push(`[pageerror] ${String(e)}`));
await page.goto(url);
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});
const startBtn = page.getByRole('button', { name: /start|play|new game/i }).first();
if (await page.getByRole('dialog').count()) await startBtn.click().catch(() => {});
console.log('mock badge present:', (await page.getByText(/mock data/i).count()) > 0);
await page.evaluate(() => { const g = window.__game; for (let i = 0; i < 5; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); } });
await page.waitForTimeout(800);
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});
for (const tab of ['Ecology', 'Finance', 'Society']) {
  await page.getByRole('tab', { name: tab }).click();
  await page.waitForTimeout(400);
  await page.screenshot({ path: `${out}/${tab.toLowerCase()}-${W}.png` });
  await page.locator('aside[aria-label="Metrics"]').evaluate((el) => (el.scrollTop = 99999)).catch(() => {});
  await page.waitForTimeout(200);
  await page.screenshot({ path: `${out}/${tab.toLowerCase()}-${W}-lower.png` });
  await page.locator('aside[aria-label="Metrics"]').evaluate((el) => (el.scrollTop = 0)).catch(() => {});
}
console.log('errors:', errors.length ? errors : 'none');
await browser.close();
