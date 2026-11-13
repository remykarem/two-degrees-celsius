import type { AgentKindInfo, EdgeEffect, EdgeStrength, CausalLoop, AgentsCatalog, AgentVariant, AssumptionGroup, AssumptionTopic, CausalGraph, CausalNode, Catalog, DecisionInfo, Distribution, MechanismInfo, Snapshot } from '../contract';
import { fill } from '../store/catalog';
import { compact } from './format';

/** Pure logic of the "How it works" explorer. The catalogue (text, parameters, examples) comes from the model through SimPort.assumptions(). */
export const GROUP_LABEL: Record<AssumptionGroup, string> = {
  emissions_energy: 'Emissions and energy', people_households: 'People and households', production_prices: 'Production and prices', nature_climate: 'Nature and climate', weather: 'Weather',
  government_policy: 'Government and policy', finance_credit: 'Finance and credit', class_politics: 'Class and politics', scoring: 'Scoring', leaves_out: 'What the model leaves out',
};
export const STATUS_WORD: Record<AssumptionTopic['status'], string> = { placeholder: 'Placeholder', tuned: 'Tuned for play', rough_real_world: 'Rough real-world' };

const text = (t: AssumptionTopic): string => [t.title, t.summary, t.formula, t.statusNote, ...t.leavesOut, ...t.params.map((p) => `${p.label} ${p.path}`), ...t.examples.map((e) => e.label), ...t.usedBy].join(' ').toLowerCase();

/** Case-insensitive AND search over title, summary, formula, parameters, examples and anchors. An empty query keeps everything. */
export function searchTopics(topics: AssumptionTopic[], query: string, status: AssumptionTopic['status'] | 'all' = 'all'): AssumptionTopic[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return topics.filter((t) => (status === 'all' || t.status === status) && words.every((w) => text(t).includes(w)));
}
export function groupTopics(topics: AssumptionTopic[]): { group: AssumptionGroup; label: string; topics: AssumptionTopic[] }[] {
  const order = Object.keys(GROUP_LABEL) as AssumptionGroup[];
  return order.map((g) => ({ group: g, label: GROUP_LABEL[g], topics: topics.filter((t) => t.group === g) })).filter((g) => g.topics.length);
}
/** The topic a "How is this calculated?" link opens for a UI anchor (a metric key, lever key or panel id): the first topic that lists it in `usedBy`. */
export const topicFor = (topics: AssumptionTopic[], anchor: string): AssumptionTopic | null => topics.find((t) => t.usedBy.includes(anchor)) ?? null;
/** Anchors with no topic (a test fails when a metric or lever is in this list). */
export const anchorsWithoutTopic = (topics: AssumptionTopic[], anchors: string[]): string[] => anchors.filter((a) => !topicFor(topics, a));
/** Related topics that exist (dangling ids are dropped, and a test checks there are none). */
export const relatedOf = (topics: AssumptionTopic[], t: AssumptionTopic): AssumptionTopic[] => t.related.map((id) => topics.find((x) => x.id === id)).filter((x): x is AssumptionTopic => !!x);

// ---- Agents section (contract v1.7) ----

