import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  // GitHub Pages serves a project site from /<repo>/, so CI sets VITE_BASE=/<repo>/.
  base: process.env.VITE_BASE ?? '/',
  plugins: [svelte()],
  test: { include: ['tests/**/*.test.ts'], environment: 'node', testTimeout: 30000, hookTimeout: 30000 },
});
