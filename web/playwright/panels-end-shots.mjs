// Usage: node playwright/panels-end-shots.mjs <outDir> [baseUrl]: policy drawer and end screen (plays to the end). Dev server on 5181.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
const out = process.argv[2], base = process.argv[3] ?? 'http://localhost:5181/';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
for (const sim of ['wasm', 'real']) for (const [w, h] of [[1440, 900], [1024, 768]]) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h } });
  await ctx.addInitScript(() => localStorage.setItem('panels.coach.done', '1'));
  const page = await ctx.newPage();
  await page.goto(`${base}?seed=7&sim=${sim}`);
  await page.waitForSelector('canvas'); await page.waitForTimeout(800);
  await page.getByRole('button', { name: /Policies/ }).click(); await page.waitForTimeout(300);
  await page.screenshot({ path: `${out}/${sim}-${w}-10-drawer.png` });
  await page.evaluate(() => { const g = window.__game; for (let i = 0; i < 200 && !g.state.get().snapshot.gameOver; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); } });
  await page.waitForTimeout(800);
  await page.screenshot({ path: `${out}/${sim}-${w}-11-end.png` });
  await ctx.close();
}
await browser.close();