/** The explorer's section id and the prefix a person link uses (`agents:worker` opens Households on that class). */
export const AGENTS_ANCHOR = 'agents';
export const agentsAnchorFor = (cls: string): string => `${AGENTS_ANCHOR}:${cls}`;
export const isAgentsAnchor = (a: string | null | undefined): boolean => !!a && (a === AGENTS_ANCHOR || a.startsWith(`${AGENTS_ANCHOR}:`));
/** The kind an agents link names: `agents:<class>` is Households, a bare kind id (or `agents:<kind id>`) is that kind. */
export function kindForAnchor(c: AgentsCatalog, anchor: string): AgentKindInfo | null {
  const rest = anchor.startsWith(`${AGENTS_ANCHOR}:`) ? anchor.slice(AGENTS_ANCHOR.length + 1) : '';
  return c.kinds.find((k) => k.id === rest) ?? (rest && c.kinds[0]?.variants.some((v) => v.id === rest) ? c.kinds[0] : null) ?? c.kinds[0] ?? null;
}
export const DISTRIBUTION_WORD: Record<Distribution, string> = { bernoulli: 'Yes/no draw (Bernoulli)', uniform: 'Uniform on 0 to 1', uniform_index: 'Uniform pick from a list' };
export const ACTOR_WORD: Record<AgentKindInfo['actor'], string> = { agent: 'Agent', non_agent: 'Not an agent' };
/** Case-insensitive AND search over kinds (title, summary, decisions, rule) and draws (event, probability, when). */
export function searchAgents(c: AgentsCatalog, query: string): { kinds: AgentKindInfo[]; draws: AgentsCatalog['randomness'] } {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const hit = (t: string) => words.every((w) => t.toLowerCase().includes(w));
  return {
    kinds: c.kinds.filter((k) => hit([k.title, k.summary, k.rule, ...k.decides, ...k.observes, ...k.params.map((p) => `${p.label} ${p.path}`)].join(' '))),
    draws: c.randomness.filter((r) => hit([r.event, r.probability, r.when, r.system, ...r.params.map((p) => `${p.label} ${p.path}`)].join(' '))),
  };
}
/** The one-line decision tag of a person: the class of their household and how such households decide. Null when the catalogue or the household is unavailable. */
export function decisionTag(c: AgentsCatalog | null, snap: Snapshot | null, person: number): { cls: string; variant: AgentVariant } | null {
  if (!c || !snap) return null;
  const h = snap.households.find((x) => x.members.includes(person));
  const v = h && c.kinds.find((k) => k.id === 'households')?.variants.find((x) => x.id === h.class);
  return h && v ? { cls: h.class, variant: v } : null;
}

// ---- Decisions (contract v2.7): generated from the registry, rendered generically ----

/** The registered decisions that belong to an Agents kind (`catalog.decisions[].agentKind`). Nothing here names a decision: a new one in the catalogue appears. */
export const decisionsOfKind = (decisions: DecisionInfo[], kindId: string): DecisionInfo[] => decisions.filter((d) => d.agentKind === kindId);
/** One text line per decision: what it reads, the decision, what it writes. Labels come from the wire. */
export const decisionLine = (d: DecisionInfo): { inputs: string; title: string; outputs: string } => ({
  inputs: d.inputs.map((q) => q.label).join(', '),
  title: d.title,
  outputs: d.outputs.map((q) => q.label).join(', '),
});

/** The registered mechanisms (plain physics and accounting code, not choices) that an Agents kind lists (`catalog.mechanisms[].agentKind`). Nothing here names one: a new one in the catalogue appears (contract v2.10). */
export const mechanismsOfKind = (mechanisms: MechanismInfo[], kindId: string): MechanismInfo[] => mechanisms.filter((m) => m.agentKind === kindId);
/** One text line per mechanism: what it reads, the mechanism, what it writes. Labels come from the wire. */
export const mechanismLine = (m: MechanismInfo): { inputs: string; title: string; outputs: string } => ({
  inputs: m.inputs.map((q) => q.label).join(', '),
  title: m.title,
  outputs: m.outputs.map((q) => q.label).join(', '),
});

// ---- Causal edges (contract v2.8): read from catalog.causal, nothing here names a decision or a quantity ----

export interface CausalLines {
  /** What the decision reads, each marked when it is a starting point or an end point (and with a loop mark when the decision also writes it). */
  inputs: string;
  /** What it writes, marked the same way. */
  outputs: string;
  /** For each written quantity nobody else reads (an outcome): the quantities that can change it, nearest first. */
  upstream: { id: string; label: string; causes: string }[];
  /** The quantities this node reads or writes that the model can explain (they have causes), each opens the Why? view. */
  quantities: { id: string; label: string }[];
}

/** The loop mark for a quantity a decision both reads and writes. */
export const LOOP_MARK = '\u21ba';

/**
 * The live value of a quantity as a short phrase ("4.2 units per month, rising"), or null when it has none (unbound in the catalogue
 * with its reason, or not on this snapshot). Everything comes from the wire: `snapshot.quantities` (value, trend id), the catalogue's
 * unit and binding, and the Markets direction words (`markets.directions`) for the trend. Nothing here names a quantity (contract v2.9).
 */
