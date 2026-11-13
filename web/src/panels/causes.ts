import type { CausalEdge, CausalGraph, CausalNode, CauseRef, Catalog, EdgeEffect, LeverInfo, Snapshot } from '../contract';
import { fill, text } from '../store/catalog';
import { fmtUnit } from './explain';
import { DIRECTION_ARROW } from './markets';
import { loopLines, SIGN_GLYPH, type LoopLine } from './explorer';
import { leadText } from './format';

/**
 * The "Why?" view (causality graph stage 4): the causes of one quantity as a layered graph. Everything a player reads as a model fact comes from the wire:
 * the causes with their depth and net sign (`CausalNode.causes`, Rust `causal::causes_of` and `sign_of`), the signed edges with their sentence and lag,
 * the loops, the live values and the words. This file only selects, joins and lays out (presentational: ranks, order, coordinates), so a quantity or edge added only to
 * the catalogue renders with no change here.
 */

/** Presentation constants (pixels). Layout is the web's job; nothing here is a model number. */
export const LAYOUT = { nodeW: 176, nodeH: 78, focusH: 92, gapX: 64, gapY: 10, pad: 10, lane: 7, chipW: 50, chipH: 18 } as const;

/** Line widths in pixels (presentation). The wire's 0 to 1 weight maps linearly between `min` and `max`; the scale that makes it a weight (the log range, the words) is content. `hover` is added while a link is active, so thickness never collides with the hover state. */
export const STROKE = { base: 1.8, min: 1.2, max: 4.4, hover: 1.4 } as const;
/** The stroke width of a link: its weight on the wire, or the plain width when it has none (depends, yes or no, not measured). */
export const strokeWidth = (weight: number | null, active = false): number =>
  (weight === null ? STROKE.base : STROKE.min + Math.min(1, Math.max(0, weight)) * (STROKE.max - STROKE.min)) + (active ? STROKE.hover : 0);

/** The dash pattern of a minus link scaled to its width, so a thick dashed line still reads as dashed (the style channel stays the sign's). */
export const strokeDash = (sign: string, width: number): string | undefined => (sign === 'down' ? `${Math.max(7, width * 3)} ${Math.max(4, width * 1.8)}` : undefined);

const tt = (k: string, vars: Record<string, string | number> = {}): string => fill(text('causes', k), vars);

export interface LoopMark { kind: string; badge: string; label: string; count: number }
export interface LeverRef { key: string; label: string; leadText: string }
export interface CauseNodeModel {
  id: string;
  refId: string;
  label: string;
  depth: number;
  focus: boolean;
  /** The wire's net sign id on the focus (`up`, `down`, `varies`, `mixed`, `none`), null for the focus itself. */
  netSign: string | null;
  netLabel: string | null;
  /** The formatted live value, or null when there is none. */
  value: string | null;
  trendArrow: string;
  trendWord: string;
  /** Why there is no live value (the wire's reason label), else null. */
  unpublished: string | null;
  /** A starting point (nothing in the registry writes it): the wire's role label, else null. */
  startLabel: string | null;
  selfLoop: boolean;
  loops: LoopMark[];
  lever: LeverRef | null;
  /** Set when the Why? panel has a graph for this quantity (it has causes). */
  hasCauses: boolean;
  /** The overall size of this cause on the focus (the wire's `netStrength`): the short line, the full sentence and the 0 to 1 weight; null for the focus and for causes with no sized route. */
  strengthLine: string | null;
  strengthText: string | null;
  strengthLabel: string | null;
}
export interface CauseEdgeModel {
  key: string;
  from: string;
  to: string;
  via: string;
  viaLabel: string;
  sign: string;
  glyph: string;
  lagMonths: number;
  lagLabel: string;
  sentence: string;
  checked: string;
  /** The wire's thickness weight (0 to 1), or null when the link has none: it depends on the state, is a yes or no, was not measured, or has no effect. */
  weight: number | null;
  /** The list line ("Strength 0.42 (Moderate)", "Strength: depends", ...), empty when there is nothing to say; and the full sentence with its caveats. */
  strengthLine: string;
  strengthText: string;
  selfLoop: boolean;
  /** 0-based index among edges joining the same pair, and how many there are (two writers of one quantity are separate edges). */
  lane: number;
  lanes: number;
}
export interface CauseModel {
  focus: CauseNodeModel;
  /** Nodes in reading order: furthest causes first, the focus last. */
  nodes: CauseNodeModel[];
  /** Edges drawn (a step towards the focus), then the self-loops last. */
  edges: CauseEdgeModel[];
  /** Links between causes in the same step or looping back (not drawn; the list shows them). */
  undrawn: CauseEdgeModel[];
  /** Links with no direct effect that the toggle is hiding. */
  noneHidden: number;
  maxDepth: number;
  levers: { node: CauseNodeModel; lever: LeverRef }[];
  loops: LoopLine[];
}

