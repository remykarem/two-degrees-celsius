// Usage: node playwright/decisions-shots.mjs <outDir> [url]. Needs `npm run dev -- --port 5190`. How it works > Agents > Government, Estate and Nature on the real simulation:
// the generated "Who decides" lines, the Decisions list (inputs -> decision -> outputs) and the live parameters (contract v2.7).
import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5190/?sim=wasm&seed=42';
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
console.log('sim:', await page.evaluate(() => document.documentElement.dataset.sim));
await page.getByTestId('how-it-works').click();
await page.getByTestId('tab-agents').click();
for (const [label, slug] of [['Government', 'government'], ['The estate and its pilot', 'estate'], ['Nature', 'nature']]) {
  await page.getByRole('button', { name: label, exact: true }).first().click();
  await page.waitForSelector('[data-testid=agent-decisions]');
  console.log(`${slug} decisions:`, await page.getByTestId('agent-decision').evaluateAll((els) => els.map((e) => e.getAttribute('data-id'))));
  await page.screenshot({ path: `${out}/agents-${slug}.png` });
  await page.getByTestId('agent-decisions').scrollIntoViewIfNeeded();
  await page.getByTestId('agent-decision').first().locator('summary').click();
  await page.screenshot({ path: `${out}/agents-${slug}-decisions.png` });
}
console.log('errors:', errors);
await browser.close();
