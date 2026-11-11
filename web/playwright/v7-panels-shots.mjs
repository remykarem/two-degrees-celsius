// Usage: node playwright/v7-panels-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5191`. REAL simulation (wasm): Town v7 panels
// (Society estate card, Finance, Ecology aviation line, Policies, Industry, Gini view, inspector for the estate owner and the pilot).
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5191/?sim=wasm&seed=7';
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
console.log('sim:', await page.evaluate(() => document.documentElement.dataset.sim));
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
const play = (n) => page.evaluate((k) => { const g = window.__game; for (let i = 0; i < k; i++) { g.endTurn(); g.finishPlayback(); g.continueFromResults(); } }, n);
await play(6);
const ids = await page.evaluate(() => { const s = window.__game.state.get().snapshot; return { owner: s.people.find((p) => p.household === s.estate.household).id, pilot: s.estate.pilot.person, flights: s.estate.flightsTotal }; });
console.log(JSON.stringify(ids));
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});
const scrollTo = async (sel) => { await page.evaluate((q) => { const e = document.querySelector(q); e?.scrollIntoView({ block: 'start' }); }, sel); await page.waitForTimeout(200); };
for (const t of ['Society', 'Finance', 'Ecology', 'Policies', 'Industry']) { await page.getByRole('tab', { name: t }).click(); await page.waitForTimeout(300); await shot(t.toLowerCase());
  if (t === 'Society') { await scrollTo('[data-testid=estate-card] h4:nth-of-type(4)'); await shot('society-scrolled'); }
  if (t === 'Finance') { await scrollTo('[data-testid=finance-estate]'); await shot('finance-estate'); await scrollTo('#fin-households'); await shot('finance-households'); }
  if (t === 'Ecology') { await scrollTo('[data-testid=emissions-panel]'); await shot('ecology-emissions'); }
}
await page.getByRole('tab', { name: 'Society' }).click();
await page.evaluate((id) => window.__game.select({ kind: 'person', id }), ids.owner); await page.waitForTimeout(400); await shot('inspector-owner');
await page.evaluate(() => { const a = document.querySelector('aside[aria-label=Inspector]'); if (a) a.scrollTop = 400; }); await shot('inspector-owner-2');
await page.evaluate((id) => window.__game.select({ kind: 'person', id }), ids.pilot); await page.waitForTimeout(400); await shot('inspector-pilot');
await page.evaluate(() => { const a = document.querySelector('aside[aria-label=Inspector]'); if (a) a.scrollTop = 0; });
await page.evaluate(() => window.__game.select(null));
await page.getByRole('button', { name: /how/i }).first().click().catch(() => {});
await page.keyboard.press('Escape');
// Gini view: the "how?" button of Inequality in the Society tab
const how = page.getByRole('button', { name: /Inequality/ });
console.log('inequality how buttons', await how.count());
await how.first().click().catch((e) => console.log('no gini button', String(e).slice(0, 80)));
await page.waitForTimeout(500); await shot('gini');
await page.keyboard.press('Escape');
// Squeeze the estate: a 5% wealth tax from zero, then play on until its savings fall short (the "paid in part" note) and the pilot may be short-paid.
await page.evaluate(() => { const g = window.__game; g.draftCommand({ type: 'set_policy', lever: 'wealth_tax', value: 0.05 }); g.draftCommand({ type: 'set_policy', lever: 'wealth_threshold', value: 0 }); });
await play(10);
console.log(JSON.stringify(await page.evaluate(() => { const s = window.__game.state.get().snapshot; return { turn: s.turn, pilot: s.estate.pilot, savings: s.households.find((h) => h.id === s.estate.household).savings, fin: s.estate.finance.month }; })));
await page.getByRole('tab', { name: 'Policies' }).click(); await page.waitForTimeout(300); await shot('policies-squeezed');
await page.getByRole('tab', { name: 'Society' }).click(); await page.waitForTimeout(300); await shot('society-squeezed');
await page.getByRole('tab', { name: 'Finance' }).click(); await scrollTo('[data-testid=finance-estate]'); await shot('finance-squeezed');
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
