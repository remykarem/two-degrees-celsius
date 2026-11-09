// Usage: node playwright/panels-a11y.mjs [url]. Checks focus trapping, Escape, focus return, live regions and reduced motion.
import { chromium } from '@playwright/test';

const url = process.argv[2] ?? 'http://localhost:5175/?seed=42';
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, reducedMotion: 'reduce' });
const page = await ctx.newPage();
await page.goto(url);
await page.waitForSelector('canvas');
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
const results = [];
const check = (name, ok) => { results.push([name, ok]); console.log(ok ? 'PASS' : 'FAIL', name); };
const inside = () => page.evaluate(() => !!document.activeElement?.closest('[role=dialog]'));

await page.getByRole('button', { name: /Policies/ }).click();
await page.getByRole('button', { name: 'Compare systems…' }).click();
check('system modal takes focus', await inside());
let always = true;
for (let i = 0; i < 25; i++) { await page.keyboard.press('Tab'); always &&= await inside(); }
for (let i = 0; i < 25; i++) { await page.keyboard.press('Shift+Tab'); always &&= await inside(); }
check('Tab and Shift+Tab stay inside the system modal', always);
await page.keyboard.press('Escape');
check('Escape closes the system modal', (await page.getByRole('dialog').count()) === 0);
check('focus returns to the opener', await page.evaluate(() => document.activeElement?.textContent?.includes('Compare systems')));

await page.evaluate(() => { const g = window.__game; g.endTurn(); g.finishPlayback(); });
check('results modal is open and has focus', (await inside()) && (await page.getByRole('dialog').count()) === 1);
await page.keyboard.press('Escape');
check('Escape does not dismiss the results screen (must continue)', (await page.getByRole('dialog').count()) === 1);
await page.keyboard.press('Tab');
check('focus stays in results on Tab', await inside());
await page.getByRole('button', { name: /Continue/ }).click();
check('review resumes after Continue', (await page.getByRole('dialog').count()) === 0);

await page.getByRole('tab', { name: 'Ecology' }).focus();
await page.keyboard.press('ArrowRight');
check('tabs move with arrow keys', await page.evaluate(() => document.activeElement?.id === 'tab-Society'));
const live = await page.evaluate(() => [...document.querySelectorAll('[aria-live]')].map((e) => e.getAttribute('aria-live')));
check('crisis banner is a live region', live.includes('polite') || live.includes('assertive'));
await page.evaluate(() => { const g = window.__game; g.draftCommand({ type: 'set_policy', lever: 'tax_wage', value: 0.3 }); g.endTurn(); });
check('playback ticker has a live log', (await page.locator('[role=log][aria-live=polite]').count()) === 1);
const anim = await page.evaluate(() => getComputedStyle(document.body).transitionDuration);
check('reduced motion removes transitions', anim === '0s');
await browser.close();
process.exit(results.every(([, ok]) => ok) ? 0 : 1);
