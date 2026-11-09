// Usage: node playwright/scene-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5174`. Scene visual check at 1440x900.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5174/?seed=42';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(url);
await page.waitForSelector('canvas');
await page.waitForTimeout(1200);
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
const g = (fn, arg) => page.evaluate(`(${fn})(window.__game, ${JSON.stringify(arg ?? null)})`);
const canvas = page.locator('canvas');

for (let i = 0; i < 4; i++) {
  await shot(`rot${i}`);
  await g('(game) => game.rotate(1)');
  await page.waitForTimeout(900);
}
for (const lens of ['emissions', 'class', 'wealth', 'employment', 'flows']) {
  await g('(game, l) => game.setLens(l)', lens);
  await page.waitForTimeout(500);
  await shot(`lens-${lens}`);
}
await g('(game, l) => game.setLens(l)', 'none');
// zoom in / out with the wheel
await canvas.hover();
for (let i = 0; i < 6; i++) await page.mouse.wheel(0, -200);
await page.waitForTimeout(900);
await shot('zoom-in');
for (let i = 0; i < 14; i++) await page.mouse.wheel(0, 300);
await page.waitForTimeout(900);
await shot('zoom-out');
for (let i = 0; i < 6; i++) await page.mouse.wheel(0, -200);
await page.waitForTimeout(500);
await page.mouse.move(700, 450); await page.mouse.down(); await page.mouse.move(400, 300, { steps: 8 }); await page.mouse.up();
await page.waitForTimeout(500);
await shot('zoom-panned');
await page.evaluate(async () => { (await import('/src/store/scene-bus.ts')).sceneBus.requestReset(); });
await page.waitForTimeout(1500);
await shot('after-reset');
// pending build + build mode
await page.evaluate(() => { window.__game.enterPlan(); });
await g('(game) => game.setBuildKind("renewable_plant")');
await page.mouse.move(720, 470);
await page.waitForTimeout(400);
await shot('build-mode');
await g('(game) => { game.clickTile({ x: 6, y: 8 }); game.setBuildKind("public_housing"); game.clickTile({ x: 3, y: 7 }); game.setBuildKind(null); }');
await page.waitForTimeout(400);
await shot('draft-placed');
await g('(game) => game.endTurn()');
for (const [n, ms] of [['play-a', 900], ['play-b', 1900], ['play-c', 2200]]) { await page.waitForTimeout(ms); await shot(n); }
await g('(game) => game.finishPlayback()');
await page.waitForTimeout(600);
await page.getByRole('button', { name: /Continue/ }).click();
await page.waitForTimeout(900);
await shot('after-continue-pending');
await g('(game) => game.setLens("flows")');
await page.waitForTimeout(1500);
await shot('flows-after-turn');
await g('(game) => game.setLens("employment")');
await page.waitForTimeout(500);
await shot('employment-after-turn');
console.log('stats', JSON.stringify(await page.evaluate(() => window.__game && null)));
console.log('console errors:', errors.length ? errors : 'none');
await browser.close();
