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
const REPLAY = '**/sim-port/replay';
const REPLAY_DEEP = '**/sim-port/replay/*';
const WASM = '**/sim-port/wasm';
const WASM_DEEP = '**/sim-port/wasm/*';

// import() is not covered by no-restricted-imports, so the same boundaries are enforced for dynamic imports by this small local rule.
const dynamicImport = {
  meta: { type: 'problem', schema: [{ type: 'object' }] },
  create(ctx) {
    const { pattern, message } = ctx.options[0];
    const re = new RegExp(pattern);
    return {
      ImportExpression(node) {
        if (node.source.type !== 'Literal') ctx.report({ node, message: 'Dynamic import() needs a literal path so module boundaries can be checked' });
        else if (re.test(String(node.source.value))) ctx.report({ node, message });
      },
    };
  },
};
const boundary = { rules: { 'dynamic-import': dynamicImport } };
const dyn = (pattern, message) => ({ 'boundary/dynamic-import': ['error', { pattern, message }] });
const P = (names) => `(^|/)(${names.join('|')})(/|$)`;
const NOT_SVELTE_THREE = ['three', 'svelte'];

export default tseslint.config(
  { ignores: ['dist', 'node_modules', 'test-results', 'src/sim-port/wasm/pkg'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    rules: { '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }] },
  },
  { files: ['**/*.svelte'], languageOptions: { parserOptions: { parser: tseslint.parser } } },

  { plugins: { boundary } },
  // Dynamic import() may not cross the boundaries either.
  { files: ['src/**/*.{ts,svelte}'], ignores: ['src/sim-port/**'], rules: dyn(P(['sim-port/replay', 'sim-port/wasm']), 'Only sim-port may import sim-port/replay or sim-port/wasm') },
  { files: ['src/contract/**'], rules: dyn('^(?!\\./generated$).*', 'contract must not import anything (index.ts may re-export ./generated)') },
  { files: ['src/scene/**'], rules: dyn(`${P(['panels', 'ui-kit', 'app', 'sim-port', 'svelte'])}|\\.svelte$`, 'scene is Three.js only: no Svelte, panels, ui-kit, app or sim-port') },
  { files: ['src/panels/**'], rules: dyn(P(['three', 'scene', 'app', 'sim-port']), 'panels are Svelte only: no Three.js, scene, app or sim-port') },
  { files: ['src/ui-kit/**'], rules: dyn(P(['three', 'scene', 'panels', 'store', 'sim-port', 'app']), 'ui-kit depends on nothing but Svelte') },
  { files: ['src/store/**'], rules: dyn(`${P([...NOT_SVELTE_THREE, 'scene', 'panels', 'ui-kit', 'app'])}|(^|/)sim-port(/(?!port$).*)?$`, 'store is framework-agnostic and may only reach sim-port/port') },
  { files: ['src/sim-port/**'], rules: dyn(P([...NOT_SVELTE_THREE, 'scene', 'panels', 'store', 'app', 'ui-kit']), 'sim-port depends only on contract') },

  // Boundary rules (docs/ui-design.md "Rules"). Enforced by tests/boundaries.test.ts.
  // Nothing outside sim-port may import the replay port or the wasm wrapper. Applied first; per-folder blocks below add to it.
  { files: ['src/**/*.{ts,svelte}'], ignores: ['src/sim-port/**'],
    rules: forbid([[REPLAY, REPLAY_DEEP, WASM, WASM_DEEP], 'Only sim-port may import sim-port/replay or sim-port/wasm (use selectSimPort from sim-port)']) },
  { files: ['src/contract/**'], ignores: ['src/contract/index.ts'],
    rules: forbid([['*', '../*', './*', '**'], 'contract must not import anything: types only, no dependencies']) },
  // index.ts re-exports the generated file and may import nothing else.
  { files: ['src/contract/index.ts'],
    rules: { 'no-restricted-imports': ['error', { patterns: [{ regex: '^(?!\\./generated$)', message: 'contract/index.ts may import only ./generated' }] }] } },
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
        patterns: [
          { regex: '(^|/)sim-port(/(?!port$).*)?$', message: 'store may only use the SimPort type: import from ../sim-port/port (no factory, no mock, no other sim-port subpath)' },
          { group: [...THREE, ...SVELTE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/ui-kit', '**/ui-kit/*', '**/app', '**/app/*'],
            message: 'store is framework-agnostic: no DOM frameworks, Three.js, scene, panels or app' }],
      }],
    } },
  { files: ['src/sim-port/**'],
    rules: forbid([[...THREE, ...SVELTE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/store', '**/store/*', '**/app', '**/app/*', '**/ui-kit', '**/ui-kit/*'],
      'sim-port depends only on contract']) },
  { files: ['src/sim-port/replay/**'],
    rules: forbid([[...THREE, ...SVELTE, '**/scene', '**/scene/*', '**/panels', '**/panels/*', '**/store', '**/store/*', '**/app', '**/app/*', '**/ui-kit', '**/ui-kit/*', '../index', '../../sim-port/index'],
      'replay depends only on contract and sim-port/port (no factory, no UI)']) },
);