export type LiveLookup = (quantityRef: string) => string | null;
export function liveLookup(c: Catalog, snap: Snapshot | null): LiveLookup {
  const values = new Map((snap?.quantities ?? []).map((l) => [l.id, l]));
  const info = new Map(c.quantities.map((q) => [q.id, q]));
  return (ref) => {
    const l = values.get(ref);
    const q = info.get(ref);
    if (!l || !q) return null;
    const trend = l.trend ? c.markets.directions[l.trend] : undefined;
    const body = q.live.aggregate === 'flag' ? (l.value >= 0.5 ? 'yes' : 'no') : `${compact(l.value)} ${q.unit}`;
    return trend && l.trend !== 'steady' ? `${body}, ${trend}` : body;
  };
}

/** The causal lines of one decision, from the `reads` and `writes` edges that touch its node. `live` adds each cause's current value. */
export const causalLines = (g: CausalGraph, decisionId: string, live?: LiveLookup): CausalLines => nodeLines(g, `decision:${decisionId}`, live);

/** The same lines for any decision or mechanism node (`decision:<id>` or `mechanism:<id>`; contract v2.10). */
export function nodeLines(g: CausalGraph, nodeId: string, live?: LiveLookup): CausalLines {
  const byId = new Map(g.nodes.map((n) => [n.id, n]));
  const roleLabel = (n: CausalNode): string => g.roles.find((r) => r.id === n.role)?.label ?? '';
  const name = (n: CausalNode, loop: boolean): string => {
    const mark = n.kind === 'quantity' && (n.role === 'source' || n.role === 'sink') ? ` (${roleLabel(n)})` : '';
    return `${n.label}${mark}${loop ? ` ${LOOP_MARK}` : ''}`;
  };
  const me = nodeId;
  const ins = g.edges.filter((e) => e.kind === 'reads' && e.to === me);
  const outs = g.edges.filter((e) => e.kind === 'writes' && e.from === me);
  const label = (id: string, loop: boolean): string => {
    const n = byId.get(id);
    return n ? name(n, loop) : id;
  };
  const upstream = outs.flatMap((e) => {
    const n = byId.get(e.to);
    if (!n || n.role !== 'sink' || n.upstream.length === 0) return [];
    const cause = (u: string): string => {
      const un = byId.get(u);
      const v = un && live ? live(un.refId) : null;
      return `${un?.label ?? u}${v ? ` (${v})` : ''}`;
    };
    return [{ id: n.id, label: n.label, causes: n.upstream.map(cause).join(', ') }];
  });
  const seen = new Set<string>();
  const quantities = [...ins.map((e) => e.from), ...outs.map((e) => e.to)].flatMap((id) => {
    const n = byId.get(id);
    if (!n || n.causes.length === 0 || seen.has(id)) return [];
    seen.add(id);
    return [{ id, label: n.label }];
  });
  return { inputs: ins.map((e) => label(e.from, e.selfLoop)).join(', '), outputs: outs.map((e) => label(e.to, e.selfLoop)).join(', '), upstream, quantities };
}

/** Sign glyphs. The ids come from the wire (`CausalEdge.effect.sign`); only their drawing is here. */
export const SIGN_GLYPH: Record<string, string> = { up: '+', down: '-', varies: '+/-', none: '0' };

export interface EffectLine {
  /** `wage (+) -> unit cost, the same month` */
  text: string;
  sign: string;
  /** The wire's sentence ("When wage goes up, ..."), shown on hover and on expand. */
  sentence: string;
  /** How the sign was checked, in words (wire). */
  checked: string;
  basis: string;
}

/** How an effect was checked, in words: the template for its basis from the catalogue (`causal.words`), with the reason filled in. */
export function checkedText(g: CausalGraph, fx: EdgeEffect): string {
  const t = fx.basis === 'system' ? g.words.checkedSystem : fx.basis === 'declared_unverified' ? g.words.checkedDeclaredUnverified : g.words.checkedPerturbed;
  return fill(t, { reason: fx.reason ?? '' });
}

/** The sentence of one link's size: the template for its kind (`causal.strength.edge*`) filled from the wire's numbers. */
export function edgeSizeText(g: CausalGraph, s: EdgeStrength): string {
  const t = g.strength;
  const template = s.kind === 'elasticity' ? t.edgeElasticity : s.kind === 'slope' ? t.edgeSlope : s.kind === 'switch' ? t.edgeSwitch : t.edgeUnmeasured;
  const word = (t.words.find((w) => w.id === s.word)?.label ?? '').toLowerCase();
  return fill(template, { word, pct: s.pct ?? '', value: s.label, low: s.lowLabel ?? '', high: s.highLabel ?? '', n: s.samples, of: s.total });
}

