// Usage: node playwright/v5-shots.mjs <outDir> [url] [WxH]. Needs a dev server (npm run dev -- --port 5184). Town v5 screenshots: Finance tab, ledgers, energy.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5184/?sim=mock&seed=42';
const [W, H] = (process.argv[4] ?? '1440x900').split('x').map(Number);
const real = !url.includes('sim=mock');
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const ctx = await browser.newContext({ viewport: { width: W, height: H } });
const page = await ctx.newPage();
page.on('console', (m) => (m.type() === 'error') && errors.push(`[${m.type()}] ${m.text()}`));
page.on('pageerror', (e) => errors.push(`[pageerror] ${String(e)}`));
await page.goto(url);
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(1200);
const tag = `${real ? 'real' : 'mock'}-${W}`;
const shot = (n) => page.screenshot({ path: `${out}/${tag}-${n}.png` });
const check = (name, ok, extra = '') => console.log(ok ? 'PASS' : 'FAIL', name, extra);
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});
const startBtn = page.getByRole('button', { name: /start|play|new game/i }).first();
if (await page.getByRole('dialog').count()) await startBtn.click().catch(() => {});

// play four turns quickly through the store
await page.evaluate(() => { const g = window.__game; for (let i = 0; i < 4; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); } });
await page.waitForTimeout(600);
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});

await page.getByRole('tab', { name: 'Finance' }).click();
await page.waitForTimeout(300);
await shot('1-finance');
check('finance panel', (await page.getByTestId('finance-panel').count()) === 1);
await page.locator('aside[aria-label="Metrics"]').evaluate((el) => (el.scrollTop = 700));
await shot('1b-finance-lower');
await page.locator('aside[aria-label="Metrics"]').evaluate((el) => (el.scrollTop = 99999));
await shot('1c-finance-flow');

// household ledger
await page.evaluate(() => window.__game.select({ kind: 'household', id: 1 }));
await page.waitForTimeout(300);
await shot('2-household-inspector');
await page.getByRole('button', { name: /Open full ledger/ }).click();
await page.waitForTimeout(300);
await shot('2b-household-ledger');
check('reconciliation badge', (await page.getByTestId('reconciliation').innerText()).includes('Reconciled'));
await page.keyboard.press('Escape');

// government ledger via Finance tab
await page.getByRole('button', { name: 'Government ledger' }).click();
await page.waitForTimeout(300);
await shot('3-government-ledger');
await page.keyboard.press('Escape');
// firm ledger
await page.evaluate(() => window.__game.openLedger({ kind: 'firm', id: 2 }));
await page.waitForTimeout(300);
await shot('4-firm-ledger');
await page.keyboard.press('Escape');
// person
await page.evaluate(() => { const g = window.__game; const s = g.state.get().snapshot; g.select({ kind: 'person', id: s.people.find((p) => p.job).id }); });
await page.waitForTimeout(300);
await shot('5-person-earnings');
// energy
await page.evaluate(() => window.__game.select(null));
await page.getByRole('tab', { name: 'Ecology' }).click();
await page.locator('aside[aria-label="Metrics"]').evaluate((el) => (el.scrollTop = 99999));
await page.waitForTimeout(300);
await shot('6-energy');
check('energy panel', (await page.getByTestId('energy-panel').count()) === 1);
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
