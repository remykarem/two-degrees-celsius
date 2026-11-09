import type { CollapseReason, GameOutcome, LeverKey } from '../../contract';
import { GEN, INFRA_KINDS, LEVERS, LEVER_BY_KEY, MOCK } from './data';
import { noise } from './rng';
import { activeNumeric, spend } from './rules';
import { gini, netWorth } from './people';
import { baseEmissions, buildingCounts, type MockState } from './state';

// MOCK ONLY. Everything in this file other than the rule hooks (lobbying gap, outcome checks) is an invented toy economy.
const D = MOCK.dynamics;
const X = MOCK.extra;

/** Value in force after lobbying (S9b), for the four lobbied levers. Mock gap model: the change from the default is scaled by (1 - net_lobby). */
export function effectiveValue(s: MockState, key: LeverKey): number {
  const l = LEVER_BY_KEY.get(key)!;
  const nominal = activeNumeric(s, l);
  if (key === 'emissions_cap') return nominal > 0 ? nominal / (1 - Math.min(0.95, s.netLobby)) : 0;
  const def = Number(l.default);
  return def + (nominal - def) * (1 - s.netLobby);
}

type Totals = Record<'gdp' | 'unemployment' | 'gini' | 'emissions' | 'legitimacy', number>;

export function policyTotals(s: MockState): Totals {
  const t: Totals = { gdp: 0, unemployment: 0, gini: 0, emissions: 0, legitimacy: 0 };
  for (const l of LEVERS) {
    if (l.unit === 'enum' || l.unit === 'bool') continue;
    const range = l.max - l.min || 1;
    const value = l.lobbied ? effectiveValue(s, l.key) : activeNumeric(s, l);
    const delta = l.key === 'emissions_cap' ? 0 : (value - Number(l.default)) / range;
    for (const [k, w] of Object.entries(l.effect)) t[k as keyof Totals] += (w ?? 0) * delta;
  }
  const cap = effectiveValue(s, 'emissions_cap');
  const strict = cap > 0 ? 1 - Math.min(1, cap / LEVER_BY_KEY.get('emissions_cap')!.max) : 0; // 0 = no cap
  t.emissions += X.capStrictnessEmissions! * strict;
  t.gdp += X.capStrictnessGdp! * strict;
  const start = { public_housing: GEN.build.start.publicHousing, fossil_plant: GEN.build.start.fossilPlant, renewable_plant: GEN.build.start.renewablePlant, road: GEN.build.start.road };
  const counts = buildingCounts(s);
  for (const kind of INFRA_KINDS) {
    const built = counts[kind] - start[kind];
    t.emissions += (MOCK.infraEffect[kind]?.emissions ?? 0) * built;
    t.gdp += (MOCK.infraEffect[kind]?.gdp ?? 0) * built;
  }
  return t;
}

const clamp = (x: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, x));
/** Wealth Gini over people (each member carries an equal share of the household's net worth). */
export const wealthGini = (s: MockState): number => gini(s.households.flatMap((h) => h.members.map(() => netWorth(s, h) / h.members.length)));
export const remainingBudget = (s: MockState) => Math.max(0, GEN.carbon.budget - s.cumulativeTonnes);

export interface MonthOut { emissions: number; unemploymentDelta: number; bankFailedNow: boolean; bailoutAmount: number }

