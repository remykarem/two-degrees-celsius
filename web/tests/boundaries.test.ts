import { ESLint } from 'eslint';
import { describe, expect, it } from 'vitest';

const eslint = new ESLint({ cwd: process.cwd() });
const restricted = (r: { messages: { ruleId: string | null; message: string }[] }) => r.messages.filter((m) => m.ruleId === 'no-restricted-imports' || m.ruleId === 'boundary/dynamic-import');

describe('module boundaries (ESLint no-restricted-imports)', () => {
  it('the current source tree has no boundary violations', async () => {
    const results = await eslint.lintFiles(['src/**/*.ts', 'src/**/*.svelte']);
    const bad = results.flatMap((r) => restricted(r).map((m) => `${r.filePath}: ${m.message}`));
    expect(bad).toEqual([]);
  });

  const violations: [string, string, string][] = [
    ['src/scene/x.ts', "import { x } from 'svelte';", 'scene -> svelte'],
    ['src/scene/x.ts', "import X from '../panels/TopBar.svelte';", 'scene -> panels'],
    ['src/panels/x.ts', "import * as T from 'three';", 'panels -> three'],
    ['src/panels/x.ts', "import { createScene } from '../scene';", 'panels -> scene'],
    ['src/contract/x.ts', "import type { A } from './index';", 'contract imports something'],
    ['src/contract/x.ts', "import type { B } from '../store';", 'contract -> store'],
    ['src/store/x.ts', "import * as T from 'three';", 'store -> three'],
    ['src/store/x.ts', "import { createSimPort } from '../sim-port';", 'store -> sim-port factory'],
    ['src/store/x.ts', "import { createMockSim } from '../sim-port/mock';", 'store -> mock'],
    ['src/app/x.ts', "import { createMockSim } from '../sim-port/mock';", 'app -> mock'],
    ['src/panels/x.ts', "import data from '../sim-port/mock/data.json';", 'panels -> mock internals'],
    ['src/sim-port/x.ts', "import { writable } from '../store';", 'sim-port -> store'],
    ['src/sim-port/mock/x.ts', "import { createSimPort } from '../index';", 'mock -> factory'],
    // store may only reach sim-port/port
    ['src/store/x.ts', "import { createMockSim } from '../sim-port/mock/index';", 'store -> mock/index'],
    ['src/store/x.ts', "import type { X } from '../sim-port/other';", 'store -> other sim-port subpath'],
    ['src/store/x.ts', "import { createSimPort } from '../sim-port/index';", 'store -> sim-port/index'],
    ['src/store/nested/x.ts', "import { createSimPort } from '../../sim-port';", 'nested store -> sim-port'],
    // dynamic import() crosses boundaries too
    ['src/scene/x.ts', "export const f = () => import('../panels/TopBar.svelte');", 'scene -> panels (dynamic)'],
    ['src/scene/x.ts', "export const f = () => import('svelte');", 'scene -> svelte (dynamic)'],
    ['src/panels/x.ts', "export const f = () => import('three');", 'panels -> three (dynamic)'],
    ['src/panels/x.ts', "export const f = () => import('../scene');", 'panels -> scene (dynamic)'],
    ['src/panels/x.ts', "export const f = () => import('../sim-port/port');", 'panels -> sim-port (dynamic)'],
    ['src/ui-kit/x.ts', "export const f = () => import('../store');", 'ui-kit -> store (dynamic)'],
    ['src/store/x.ts', "export const f = () => import('../sim-port/mock');", 'store -> mock (dynamic)'],
    ['src/store/x.ts', "export const f = () => import('../sim-port');", 'store -> sim-port factory (dynamic)'],
    ['src/store/x.ts', "export const f = () => import('three');", 'store -> three (dynamic)'],
    ['src/app/x.ts', "export const f = () => import('../sim-port/mock/rules');", 'app -> mock (dynamic)'],
    ['src/contract/x.ts', "export const f = () => import('./index');", 'contract imports something (dynamic)'],
    ['src/sim-port/x.ts', "export const f = () => import('../store');", 'sim-port -> store (dynamic)'],
    ['src/scene/x.ts', "export const f = (p: string) => import(p);", 'non-literal dynamic import'],
  ];
  it.each(violations)('%s fails lint: %s', async (file, code, _why) => {
    const [res] = await eslint.lintText(code + '\nexport {};\n', { filePath: file });
    expect(restricted(res!).length, _why).toBeGreaterThan(0);
  });

  it('allows legitimate imports', async () => {
    const ok: [string, string][] = [
      ['src/scene/x.ts', "import * as THREE from 'three'; import type { Game } from '../store'; import type { Snapshot } from '../contract';"],
      ['src/app/x.ts', "import { createSimPort } from '../sim-port'; import { createGame } from '../store'; import { createScene } from '../scene';"],
      ['src/store/x.ts', "import type { SimPort } from '../sim-port/port';"],
      ['src/sim-port/x.ts', "import { createMockSim } from './mock';"],
      ['src/store/x.ts', "export const f = () => import('../sim-port/port');"],
      ['src/store/x.ts', "export const f = () => import('./layout');"],
      ['src/scene/x.ts', "export const f = () => import('three');"],
      ['src/app/x.ts', "export const f = () => import('../panels');"],
    ];
    for (const [file, code] of ok) {
      const [res] = await eslint.lintText(code + '\nexport {};\n', { filePath: file });
      expect(restricted(res!), file).toEqual([]);
    }
  });
});
