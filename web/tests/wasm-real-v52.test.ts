import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { Snapshot } from '../src/contract';
import { emissionsView } from '../src/panels/emissions';
import { sumCheck } from '../src/panels/explain';
import { firmLines, noStateOwnership } from '../src/panels/industry';
import { METRICS, metricText } from '../src/panels/metrics';
import { isBarePercent } from '../src/panels/backing';
import { policyLines, timelineLines } from '../src/panels/policies';
import { taxLines } from '../src/panels/taxes';
import { anchorsWithoutTopic } from '../src/panels/explorer';
import { createWasmPort, type WasmModuleLike } from '../src/sim-port/wasm';
import { ok } from './helpers';

// The v1.5/v1.6 transparency views on the REAL simulation (wasm). Skipped when the package is not built or predates v1.6.
const pkg = resolve(__dirname, '..', 'src', 'sim-port', 'wasm', 'pkg');
const present = existsSync(resolve(pkg, 'sim_wasm_bg.wasm'));

describe.skipIf(!present)('the real sim: breakdowns, policies, industry, explorer', async () => {
  const mod = (await import(/* @vite-ignore */ resolve(pkg, 'sim_wasm.js'))) as WasmModuleLike & { initSync(o: { module: Uint8Array }): unknown };
  mod.initSync({ module: readFileSync(resolve(pkg, 'sim_wasm_bg.wasm')) });
  const port = createWasmPort(mod);
  const first = ok(port.init(1));
  const hasV16 = !!first.breakdowns;
  let snap: Snapshot = first;
  const cur = Number(first.policy.tax_profit.nominal);
  if (hasV16) {
    snap = ok(port.submitTurn([{ type: 'set_policy', lever: 'tax_profit', value: Math.min(1, cur + 0.05) }])).snapshot;
    for (let t = 0; t < 5; t++) snap = ok(port.submitTurn([])).snapshot;
  }
  it.skipIf(!hasV16)('breakdown parts sum to results and the emissions tally reconciles', () => {
    const b = snap.breakdowns!;
    for (const k of ['gdp', 'gpi', 'wageShare', 'profitShare', 'debtToGdp', 'gini', 'legitimacy', 'capitalRatio'] as const) if (b[k].parts) expect(sumCheck(b[k])?.ok, k).toBe(true);
    expect(emissionsView(b.emissions).reconciliation.ok).toBe(true);
    expect(b.emissions.reconciles).toBe(true);
    expect(taxLines(snap).find((t) => t.id === 'gst')?.modelled).toBe(false);
    expect(b.gini.lorenz.length).toBeGreaterThan(2);
  });
  it.skipIf(!hasV16)('policies carry the turn-0 values, the change and its timeline; industry says no state share', () => {
    const lines = policyLines(snap.policies!, port.levers());
    expect(lines.find((l) => l.lever === 'tax_profit')!.changedFromInitial).toBe(true);
    expect(lines.find((l) => l.lever === 'tax_wage')!.label).toBe('Income tax');
    expect(timelineLines(snap.policies!, port.levers()).some((e) => e.kind === 'activated')).toBe(true);
    expect(noStateOwnership(snap.industry!)).toBe(true);
    expect(firmLines(snap.industry!).length).toBeGreaterThan(0);
  });
  it.skipIf(!hasV16)('no metric string on the real wire is a bare percentage', () => {
    for (const m of METRICS) expect(isBarePercent(metricText(m, snap)), m.id).toBe(false);
  });
  it.skipIf(!hasV16)('the explorer catalogue covers every metric and lever (skipped until the package exports assumptions)', () => {
    const r = port.assumptions();
    if (!r.ok) return; // package predates v1.5
    expect(anchorsWithoutTopic(r.value, [...new Set(METRICS.map((m) => m.metric)), ...port.levers().map((l) => l.key)])).toEqual([]);
  });
});
