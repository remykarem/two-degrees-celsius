import type { AgentKindInfo, AgentsCatalog, AgentVariant, AssumptionGroup, AssumptionTopic, Distribution, Snapshot } from '../contract';

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