/** One month of toy dynamics. */
export function stepMonth(s: MockState): MonthOut {
  const t = policyTotals(s);
  const u0 = GEN.town.unemployment;
  s.gdp *= 1 + (D.baseGrowth! + t.gdp) / 12 + noise(s.rng, D.noise!);
  if (s.bank.failed) s.gdp *= 0.995;
  const target = clamp(u0 + t.unemployment + (s.bank.failed ? X.bankFailureUnemployment! : 0), 0.01, 0.6);
  const prevU = s.unemployment;
  s.unemployment = clamp(prevU + (target - prevU) * D.unemploymentPull! + noise(s.rng, D.noise!), 0.01, 0.7);
  s.gini = clamp(s.gini + (s.gini0 + t.gini - s.gini) * 0.1 + noise(s.rng, 0.002), 0.1, 0.9);
  s.legitimacy = clamp(s.legitimacy + D.legitimacyBase! + (t.legitimacy / 12) * 0.3 - D.legitimacyUnemploymentWeight! * (s.unemployment - u0) + noise(s.rng, 0.003), 0, 1);
  s.wellbeing = clamp(s.wellbeing + (0.9 - s.unemployment * 2 - s.gini * 0.3 - s.wellbeing) * 0.05 + noise(s.rng, 0.003), 0, 1);
  s.prices *= 1 + 0.0015 + noise(s.rng, 0.0005);
  const open = s.policy.commons_access === 'open';
  s.forest = clamp(s.forest - D.forestDecayMonthly! * (open ? 1 : 0.3) + noise(s.rng, 0.0003), 0, 1);
  s.commons = clamp(s.commons + (open ? -1 : 1) * 0.0004 * (Number(s.policy.commons_harvest) > 1 ? 2 : 0.5) + noise(s.rng, 0.0003), 0, 1);
  spend(s, -((s.gdp / 12) * (0.004 + noise(s.rng, 0.003)))); // negative spend = receipts
  const emissions = Math.max(0, baseEmissions() * (s.gdp / (GEN.derived.monthlyOutputValue * 12)) * (1 + t.emissions));
  s.lastMonthlyEmissions = emissions;
  s.cumulativeTonnes += emissions;

  // Bank (mock): loses equity to defaults that rise with unemployment and the base rate, earns a small margin.
  let bankFailedNow = false, bailoutAmount = 0;
  if (!s.bank.failed) {
    const defaultRate = 0.005 + 0.6 * Math.max(0, s.unemployment - 0.1) + 0.5 * Math.max(0, Number(s.policy.base_rate) - 0.08);
    s.bank.equity += s.bank.loans * (0.0006 - defaultRate / 12);
    if (s.bank.equity < 0) {
      if (s.policy.bailout === true) {
        bailoutAmount = 0.12 * s.bank.loans - s.bank.equity;
        s.bank.equity = 0.12 * s.bank.loans;
        s.debt += bailoutAmount;
        s.legitimacy = clamp(s.legitimacy - 0.05, 0, 1);
      } else { s.bank.failed = true; bankFailedNow = true; }
    }
  }

  if (s.transition && --s.transition.monthsLeft <= 0) {
    s.system = s.transition.to;
    s.transition = null;
  }
  s.tick++;
  return { emissions, unemploymentDelta: s.unemployment - prevU, bankFailedNow, bailoutAmount };
}

/** Mirrors check_outcome in crates/sim-engine/src/lib.rs (same order). */
export function checkOutcome(s: MockState): { outcome: GameOutcome; event: 'budget_exhausted' | CollapseReason } | null {
  if (s.cumulativeTonnes >= GEN.carbon.budget) return { outcome: 'budget_exhausted', event: 'budget_exhausted' };
  if (s.legitimacy <= 0) return { outcome: { collapse: 'legitimacy_zero' }, event: 'legitimacy_zero' };
  if (s.unemployment > GEN.collapseUnemployment) return { outcome: { collapse: 'mass_unemployment' }, event: 'mass_unemployment' };
  if (s.bank.failed && s.unemployment > 0.2) return { outcome: { collapse: 'bank_failure_cascade' }, event: 'bank_failure_cascade' };
  return null;
}

/** Between turns: lobbying pressure drifts toward a level set by the system and union density (mock). */
export function updateLobby(s: MockState): void {
  const target = clamp(GEN.systems[s.system].lambdaMax * X.lobbyTargetFactor! - GEN.town.unionDensity * X.lobbyUnionWeight!, 0, 0.95);
  s.netLobby = clamp(s.netLobby + (target - s.netLobby) * X.lobbySmoothing! + noise(s.rng, D.lobbyWander!), 0, 0.95);
}