/** The delay round a loop in words, from the catalogue templates. */
export function loopDelayText(g: CausalGraph, l: CausalLoop): string {
  return l.delayMonths === 1 ? g.words.delayOne : fill(g.words.delayMany, { n: l.delayMonths });
}

/** One line per input -> output pair of a decision or mechanism node (contract v2.11): the sign, the lag and the sentence all come from the wire, so a made-up effect added only to the catalogue renders with no change here. */
export function effectLines(g: CausalGraph, nodeId: string): EffectLine[] {
  const byId = new Map(g.nodes.map((n) => [n.id, n]));
  const label = (id: string): string => byId.get(id)?.label ?? id;
  return g.edges
    .filter((e) => e.kind === 'drives' && e.via === nodeId && e.effect)
    .map((e) => {
      const fx = e.effect!;
      const glyph = SIGN_GLYPH[fx.sign] ?? fx.sign;
      const tail = fx.sign === 'none' ? 'no direct effect' : fx.lagLabel;
      return { text: `${label(e.from)} (${glyph}) -> ${label(e.to)}, ${tail}`, sign: fx.sign, sentence: fx.sentence, checked: checkedText(g, fx), basis: fx.basis };
    });
}

export interface LoopLine {
  id: string;
  /** The wire's kind id (`R`, `B`, `depends`): picks the badge shape, never read as a rule. */
  kind: string;
  /** The wire's badge letter (`R`, `B`, `?`) and kind label, shown as text so the kind never rests on colour alone. */
  badge: string;
  kindLabel: string;
  /** The loop's name when the content has one, else its quantities in cycle order (`Wage -> Unit cost -> ... -> Wage`). */
  title: string;
  /** The quantities in cycle order, closed back to the first. */
  chain: string;
  /** The wire's sentence. */
  sentence: string;
  delay: string;
  story: string | null;
  /** Labels of the reasons the loop is incomplete (wire words); empty when it is not. */
  incomplete: string[];
}

/**
 * The feedback loops (contract v2.12) that pass through any of the given nodes, in the wire's order (shortest first, a loop of a single quantity last). Membership comes from the nodes'
 * `inLoops`, the kind words, the sentence, the delay and the incomplete reasons from the wire; this only joins and lays them out, so a loop
 * added only to the catalogue renders with no change here.
 */
export function loopLines(g: CausalGraph, nodeIds: string[]): LoopLine[] {
  const wanted = new Set(g.nodes.filter((n) => nodeIds.includes(n.id)).flatMap((n) => n.inLoops));
  const byId = new Map(g.nodes.map((n) => [n.id, n]));
  const label = (id: string): string => byId.get(id)?.label ?? id;
  // Ordering only: loops of several quantities first, then a quantity's own carry-over (a loop of one), each in the wire's order.
  const mine = g.loops.filter((l) => wanted.has(l.id));
  return [...mine.filter((l) => l.members.length > 1), ...mine.filter((l) => l.members.length === 1)]
    .map((l) => {
      const k = g.loopKinds.find((x) => x.id === l.kind);
      const chain = [...l.members, l.members[0]!].map(label).join(' -> ');
      return {
        id: l.id,
        kind: l.kind,
        badge: k?.badge ?? l.kind,
        kindLabel: k?.label ?? l.kind,
        title: l.name ?? chain,
        chain,
        sentence: l.sentence,
        delay: loopDelayText(g, l),
        story: l.story ?? null,
        incomplete: l.incompleteReasons.map((r) => g.loopIncomplete.find((x) => x.id === r)?.label ?? r),
      };
    });
}

/** "In 12 feedback loops: 5 Reinforcing, 6 Balancing, 1 Depends" for one decision or mechanism node, or '' when it is in none. */
export function loopSummary(g: CausalGraph, nodeId: string): string {
  const lines = loopLines(g, [nodeId]);
  if (!lines.length) return '';
  const parts = g.loopKinds.map((k) => ({ k, n: lines.filter((l) => l.kind === k.id).length })).filter((p) => p.n > 0);
  return `In ${lines.length} feedback loop${lines.length === 1 ? '' : 's'}: ${parts.map((p) => `${p.n} ${p.k.label.toLowerCase()}`).join(', ')}`;
}
