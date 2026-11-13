// Builds the Rust simulation for the browser: sim-wasm -> web/src/sim-port/wasm/pkg/
//   node scripts/build-wasm.mjs            (web target, used by the app and the Vitest golden test)
// Needs: rustup target add wasm32-unknown-unknown, and wasm-bindgen-cli at the version pinned in Cargo.lock.
// wasm-opt (binaryen) is used when installed; its absence only makes the file bigger.
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, statSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { gzipSync } from 'node:zlib';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const out = resolve(root, 'web/src/sim-port/wasm/pkg');
const raw = resolve(root, 'target/wasm32-unknown-unknown/wasm/sim_wasm.wasm');

const lock = readFileSync(resolve(root, 'Cargo.lock'), 'utf8');
const version = /name = "wasm-bindgen"\nversion = "([^"]+)"/.exec(lock)?.[1];
if (!version) fail('Could not find wasm-bindgen in Cargo.lock.');

function run(cmd, args, opts = {}) {
  const r = spawnSync(cmd, args, { cwd: root, stdio: 'inherit', ...opts });
  if (r.error?.code === 'ENOENT') return null;
  return r.status === 0;
}
function fail(msg) {
  console.error(`\nbuild-wasm: ${msg}\n`);
  process.exit(1);
}

const have = (cmd, args) => spawnSync(cmd, args, { encoding: 'utf8' });
const wb = have('wasm-bindgen', ['--version']);
const wbVersion = wb.status === 0 ? wb.stdout.trim().split(' ')[1] : null;
if (wbVersion !== version) {
  fail(
    `wasm-bindgen-cli ${version} is required (found: ${wbVersion ?? 'none'}).\n` +
      `  Install it with:\n    cargo install wasm-bindgen-cli --version ${version} --locked\n` +
      `  and the target with:\n    rustup target add wasm32-unknown-unknown\n` +
      `  The app still runs on the recorded demo without the wasm package.`,
  );
}

console.log(`> cargo build --profile wasm --target wasm32-unknown-unknown -p sim-wasm`);
const built = run('cargo', ['build', '--profile', 'wasm', '--target', 'wasm32-unknown-unknown', '-p', 'sim-wasm']);
if (!built) fail('cargo build failed (is the wasm32-unknown-unknown target installed? rustup target add wasm32-unknown-unknown).');

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
console.log(`> wasm-bindgen --target web`);
if (!run('wasm-bindgen', ['--target', 'web', '--out-dir', out, '--out-name', 'sim_wasm', raw])) fail('wasm-bindgen failed.');

const bin = resolve(out, 'sim_wasm_bg.wasm');
async function smokeTest(file) {
  try {
    const mod = await import(`${pathToFileURL(resolve(out, 'sim_wasm.js')).href}?smoke=${Date.now()}`);
    mod.initSync({ module: readFileSync(file) });
    return true;
  } catch (e) {
    console.warn(`smoke test failed: ${e?.message ?? e}`);
    return false;
  }
}
const size = (p) => statSync(p).size;
const before = size(bin);
const opt = have('wasm-opt', ['--version']);
if (opt.status === 0) {
  console.log('> wasm-opt -Oz');
  const plain = readFileSync(bin);
  const optimised = run('wasm-opt', ['-Oz', '--enable-bulk-memory', '--enable-nontrapping-float-to-int', '--enable-sign-ext', '--enable-mutable-globals', '--enable-reference-types', '--enable-multivalue', bin, '-o', bin]);
  // An old wasm-opt can emit a module that instantiates but breaks at run time (it rewrote the externref table as funcref).
  // Smoke-test the optimised file in Node and fall back to the plain one if it does not start.
  const healthy = optimised && (await smokeTest(bin));
  if (!healthy) {
    writeFileSync(bin, plain);
    console.warn('wasm-opt failed or produced a module that does not start (is binaryen old? use version 133 or newer); keeping the unoptimised file.');
  }
} else {
  console.warn('WARNING: wasm-opt (binaryen) is not installed, so the wasm is larger than it needs to be (the build still works). Install it: brew install binaryen, or apt-get install binaryen.');
}
const kb = (n) => `${(n / 1024).toFixed(0)} KiB`;
console.log(`\nsize: ${kb(before)} raw${size(bin) !== before ? `, ${kb(size(bin))} after wasm-opt` : ''}, ${kb(gzipSync(readFileSync(bin)).length)} gzipped`);
console.log(`output: ${out}`);
if (!existsSync(resolve(out, 'sim_wasm.js'))) fail('sim_wasm.js missing after build.');