const nodeId = (q: string): string => (q.startsWith('quantity:') ? q : `quantity:${q}`);

/** The quantity node id a UI anchor (a metric, a lever or a market id) opens Why? for, from the wire's entry points; null when it has none. */
export function entryQuantity(g: CausalGraph, anchor: string): string | null {
  return g.entryPoints.find((e) => e.anchor === anchor)?.quantity ?? null;
}
/** True when the catalogue knows this quantity. One with no causes still opens the view: it says the model does not explain it (a starting point), which is itself the answer. */
export function inGraph(g: CausalGraph, quantity: string): boolean {
  return g.nodes.some((x) => x.id === nodeId(quantity) && x.kind === 'quantity');
}

interface Ctx { g: CausalGraph; c: Catalog; snap: Snapshot | null; levers: LeverInfo[] }

function nodeModel(x: Ctx, n: CausalNode, depth: number, netSign: string | null, selfLoop: boolean, ref: CauseRef | null): CauseNodeModel {
  const { g, c, snap, levers } = x;
  const info = c.quantities.find((q) => q.id === n.refId);
  const live = snap?.quantities.find((l) => l.id === n.refId);
  const reason = info?.live.unbound ? g.live.unbound.find((u) => u.id === info.live.unbound)?.label ?? info.live.unbound : null;
  const flag = info?.live.aggregate === 'flag';
  const value = live && info ? (flag ? (live.value >= 0.5 ? 'yes' : 'no') : fmtUnit(live.value, info.unit)) : null;
  const marks = new Map<string, LoopMark>();
  for (const l of g.loops.filter((lp) => n.inLoops.includes(lp.id))) {
    const k = g.loopKinds.find((kk) => kk.id === l.kind);
    const m = marks.get(l.kind);
    if (m) m.count++;
    else marks.set(l.kind, { kind: l.kind, badge: k?.badge ?? l.kind, label: k?.label ?? l.kind, count: 1 });
  }
  const lever = info?.live.lever ? levers.find((l) => l.key === info.live.lever) : undefined;
  const role = g.roles.find((r) => r.id === n.role);
  return {
    id: n.id, refId: n.refId, label: n.label, depth, focus: depth === 0, netSign,
    netLabel: netSign ? g.netSigns.find((s) => s.id === netSign)?.label ?? netSign : null,
    value: value ?? null,
    trendArrow: live?.trend ? DIRECTION_ARROW[live.trend] ?? '' : '',
    trendWord: live?.trend ? c.markets.directions[live.trend] ?? '' : '',
    unpublished: value === null ? (reason ?? '') : null,
    startLabel: n.role === 'source' ? role?.label ?? null : null,
    selfLoop, loops: [...marks.values()], hasCauses: n.causes.length > 0,
    lever: lever ? { key: lever.key, label: lever.label, leadText: leadText(lever.leadTime) } : null,
    ...netStrength(g, ref),
  };
}

/** The overall size of a cause from the wire's `netStrength` and the templates in the catalogue (filled here, so the sentence is not repeated per cause on the wire). */
function netStrength(g: CausalGraph, ref: CauseRef | null): Pick<CauseNodeModel, 'strengthLine' | 'strengthText' | 'strengthLabel'> {
  const ns = ref?.netStrength;
  if (!ns) return { strengthLine: null, strengthText: null, strengthLabel: null };
  const word = g.strength.words.find((w) => w.id === ns.word)?.label ?? ns.word;
  const sentence = fill(ns.cancels ? g.strength.netCancel : g.strength.net, { value: ns.label, word, gross: ns.grossLabel }) + (ns.partial ? g.strength.netPartial : '');
  return { strengthLine: tt('net_strength_line', { value: ns.label, word }), strengthText: sentence, strengthLabel: ns.label };
}

