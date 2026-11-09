import type { LeverKey } from '../../contract';
import { DATA, LEVER_BY_KEY } from './data';
import { noise } from './rng';
import { numericValue, opposition } from './rules';
import type { MockState } from './state';

/** Value actually in force: lobbied levers are weakened toward their default. */
export function effectiveValue(s: MockState, key: LeverKey): number {
  const l = LEVER_BY_KEY.get(key)!;
  const nominal = numericValue(s.policy[key]);
  const def = numericValue(l.default);
  return def + (nominal - def) * (1 - opposition(s, key));
}

type Totals = Record<'gdp' | 'unemployment' | 'gini' | 'emissions' | 'legitimacy', number>;

export function policyTotals(s: MockState): Totals {
  const t: Totals = { gdp: 0, unemployment: 0, gini: 0, emissions: 0, legitimacy: 0 };
  for (const l of DATA.levers) {
    const range = l.max - l.min || 1;
    const delta = (effectiveValue(s, l.key) - numericValue(l.default)) / range;
    for (const [k, w] of Object.entries(l.effect)) t[k as keyof Totals] += w * delta;
  }
  for (const [kind, inf] of Object.entries(DATA.infra)) {
    const built = (s.infra[kind] ?? 0) - (kind === 'public_housing' ? DATA.start.publicHousing! : kind === 'fossil_plant' ? DATA.start.fossilCap! : DATA.start.renewableCap!);
    t.emissions += (inf.effect.emissions ?? 0) * built;
    t.gdp += (inf.effect.gdp ?? 0) * built;
  }
  return t;
}

const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));

/** One month of canned dynamics. Not an economic model. */
export function stepMonth(s: MockState): { emissions: number; unemploymentDelta: number } {
  const d = DATA.dynamics;
  const t = policyTotals(s);
  s.gdp *= 1 + (d.baseGrowth! + t.gdp) / 12 + noise(s.rng, d.noise!);
  const target = clamp(d.baseUnemployment! + t.unemployment, 0.01, 0.4);
  const prevU = s.unemployment;
  s.unemployment = clamp(prevU + (target - prevU) * d.unemploymentPull! + noise(s.rng, d.noise!), 0.01, 0.45);
  s.gini = clamp(s.gini + (DATA.start.gini! + t.gini - s.gini) * 0.1 + noise(s.rng, 0.002), 0.1, 0.8);
  s.legitimacy = clamp(
    s.legitimacy + d.legitimacyBase! + t.legitimacy / 12 * 0.3
      - d.legitimacyUnemploymentWeight! * (s.unemployment - d.baseUnemployment!) + noise(s.rng, 0.003), 0, 1);
  s.wellbeing = clamp(s.wellbeing + (0.9 - s.unemployment * 2 - s.gini * 0.3 - s.wellbeing) * 0.05 + noise(s.rng, 0.003), 0, 1);
  s.prices *= 1 + 0.0015 + noise(s.rng, 0.0005);
  s.forest = clamp(s.forest - d.forestDecayMonthly! * (s.commonsMode === 'open' ? 1 : 0.3) + noise(s.rng, 0.0003), 0, 1);
  s.cash += (s.gdp / 12) * (0.004 + noise(s.rng, 0.003));
  s.debt = Math.max(0, -s.cash);
  const gdpIndex = s.gdp / DATA.start.gdp!;
  const emissions = Math.max(0, d.baseMonthlyEmissions! * gdpIndex * (1 + t.emissions));
  s.lastMonthlyEmissions = emissions;
  s.cumulativeTonnes += emissions;
  if (s.transition && --s.transition.monthsLeft <= 0) {
    s.system = s.transition.to;
    s.transition = null;
  }
  s.tick++;
  return { emissions, unemploymentDelta: s.unemployment - prevU };
}

export const remainingBudget = (s: MockState) => Math.max(0, DATA.dynamics.carbonBudget! - s.cumulativeTonnes);
