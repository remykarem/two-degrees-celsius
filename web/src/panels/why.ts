import type { LeverInfo, Snapshot, TraceRecord } from '../contract';
import { formatLever, lobbyingDiffers, money, pct, tonnes } from './format';
import type { WhyKind } from './metrics';

export interface Bar { label: string; value: number; text: string; sub?: string; /** Second bar in the same row (lobbying: the effective value). */ pair?: number; tone?: 'plain' | 'sink' | 'muted' }
export interface BarGroup { heading: string; bars: Bar[]; /** Paired reference value (lobbying: what you set). */ pairLabel?: [string, string] }
export interface WhyData { kind: WhyKind; title: string; blurb: string; groups: BarGroup[]; empty: string | null }

const KIND_LABEL: Record<string, string> = { wages: 'Wages', dividends: 'Dividends', rent: 'Rent', interest: 'Interest', benefits: 'Benefits' };

/** 'Fossil plant at (4,7)': the plant behind a trace record (by id, else by tile). */
export function plantLabel(t: { id?: number; tile?: { x: number; y: number } }, snap: Snapshot | null): string {
  const plant = snap?.plants?.find((p) => (t.id !== undefined && p.id === t.id) || (t.tile && p.tile.x === t.tile.x && p.tile.y === t.tile.y));
  const tile = plant?.tile ?? t.tile;
  const kind = plant ? (plant.kind === 'fossil' ? 'Fossil plant' : 'Renewable plant') : 'Power plant';
  return tile ? `${kind} at (${tile.x},${tile.y})` : kind;
}

export function emissionsBars(trace: TraceRecord[], snap: Snapshot | null): Bar[] {
  const by = new Map<string, Bar>();
  for (const t of trace) {
    if (t.type !== 'emissions') continue;
    const label = t.source === 'firm' ? snap?.firms.find((f) => f.id === t.id)?.name ?? `Firm ${t.id ?? ''}`.trim()
      : t.source === 'plant' ? plantLabel(t, snap)
      : t.source === 'households' ? 'Households' : t.source === 'fire_sink_loss' ? 'Sink lost to fire' : 'Forest sink (absorbed)';
    const cur = by.get(label) ?? { label, value: 0, text: '', tone: t.source === 'sink' ? 'sink' as const : 'plain' as const };
    cur.value += t.source === 'sink' ? -Math.abs(t.tonnes) : t.source === 'fire_sink_loss' ? Math.abs(t.tonnes) : t.tonnes;
    by.set(label, cur);
  }
  const bars = [...by.values()];
  const gross = bars.filter((b) => b.value > 0).reduce((a, b) => a + b.value, 0);
  bars.forEach((b) => { b.text = tonnes(b.value); if (b.value > 0 && gross > 0) b.sub = `${pct(b.value / gross)} of emitted${b.label === 'Sink lost to fire' ? ' (burnt forest absorbs less)' : ''}`; });
  return bars.sort((a, b) => b.value - a.value);
}

export function incomeByType(trace: TraceRecord[]): Bar[] {
  const by = new Map<string, number>();
  for (const t of trace) if (t.type === 'income') by.set(t.kind, (by.get(t.kind) ?? 0) + t.amount);
  const total = [...by.values()].reduce((a, b) => a + b, 0);
  return [...by.entries()].map(([k, v]) => ({ label: KIND_LABEL[k] ?? k, value: v, text: money(v), sub: total > 0 ? pct(v / total) : undefined }))
    .sort((a, b) => b.value - a.value);
}

export function incomeByHousehold(trace: TraceRecord[], snap: Snapshot | null): Bar[] {
  const by = new Map<number, number>();
  for (const t of trace) if (t.type === 'income') by.set(t.household, (by.get(t.household) ?? 0) + t.amount);
  return [...by.entries()].map(([id, v]) => {
    const h = snap?.households.find((x) => x.id === id);
    return { label: h ? `${h.label} (${h.class})` : `Household ${id}`, value: v, text: money(v) };
  }).sort((a, b) => b.value - a.value);
}

export function lobbyingBars(trace: TraceRecord[], levers: LeverInfo[]): { bars: Bar[]; netLobby: number | null } {
  const gaps = trace.filter((t): t is Extract<TraceRecord, { type: 'policy_gap' }> => t.type === 'policy_gap');
  const bars = gaps.filter((g) => lobbyingDiffers(g.nominal, g.effective)).map((g) => {
    const l = levers.find((x) => x.key === g.lever);
    const fmt = (v: number) => (l ? formatLever(l, v) : v.toFixed(2));
    const lost = g.nominal !== 0 ? (g.nominal - g.effective) / g.nominal : 0;
    return { label: l?.label ?? g.lever, value: g.nominal, text: `set ${fmt(g.nominal)}, in force ${fmt(g.effective)}`, sub: `${pct(Math.max(0, lost))} lost to lobbying`, pair: g.effective } satisfies Bar;
  });
  return { bars, netLobby: gaps.length ? gaps[0]!.netLobby : null };
}

export function whyData(kind: WhyKind, trace: TraceRecord[], snap: Snapshot | null, levers: LeverInfo[]): WhyData {
  if (kind === 'emissions') {
    const bars = emissionsBars(trace, snap);
    return { kind, title: 'Where the emissions came from', blurb: 'Tonnes from the latest turn\'s trace, by source. The forest sink absorbs some of it.',
      groups: [{ heading: 'Emissions by source', bars }], empty: bars.length ? null : 'No emissions trace yet. End a turn to see one.' };
  }
  if (kind === 'income') {
    const types = incomeByType(trace);
    return { kind, title: 'Where income came from', blurb: 'Income recorded in the latest turn\'s trace. Who receives which kind of income drives inequality and demand.',
      groups: [{ heading: 'Income by type', bars: types }, { heading: 'Income by household', bars: incomeByHousehold(trace, snap) }],
      empty: types.length ? null : 'No income trace yet. End a turn to see one.' };
  }
  const { bars, netLobby } = lobbyingBars(trace, levers);
  return { kind, title: 'How lobbying changed your policies', blurb: netLobby !== null ? `Net lobbying pressure is ${pct(netLobby)}. Weakened policies are the gap between what you set and what is in force.` : 'Lobbying only weakens some policies: carbon price, wealth tax, profit tax and emissions cap.',
    groups: [{ heading: 'You set versus in force', bars, pairLabel: ['You set', 'In force'] }], empty: bars.length ? null : 'No lobbied policy was weakened this turn.' };
}