/** What a link says about its size, all from the wire: a weight and numbers for a measured link; "depends", "not measured" or the kind (yes or no, not sized) otherwise. */
function edgeStrength(g: CausalGraph, fx: EdgeEffect): Pick<CauseEdgeModel, 'weight' | 'strengthLine' | 'strengthText'> {
  const none = { weight: null, strengthLine: '', strengthText: '' };
  if (fx.sign === 'varies') return { weight: null, strengthLine: tt('strength_depends'), strengthText: g.strength.depends };
  if (fx.sign === 'none') return none;
  const s = fx.strength;
  if (!s) return fx.basis === 'declared_unverified' ? { weight: null, strengthLine: tt('strength_unverified'), strengthText: g.strength.unverified } : none;
  const kind = g.strength.kinds.find((k) => k.id === s.kind)?.label ?? s.kind;
  const word = s.word ? g.strength.words.find((w) => w.id === s.word)?.label ?? s.word : kind;
  return {
    weight: s.weight ?? null,
    strengthLine: s.label ? tt('strength_line', { value: s.label, word }) : tt('strength_kind_line', { kind }),
    strengthText: s.text,
  };
}

const edgeModel = (g: CausalGraph, e: CausalEdge): CauseEdgeModel => {
  const fx = e.effect!;
  return {
    key: `${e.via}|${e.from}|${e.to}`, from: e.from, to: e.to, via: e.via ?? '', viaLabel: g.nodes.find((n) => n.id === e.via)?.label ?? '',
    sign: fx.sign, glyph: SIGN_GLYPH[fx.sign] ?? fx.sign, lagMonths: fx.lagMonths, lagLabel: fx.lagLabel, sentence: fx.sentence, checked: fx.checked, ...edgeStrength(g, fx),
    selfLoop: e.from === e.to, lane: 0, lanes: 1,
  };
};

/**
 * The model of the Why? view for `focusId` (a `quantity:<id>` node id or a bare quantity id) within `depth` steps. `showNone` adds the links that are read but have no direct effect
 * (hidden by default: they would draw lines that do nothing). Returns null when the quantity is not in the catalogue.
 */
export function causeModel(g: CausalGraph, c: Catalog, snap: Snapshot | null, levers: LeverInfo[], focusId: string, depth: number, showNone = false): CauseModel | null {
  const x: Ctx = { g, c, snap, levers };
  const focusNode = g.nodes.find((n) => n.id === nodeId(focusId) && n.kind === 'quantity');
  if (!focusNode) return null;
  const byId = new Map(g.nodes.map((n) => [n.id, n]));
  const within = focusNode.causes.filter((k) => k.depth <= depth);
  const depthOf = new Map<string, number>([[focusNode.id, 0], ...within.map((k): [string, number] => [k.id, k.depth])]);
  const netOf = new Map(within.map((k) => [k.id, k.netSign]));
  const all = g.edges.filter((e) => e.kind === 'drives' && e.effect && depthOf.has(e.from) && depthOf.has(e.to));
  const selfEdges = all.filter((e) => e.from === e.to);
  const forward = all.filter((e) => e.from !== e.to && depthOf.get(e.from)! > depthOf.get(e.to)!);
  const visible = forward.filter((e) => showNone || e.effect!.sign !== 'none');
  // A cause stays when some drawn link leads from it towards the focus (a cause reached only through links with no effect would float alone).
  const keep = new Set<string>([focusNode.id]);
  for (let d = 1; d <= depth; d++) for (const e of visible) if (depthOf.get(e.from) === d && keep.has(e.to)) keep.add(e.from);
  // Links from a kept cause to a kept cause only; a link whose target was dropped is not drawn either.
  const drawn = visible.filter((e) => keep.has(e.from) && keep.has(e.to));
  const lanes = new Map<string, CauseEdgeModel[]>();
  const models = drawn.map((e) => edgeModel(g, e));
  for (const m of models) { const k = `${m.from}|${m.to}`; lanes.set(k, [...(lanes.get(k) ?? []), m]); }
  for (const list of lanes.values()) list.forEach((m, i) => { m.lane = i; m.lanes = list.length; });
  const keptSet = (id: string) => keep.has(id);
  const drawnKeys = new Set(drawn.map((e) => `${e.via}|${e.from}|${e.to}`));
  const undrawn = all
    .filter((e) => e.from !== e.to && keptSet(e.from) && keptSet(e.to) && !drawnKeys.has(`${e.via}|${e.from}|${e.to}`) && (showNone || e.effect!.sign !== 'none'))
    .map((e) => edgeModel(g, e));
  const noneHidden = showNone ? 0 : forward.filter((e) => e.effect!.sign === 'none').length;
  const selfModels = selfEdges.filter((e) => keptSet(e.from) && (showNone || e.effect!.sign !== 'none')).map((e) => edgeModel(g, e));
  const selfSet = new Set(selfModels.map((m) => m.from));
  const nodes = [...keep]
    .map((id) => nodeModel(x, byId.get(id)!, depthOf.get(id)!, id === focusNode.id ? null : netOf.get(id) ?? 'none', selfSet.has(id) || byId.get(id)!.selfLoop, within.find((k) => k.id === id) ?? null))
    .sort((a, b) => b.depth - a.depth || order(focusNode, a.id) - order(focusNode, b.id));
  const focus = nodes.find((n) => n.focus)!;
  const reach = nodes.filter((n) => !n.focus);
  return {
    focus, nodes, edges: [...models, ...selfModels], undrawn, noneHidden, maxDepth: Math.max(0, ...nodes.map((n) => n.depth)),
    levers: reach.flatMap((n) => (n.lever ? [{ node: n, lever: n.lever }] : [])),
    loops: loopLines(g, [focusNode.id]),
  };
}
const order = (focus: CausalNode, id: string): number => (id === focus.id ? -1 : focus.causes.findIndex((k) => k.id === id));

