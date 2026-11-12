import type { GapKind, MarketCategoryDef, MarketShort, MarketView, MechanismDef, Snapshot } from '../contract';
import type { Selection } from '../store';
import { catalog, fill, text } from '../store/catalog';
import { entitySelection } from './trades';

/**
 * Pure views of the Markets view (contract v2.3, docs/features/markets.md). Every figure, sentence, label, unit, cause word and
 * mechanism tag is the wire's (`snapshot.markets`, `catalog().markets`); the web only lays the cards out, scales the bars and draws
 * the price line. It knows no market by name: a market that appears in the list is drawn like every other.
 */
const t = (key: string): string => text('markets', key);

/** The arrow for a price direction id (a wire id; the word beside it is the wire's). */
export const DIRECTION_ARROW: Record<string, string> = { rising: '▲', steady: '■', falling: '▼', unknown: '', none: '' };

export interface StripBar { label: string; value: string; unit: string; width: number; kind: 'wanted' | 'available' }
export interface ShortChip { id: string; label: string; selection: Selection | null }
export interface MechanismTag { id: string; label: string; blurb: string }
export interface MarketCardModel {
  id: string; label: string; blurb: string; unit: string;
  /** Two bars on one scale: what was wanted and what was available (widths 0 to 1). */
  bars: [StripBar, StripBar];
  /** The traded line: "Bought 545 units". */
  traded: string;
  gap: string[];
  gapKind: GapKind;
  verdict: string;
  /** How this price is really set (the wire's sentence), also for a market with no price. */
  note: string;
  price: { text: string; unit: string; label: string; arrow: string; direction: string; word: string; cause: string } | null;
  series: number[];
  drivers: { id: string; label: string; value: string }[];
  chips: ShortChip[];
  mechanisms: MechanismTag[];
  how: string;
  /** A link to another view from the card's content (the food card to Ecology): the wire's label and anchor. */
  link: { label: string; target: string } | null;
  reconciles: boolean;
}
export interface MarketGroupModel { id: string; label: string; blurb: string; cards: MarketCardModel[] }
export interface MarketsModel {
  ready: boolean;
  title: string; empty: string;
  intro: { title: string; body: string; notCapitalism: string; link: string; linkAnchor: string };
  mechanisms: { title: string; foot: string; examplePrefix: string; items: (MechanismDef & { example: string })[] };
  costPlus: { title: string; body: string; anchor: string };
  smallTown: string;
  groups: MarketGroupModel[];
}

/** A chip's inspector target (the wire says what kind of thing and which one). */
export const selectionOf = (s: MarketShort): Selection | null => (s.entity ? entitySelection(s.entity) : null);

const tags = (m: MarketView, all: MechanismDef[]): MechanismTag[] =>
  m.mechanisms.map((id) => all.find((x) => x.id === id)).filter((x): x is MechanismDef => !!x).map((x) => ({ id: x.id, label: x.label, blurb: x.blurb }));

export function cardModel(m: MarketView, mechanisms: MechanismDef[]): MarketCardModel {
  const scale = Math.max(m.demand, m.supply, 1e-9);
  const w = (v: number) => Math.max(0, Math.min(1, v / scale));
  return {
    id: m.id, label: m.label, blurb: m.blurb, unit: m.unit,
    bars: [
      { label: m.demandLabel, value: m.demandText, unit: m.unit, width: w(m.demand), kind: 'wanted' },
      { label: m.supplyLabel, value: m.supplyText, unit: m.unit, width: w(m.supply), kind: 'available' },
    ],
    traded: `${m.tradedLabel}: ${m.tradedText}`,
    gap: m.gapWords, gapKind: m.gapKind, verdict: m.verdict,
    price: m.priceText === undefined ? null : {
      text: m.priceText, unit: m.priceUnit, label: m.priceLabel, arrow: DIRECTION_ARROW[m.priceDirection] ?? '', direction: m.priceDirection, word: m.priceDirectionWord,
      cause: m.priceCauseWord,
    },
    note: m.priceNote,
    series: m.priceSeries,
    drivers: m.drivers, chips: m.short.map((s) => ({ id: s.id, label: s.label, selection: selectionOf(s) })),
    mechanisms: tags(m, mechanisms), how: m.howId, link: m.link ?? null, reconciles: m.reconciles,
  };
}

/** The whole view: categories in content order, cards in wire order, and only categories that have a card. */
export function marketsModel(snap: Snapshot | null): MarketsModel {
  const c = catalog().markets;
  const cats: MarketCategoryDef[] = c.categories;
  const list = snap?.markets ?? [];
  const known = new Set(cats.map((x) => x.id));
  const groups = cats.map((cat) => ({ id: cat.id, label: cat.label, blurb: cat.blurb, cards: list.filter((m) => m.category === cat.id).map((m) => cardModel(m, c.mechanisms)) }))
    .filter((g) => g.cards.length > 0);
  // A market whose category the content does not list is still shown, so nothing on the wire is hidden.
  const stray = list.filter((m) => !known.has(m.category));
  if (stray.length > 0) groups.push({ id: 'other', label: t('also_here'), blurb: '', cards: stray.map((m) => cardModel(m, c.mechanisms)) });
  return {
    ready: list.length > 0, title: t('title'), empty: t('empty'),
    intro: { title: t('intro_title'), body: t('intro_body'), notCapitalism: t('intro_not_capitalism'), link: t('intro_link'), linkAnchor: 'markets:intro' },
    mechanisms: { title: t('mechanisms_title'), foot: t('mechanisms_foot'), examplePrefix: t('example_prefix'), items: c.mechanisms },
    costPlus: { title: t('cost_plus_title'), body: t('cost_plus_body'), anchor: 'markets:prices' },
    smallTown: t('small_town_note'),
    groups,
  };
}

/** Every `{name}` placeholder left in a text is a bug in the content (used by the tests). */
export const hasPlaceholder = (s: string): boolean => /\{\w+\}/.test(fill(s, {}));
