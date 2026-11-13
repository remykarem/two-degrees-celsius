// Usage: node playwright/causal-stage5-shots.mjs <outDir> [url] [WxH]. Needs `npm run dev -- --port 5199`. Causality graph stage 5 (strength: thickness, numbers, legend) on the real simulation.
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5199/?sim=wasm&seed=7';
const [W, H] = (process.argv[4] ?? '1440x900').split('x').map(Number);
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const page = await (await browser.newContext({ viewport: { width: W, height: H } })).newPage();
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto(url);
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(1500);
await page.getByRole('button', { name: 'Start game' }).click({ timeout: 1500 }).catch(() => {});
for (let i = 0; i < 4; i++) await page.evaluate(() => { const g = window.__game; g.endTurn(); g.finishPlayback(); g.continueFromResults(); });
await page.waitForTimeout(400);
if (await page.locator('.scrim').count()) await page.keyboard.press('Escape');
const tag = `${W}`;
const clickTab = async (name) => page.getByRole('tab', { name }).click().catch(() => {});
const report = async (label) => {
  const nodes = await page.getByTestId('cause-node').count();
  const items = await page.getByTestId('cause-item').count();
  const edges = await page.getByTestId('cause-edge').count();
  console.log(label, { nodes, items, edges });
};
async function open(testId, tab) {
  if (tab) await clickTab(tab);
  await page.getByTestId(testId).first().click();
  await page.waitForSelector('[data-testid=causes-modal]');
  await page.waitForTimeout(250);
}
await clickTab('Economy');
await page.screenshot({ path: `${out}/${tag}-panel-economy.png` });
const cases = [
  ['unemployment', 'why-unemployment', 'Economy'],
  ['net-emissions', 'why-emissions', 'Ecology'],
  ['wage-market', 'why-wage', 'Economy', 'economy-markets'],
  ['legitimacy', 'why-legitimacy', 'Society'],
];
for (const [name, id, tab, pre] of cases) {
  if (tab) await clickTab(tab);
  if (pre) await page.getByTestId(pre).click().catch(() => {});
  await open(id, null);
  for (const d of [1, 2, 3]) {
    await page.getByTestId(`depth-${d}`).click();
    await page.waitForTimeout(150);
    await page.screenshot({ path: `${out}/${tag}-${name}-d${d}.png` });
    await report(`${name} depth ${d} graph`);
  }
  await page.getByTestId('causes-toggle-list').click();
  await page.waitForTimeout(150);
  await report(`${name} depth 3 list`);
  await page.screenshot({ path: `${out}/${tag}-${name}-d3-list.png` });
  await page.keyboard.press('Escape');
  await page.waitForTimeout(150);
}
// Keyboard: Tab to a cause, arrows follow edges, Enter re-centres, Back returns, Escape closes.
await clickTab('Economy');
await page.getByTestId('economy-markets').click().catch(() => {});
await page.getByRole('button', { name: 'Overview' }).click().catch(() => {});
await page.getByTestId('why-unemployment').first().click();
await page.waitForSelector('[data-testid=causes-modal]');
await page.getByTestId('depth-2').click();
if (!(await page.getByTestId('cause-node').count())) await page.getByTestId('causes-toggle-list').click();
await page.locator('[data-testid=cause-node][data-focus=true]').focus();
await page.keyboard.press('ArrowLeft');
const focusedId = () => page.evaluate(() => document.activeElement?.getAttribute('data-id'));
console.log('after ArrowLeft focus on', await focusedId());
await page.keyboard.press('Enter');
await page.waitForTimeout(250);
console.log('recentred on', await page.getByTestId('causes-modal').getAttribute('data-focus-id'), 'focus now on', await focusedId());
await page.screenshot({ path: `${out}/${tag}-recentred.png` });
await page.getByTestId('causes-back').click();
console.log('back to', await page.getByTestId('causes-modal').getAttribute('data-focus-id'));
await page.keyboard.press('Escape');
console.log('closed', (await page.getByTestId('causes-modal').count()) === 0);
console.log(errors.length ? errors.join('\n') : 'no console errors');
await browser.close();
