// Usage: node playwright/scene-perf.mjs [url]. Reports draw calls, instance counts and frame pacing during playback (software GL, so frame times are pessimistic).
import { chromium } from '@playwright/test';
const url = process.argv[2] ?? 'http://localhost:5174/?seed=42';
const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
await page.goto(url);
await page.waitForSelector('canvas');
await page.waitForTimeout(1000);
await page.evaluate(() => window.__game.endTurn());
const samples = [];
for (let i = 0; i < 12; i++) {
  await page.waitForTimeout(500);
  samples.push(await page.evaluate(async () => {
    const { sceneBus } = await import('/src/store/scene-bus.ts');
    return { ...sceneBus.stats.get(), month: sceneBus.playback.get().month };
  }));
}
const frames = await page.evaluate(() => new Promise((res) => { let n = 0; const t0 = performance.now(); const f = () => { n++; performance.now() - t0 < 2000 ? requestAnimationFrame(f) : res(n / 2); }; requestAnimationFrame(f); }));
console.log('samples', JSON.stringify(samples.slice(0, 12)));
console.log('max instances', Math.max(...samples.map((s) => s.instances)), 'max draw calls', Math.max(...samples.map((s) => s.drawCalls)), 'rAF fps (software GL)', frames);
await browser.close();
