import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tests that import ./port need the real simulation (the wasm package). Without it they are left out of the run, loudly; CI builds the package first.
const wasmBuilt = existsSync('src/sim-port/wasm/pkg/sim_wasm_bg.wasm');
const needsWasm = readdirSync('tests').filter((f) => f.endsWith('.test.ts') && /from '\.\/port'|sim_wasm\.js/.test(readFileSync(`tests/${f}`, 'utf8'))).map((f) => `tests/${f}`);
if (!wasmBuilt && needsWasm.length) console.warn(`\n[vitest] wasm package not built: skipping ${needsWasm.length} test files that need the real simulation. Run "npm run build:wasm" to run them.\n`);

export default defineConfig({
  // GitHub Pages serves a project site from /<repo>/, so CI sets VITE_BASE=/<repo>/.
  base: process.env.VITE_BASE ?? '/',
  plugins: [svelte()],
  test: { setupFiles: ['tests/setup.ts'], include: ['tests/**/*.test.ts'], exclude: wasmBuilt ? ['node_modules/**'] : ['node_modules/**', ...needsWasm], environment: 'node', testTimeout: 30000, hookTimeout: 30000 },
});
