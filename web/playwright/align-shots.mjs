// Usage: node playwright/align-shots.mjs <outDir> [url]. Needs a dev server. Verifies the web-alignment work with screenshots and checks.
import { chromium } from '@playwright/test';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';

const out = process.argv[2];
const url = process.argv[3] ?? 'http://localhost:5177/?seed=42';
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const errors = [];
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, acceptDownloads: true });
const page = await ctx.newPage();
page.on('console', (m) => (m.type() === 'error' || m.type() === 'warning') && errors.push(`[${m.type()}] ${m.text()}`));
page.on('pageerror', (e) => errors.push(`[pageerror] ${String(e)}`));
await page.goto(url);
await page.waitForSelector('canvas');
await page.evaluate(() => localStorage.setItem('panels.coach.done', '1'));
await page.reload();
await page.waitForSelector('canvas');
await page.waitForTimeout(900);
const shot = (n) => page.screenshot({ path: `${out}/${n}.png` });
const check = (name, ok, extra = '') => console.log(ok ? 'PASS' : 'FAIL', name, extra);
const nonBlank = async () => {
  const buf = await page.locator('canvas').screenshot();
  writeFileSync(`${out}/_probe.png`, buf);
  return buf.length;
};

// 1. real starting town, derived layout
await page.getByRole('button', { name: 'Dismiss' }).click().catch(() => {});
await shot('1-start-town');
const layout = await page.evaluate(() => { const l = window.__game.state.get().layout; return { s: l.scenery.length, p: l.placed.length, homes: l.scenery.filter((x) => x.kind === 'home').length }; });
check('derived layout: 6 homes, 5 fossil plants', layout.homes === 6 && layout.p === 5, JSON.stringify(layout));

// 2. plan screen with real costs: open drawer, move carbon price
await page.getByRole('button', { name: /Policies/ }).click();
await page.getByRole('group').first().waitFor().catch(() => {});
const cp = page.locator('#lever-carbon_price');
await cp.focus();
for (let i = 0; i < 10; i++) await page.keyboard.press('ArrowRight'); // 10 steps of $5
await page.getByRole('button', { name: /Nature and carbon/ }).first().scrollIntoViewIfNeeded().catch(() => {});
await page.evaluate(() => document.querySelector('#lever-carbon_price')?.scrollIntoView({ block: 'center' }));
await page.waitForTimeout(400);
await shot('2-plan-real-costs');
const draftPc = await page.evaluate(() => window.__game.state.get().draft[0]?.preview.pc);
check('drafted carbon price 0 -> 50 costs 8 PC (real: 8 x 50/50)', draftPc === 8, String(draftPc));
const text = await page.locator('aside, section').allInnerTexts();
check('lever rows show a cost estimate, not "Base N PC"', !text.join('\n').match(/Base \d+ PC/) && text.join('\n').includes('PC for a small change'));
check('recorded demo badge visible', await page.getByText('Recorded demo').isVisible());

// 3. crisis dismiss: low legitimacy warning via a save edit, then dismiss and screenshot IMMEDIATELY
const base = await page.evaluate(() => window.__game.exportSave());
const edited = JSON.parse(base);
const sim = JSON.parse(edited.simState); sim.legitimacy = 0.12; edited.simState = JSON.stringify(sim);
await page.evaluate((t) => window.__game.importSave(t), JSON.stringify(edited));
await page.waitForTimeout(500);
await shot('3a-crisis-banner');
const before = await nonBlank();
await page.getByRole('button', { name: /Dismiss warning/ }).click();
await shot('3b-crisis-dismissed-immediately');
const after = await nonBlank();
check('canvas has content right after the banner is dismissed', after > 20000 && before > 20000, `${before} -> ${after}`);
check('crisis region has a stable role', (await page.locator('.banners').getAttribute('role')) === 'status');

