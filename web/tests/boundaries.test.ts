import { ESLint } from 'eslint';
import { describe, expect, it } from 'vitest';

const eslint = new ESLint({ cwd: process.cwd() });
const restricted = (r: { messages: { ruleId: string | null; message: string }[] }) => r.messages.filter((m) => m.ruleId === 'no-restricted-imports');

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
    ];
    for (const [file, code] of ok) {
      const [res] = await eslint.lintText(code + '\nexport {};\n', { filePath: file });
      expect(restricted(res!), file).toEqual([]);
    }
  });
});
