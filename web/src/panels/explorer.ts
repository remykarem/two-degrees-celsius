import type { AssumptionGroup, AssumptionTopic } from '../contract';

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