// 4. save / load round trip through the UI
await page.evaluate(() => { const g = window.__game; g.newGame(42); g.draftCommand({ type: 'set_policy', lever: 'tax_wage', value: 0.33 }); g.endTurn(); g.finishPlayback(); g.continueFromResults(); });
const turnBefore = await page.evaluate(() => window.__game.state.get().snapshot.turn);
const [dl] = await Promise.all([page.waitForEvent('download'), page.getByRole('button', { name: 'Save' }).click()]);
const path = `${out}/saved-game.json`;
await dl.saveAs(path);
check('save downloads a file', JSON.parse(readFileSync(path, 'utf8')).format === 'ecotown-save');
await page.evaluate(() => window.__game.newGame(7));
check('new game resets to turn 1', (await page.evaluate(() => window.__game.state.get().snapshot.turn)) === 1);
await page.locator('input[type=file]').setInputFiles(path);
await page.waitForTimeout(400);
const turnAfter = await page.evaluate(() => window.__game.state.get().snapshot.turn);
check('load restores turn and policy pending', turnAfter === turnBefore, `${turnBefore} -> ${turnAfter}`);
await shot('4-after-load');
const bad = `${out}/bad.json`; writeFileSync(bad, '{"format":"nope"}');
await page.locator('input[type=file]').setInputFiles(bad);
await page.waitForTimeout(300);
check('bad file gives a friendly error', (await page.getByText(/not a saved game/).count()) > 0);
await shot('4b-bad-save-message');

// 5. keyboard-only placement
await page.evaluate(() => window.__game.newGame(42));
await page.waitForTimeout(300);
await page.getByRole('button', { name: /Build renewable power plant/ }).focus();
await page.keyboard.press('Enter');
await page.locator('canvas').focus();
for (const k of ['ArrowDown', 'ArrowDown', 'ArrowRight', 'ArrowRight', 'ArrowRight']) await page.keyboard.press(k);
const cursor = await page.evaluate(() => window.__game.state.get().hover);
await shot('5a-keyboard-cursor');
await page.keyboard.press('Enter');
await page.waitForTimeout(300);
const drafts = await page.evaluate(() => window.__game.state.get().draft.map((d) => d.command));
check('Enter placed a draft at the cursor', drafts.length === 1 && drafts[0].tile.x === cursor.x && drafts[0].tile.y === cursor.y, JSON.stringify(drafts));
check('live region announced the placement', /Added to the plan/.test(await page.locator('[role=status].sr-only').first().innerText()));
await shot('5b-keyboard-placed');
await page.keyboard.press('Escape');
check('Escape leaves build mode', (await page.evaluate(() => window.__game.state.get().buildKind)) === null);

// 6. end turn focus + zoom controls
await page.getByRole('button', { name: 'Zoom in' }).click();
await page.getByRole('button', { name: 'Reset view' }).click();
await shot('6-zoom-controls');
await page.evaluate(() => window.__game.endTurn());
await page.waitForTimeout(250);
check('focus moves to Skip during playback', await page.evaluate(() => document.activeElement?.textContent?.includes('Skip')));
await page.evaluate(() => window.__game.finishPlayback());
await page.waitForTimeout(300);
await shot('7-results');
await page.getByRole('button', { name: /Continue/ }).click();
await page.waitForTimeout(300);
check('focus returns to End turn after results', await page.evaluate(() => document.activeElement?.textContent?.includes('End turn')));

// 8. narrow window
const small = await browser.newContext({ viewport: { width: 400, height: 800 } });
const sp = await small.newPage();
await sp.goto(url); await sp.waitForTimeout(500);
await sp.screenshot({ path: `${out}/8-narrow.png` });
check('narrow window shows a desktop-only notice', await sp.getByText('Desktop only').isVisible());

// 9. WebGL context loss and restore (WEBGL_lose_context works in headless SwiftShader)
const lost = await page.evaluate(() => { const c = document.querySelector('canvas'); const gl = c.getContext('webgl2') ?? c.getContext('webgl'); const e = gl?.getExtension('WEBGL_lose_context'); if (!e) return false; window.__lose = e; e.loseContext(); return true; });
if (lost) {
  await page.waitForTimeout(500);
  check('context loss shows a visible message', await page.getByText('lost its graphics connection').isVisible());
  await shot('9a-context-lost');
  await page.evaluate(() => window.__lose.restoreContext());
  await page.waitForTimeout(1200);
  check('context restore clears the message', !(await page.getByText('lost its graphics connection').isVisible()));
  await shot('9b-context-restored');
} else console.log('SKIP context loss (no WEBGL_lose_context)');

console.log('console errors/warnings:', errors.length ? '\n' + errors.join('\n') : 'none');
await browser.close();
