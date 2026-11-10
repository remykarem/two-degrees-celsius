// Usage: node playwright/weather-real.mjs <outDir> [url]. The REAL simulation (wasm): plays a no-action game and screenshots the first drought and the first fire.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5183/?sim=wasm&seed=7';
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
console.log('sim:', await page.evaluate(() => document.documentElement.dataset.sim + ' schema ' + window.__game.state.get().snapshot.schemaVersion));
const shot = (n) => page.screenshot({ path: `${out}/real-${n}.png` });
const info = () => page.evaluate(() => { const s = window.__game.state.get().snapshot; return { turn: s.turn, stress: s.weather.stress, drought: s.weather.drought, fires: s.weather.fires.length, forecast: s.weather.forecast, budget: s.indicators.budgetShare, over: !!s.gameOver }; });
await page.getByRole('tab', { name: 'Ecology' }).click();
await shot('1-start');
let sawDrought = false, sawFire = false, sawForecast = false;
const events = new Set();
for (let i = 0; i < 25; i++) {
  const r = await page.evaluate(() => { const g = window.__game; const res = g.endTurn(); g.finishPlayback(); g.continueFromResults(); return res ? res.months.flatMap((m) => m.events.map((e) => e.type)) : null; });
  if (r) r.forEach((t) => events.add(t));
  await page.waitForTimeout(250);
  const s = await info();
  console.log(JSON.stringify(s));
  if (!sawForecast && (s.forecast.fireRisk !== 'low' || s.forecast.droughtRisk !== 'low')) { sawForecast = true; await shot('2-forecast'); }
  if (!sawDrought && s.drought) { sawDrought = true; await shot('3-drought'); }
  if (!sawFire && s.fires) { sawFire = true; await shot('4-fire'); await page.mouse.move(670, 330); for (let k = 0; k < 4; k++) await page.mouse.wheel(0, -300); await page.waitForTimeout(500); await shot('4b-fire-zoom'); await page.getByRole('button', { name: 'Reset view' }).click(); }
  if (s.over) break;
}
console.log('event types seen:', [...events].join(', '));
await shot('9-end');
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
