import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

const msg = (m) => ({ message: m });
const forbid = (...patterns) => ({
  'no-restricted-imports': ['error', { patterns: patterns.map(([group, message]) => ({ group, ...msg(message) })) }],
});

const SVELTE = ['svelte', 'svelte/*', '**/*.svelte'];
const THREE = ['three', 'three/*'];
const MOCK = '**/sim-port/mock';
const MOCK_DEEP = '**/sim-port/mock/*';

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'test-results'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    rules: { '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }] },
  },
  { files: ['**/*.svelte'], languageOptions: { parserOptions: { parser: tseslint.parser } } },

  // Boundary rules (docs/ui-design.md "Rules"). Enforced by tests/boundaries.test.ts.
  // Nothing outside sim-port may import the mock. Applied first; per-folder blocks below add to it.
  { files: ['src/**/*.{ts,svelte}'], ignores: ['src/sim-port/**'],
    rules: forbid([[MOCK, MOCK_DEEP], 'Only sim-port may import sim-port/mock']) },
  { files: ['src/contract/**'],
    rules: forbid([['*', '../*', './*', '**'], 'contract must not import anything: types only, no dependencies']) },
  { files: ['src/scene/**'],
    rules: forbid([[...SVELTE, '**/panels', '**/panels/*', '**/ui-kit', '**/ui-kit/*', '**/app', '**/app/*', '**/sim-port', '**/sim-port/*'],
      'scene is Three.js only: no Svelte, panels, ui-kit, app or sim-port. Talk through the store']) },
  { files: ['src/panels/**'],
    rules: forbid([[...THREE, '**/scene', '**/scene/*', '**/app', '**/app/*', '**/sim-port', '**/sim-port/*'],
      'panels are Svelte only: no Three.js, scene, app or sim-port. Talk through the store']) },
  { files: ['src/ui-kit/**'],
    rules: forbid([[...THREE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/store', '**/store/*', '**/sim-port', '**/sim-port/*', '**/app', '**/app/*'],
      'ui-kit depends on nothing but Svelte']) },
  { files: ['src/store/**'],
    rules: {
      'no-restricted-imports': ['error', {
        paths: [{ name: '../sim-port', message: 'store may only use the SimPort type: import from ../sim-port/port' },
                { name: '../sim-port/index', message: 'store may only use the SimPort type: import from ../sim-port/port' }],
        patterns: [{ group: [MOCK, MOCK_DEEP, ...THREE, ...SVELTE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/ui-kit', '**/ui-kit/*', '**/app', '**/app/*'],
          message: 'store is framework-agnostic: no DOM frameworks, Three.js, scene, panels or app' }],
      }],
    } },
  { files: ['src/sim-port/**'],
    rules: forbid([[...THREE, ...SVELTE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/store', '**/store/*', '**/app', '**/app/*', '**/ui-kit', '**/ui-kit/*'],
      'sim-port depends only on contract']) },
  { files: ['src/sim-port/mock/**'],
    rules: forbid([[...THREE, ...SVELTE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/store', '**/store/*', '**/app', '**/app/*', '**/ui-kit', '**/ui-kit/*', '../index', '../../sim-port/index'],
      'mock depends only on contract and sim-port/port (no factory, no UI)']) },
);