// ---- Layout (presentational): layers by depth, focus at the right, deterministic, never overlapping ----

export interface Placed { id: string; x: number; y: number; w: number; h: number; depth: number }
export interface Layout { width: number; height: number; nodes: Placed[]; byId: Map<string, Placed> }

/** Edge routing: a curve from the right of the source to the left of the target, the ends fanned by lane so parallel links (two writers of one quantity) do not coincide. */
export function route(l: Layout, e: Pick<CauseEdgeModel, 'from' | 'to' | 'lane' | 'lanes'>): { d: string; at: (t: number) => { x: number; y: number } } | null {
  const a = l.byId.get(e.from);
  const b = l.byId.get(e.to);
  if (!a || !b) return null;
  const off = (e.lane - (e.lanes - 1) / 2) * LAYOUT.lane;
  const x1 = a.x + a.w, y1 = a.y + a.h / 2 + off, x2 = b.x, y2 = b.y + b.h / 2 + off;
  const dx = Math.max(24, (x2 - x1) / 2);
  const bez = (t: number, p0: number, p1: number, p2: number, p3: number) => (1 - t) ** 3 * p0 + 3 * (1 - t) ** 2 * t * p1 + 3 * (1 - t) * t * t * p2 + t ** 3 * p3;
  return { d: `M${x1} ${y1} C${x1 + dx} ${y1} ${x2 - dx} ${y2} ${x2} ${y2}`, at: (t) => ({ x: bez(t, x1, x1 + dx, x2 - dx, x2), y: bez(t, y1, y1, y2, y2) }) };
}

/**
 * Where each link's polarity chip sits: on the curve, at the first of a fixed list of positions where it overlaps no chip placed before it (edges taken in order, so the result is
 * deterministic); when none is free the middle is used. Chips never cover a node: the curve between two columns lies in the gap.
 */
export function placeChips(l: Layout, edges: Pick<CauseEdgeModel, 'key' | 'from' | 'to' | 'lane' | 'lanes'>[]): Map<string, { x: number; y: number }> {
  const { chipW, chipH } = LAYOUT;
  const out = new Map<string, { x: number; y: number }>();
  const placed: { x: number; y: number }[] = [];
  const hit = (p: { x: number; y: number }) => placed.some((q) => Math.abs(q.x - p.x) < chipW && Math.abs(q.y - p.y) < chipH);
  for (const e of edges) {
    const r = route(l, e);
    if (!r) continue;
    const a = l.byId.get(e.from)!, b = l.byId.get(e.to)!;
    // A chip must sit in the gap between the two nodes' columns, not over a node in a column the link passes.
    const free = (p: { x: number; y: number }) => p.x - chipW / 2 >= a.x + a.w && p.x + chipW / 2 <= b.x && !l.nodes.some((n) => n.id !== a.id && n.id !== b.id && overlaps({ id: '', x: p.x - chipW / 2, y: p.y - chipH / 2, w: chipW, h: chipH, depth: 0 }, n));
    const ts = Array.from({ length: 37 }, (_, i) => 0.05 + i * 0.025).sort((x, y) => Math.abs(x - 0.5) - Math.abs(y - 0.5) || x - y);
    const pick = ts.map((t) => r.at(t)).find((p) => free(p) && !hit(p)) ?? ts.map((t) => r.at(t)).find((p) => !hit(p)) ?? r.at(0.5);
    placed.push(pick);
    out.set(e.key, pick);
  }
  return out;
}

