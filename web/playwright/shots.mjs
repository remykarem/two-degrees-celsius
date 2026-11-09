// Usage: node playwright/shots.mjs <outDir> [url]. Dev-only visual check; needs `npm run dev` running.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5173/?seed=42';
mkdirSync(out, { recursive: true });

const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(url);
await page.waitForSelector('canvas');
await page.waitForTimeout(800);
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });

await shot('1-review');
await page.getByRole('button', { name: 'Start planning' }).click();
await page.getByRole('button', { name: /Policies/ }).click();
await page.locator('#lever-carbon_price').fill('60');
await page.getByRole('button', { name: 'Renewable plant' }).click();
await page.mouse.move(700, 450);
await page.waitForTimeout(300);
await page.mouse.click(700, 450);
await page.mouse.move(760, 480);
await page.waitForTimeout(400);
await shot('2-plan');
await page.getByRole('button', { name: 'End turn ▸' }).first().click();
await page.waitForTimeout(3500);
await shot('3-playback');
await page.getByRole('button', { name: /Skip/ }).click();
await page.waitForTimeout(300);
await shot('4-results');
await page.getByRole('button', { name: /Continue/ }).click();
await page.keyboard.press('e');
await page.waitForTimeout(700);
await shot('5-review-rotated');
console.log('console errors:', errors.length ? errors : 'none');
await browser.close();