/**
 * Rank placement. Columns by depth (furthest cause leftmost, the focus alone in the last column, vertically centred). Within a column causes are ordered by the mean vertical position
 * of the nodes they feed (one sweep from the focus outwards), ties broken by the model's order, so equal input always gives equal output. Nodes never overlap: each column stacks fixed-height boxes with a gap.
 */
export function layoutCauses(m: Pick<CauseModel, 'nodes' | 'edges' | 'maxDepth'>): Layout {
  const { nodeW, nodeH, focusH, gapX, gapY, pad } = LAYOUT;
  const cols: string[][] = [];
  for (let d = 0; d <= m.maxDepth; d++) cols.push(m.nodes.filter((n) => n.depth === d).map((n) => n.id));
  const index = new Map(m.nodes.map((n, i) => [n.id, i]));
  const height = (id: string) => (id === m.nodes.find((n) => n.focus)?.id ? focusH : nodeH);
  const colH = (c: string[]) => c.reduce((s, id) => s + height(id), 0) + Math.max(0, c.length - 1) * gapY;
  const tall = Math.max(...cols.map(colH), focusH);
  const centre = new Map<string, number>();
  const placed = new Map<string, Placed>();
  for (let d = 0; d <= m.maxDepth; d++) {
    const col = cols[d]!;
    const key = (id: string): number => {
      const ts = m.edges.filter((e) => e.from === id && !e.selfLoop && centre.has(e.to)).map((e) => centre.get(e.to)!);
      return ts.length ? ts.reduce((a, b) => a + b, 0) / ts.length : Number.MAX_SAFE_INTEGER;
    };
    const sorted = d === 0 ? col : [...col].sort((a, b) => key(a) - key(b) || index.get(a)! - index.get(b)!);
    let y = pad + (tall - colH(sorted)) / 2;
    const x = pad + (m.maxDepth - d) * (nodeW + gapX);
    for (const id of sorted) {
      const h = height(id);
      placed.set(id, { id, x, y, w: nodeW, h, depth: d });
      centre.set(id, y + h / 2);
      y += h + gapY;
    }
  }
  const nodes = [...placed.values()];
  return { width: pad * 2 + (m.maxDepth + 1) * nodeW + m.maxDepth * gapX, height: pad * 2 + tall, nodes, byId: placed };
}

/** True when two placed boxes overlap (used by the tests; the layout never produces it). */
export const overlaps = (a: Placed, b: Placed): boolean => a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;

// ---- Re-centring with a history (the back button) ----

export interface CausesState { quantity: string; trail: string[]; depth?: number; view?: 'graph' | 'list' }
/** Make `id` the centre, remembering the one left. Re-centring on the same quantity changes nothing. */
export const recentre = (s: CausesState, id: string): CausesState => (id === s.quantity ? s : { ...s, quantity: id, trail: [...s.trail, s.quantity] });
/** Go back to the previous centre; at the start it changes nothing. */
export const back = (s: CausesState): CausesState => (s.trail.length ? { ...s, quantity: s.trail[s.trail.length - 1]!, trail: s.trail.slice(0, -1) } : s);

/** The depth the view opens at: the middle of the range on a roomy screen, the nearest causes only on a narrow one (the wire says how deep it goes). Presentational. */
export const startDepth = (maxDepth: number, narrow: boolean): number => (narrow ? 1 : Math.min(maxDepth, 2));
/** Screens at most this wide open on the list and depth 1 (the app itself needs 720 px: below that it shows a "desktop only" note). Presentational. */
export const NARROW_PX = 900;
