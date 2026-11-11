//! S8 government: collect taxes, pay benefits, build and maintain infrastructure,
//! borrow from the bank for deficits, update legitimacy.

use crate::util::{apportion, cfg, labour_weights};
use sim_metrics::household_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let gp = &p.government;
    let gov = Account::Government;

    // ---- Taxes: only what households can actually pay is sent, and that is what is reported ----
    let wealth = household_wealth(s, p);
    let mut taxes = Cents::ZERO;
    for (i, h) in s.households.iter().enumerate() {
        let me = Account::Household(h.id);
        let wage_tax = crate::util::tax_on(h.income.wages, s.policy.tax_wage.0);
        let cap_tax = crate::util::tax_on(h.income.capital_income(), s.policy.tax_capital.0);
        // The wealth tax falls on a household's net worth above the threshold.
        let wealth_tax = crate::util::wealth_tax_monthly(
            wealth[i],
            s.policy.wealth_threshold,
            s.effective.wealth_tax.0,
        );
        let mut left = h.savings.max(Cents::ZERO);
        for (amt, purpose) in [
            (wage_tax, Purpose::WageTax),
            (cap_tax, Purpose::CapitalTax),
            (wealth_tax, Purpose::WealthTax),
        ] {
            let paid = amt.min(left);
            left -= paid;
            taxes += paid;
            d.transfer_as(me, gov, paid, FlowKind::Tax, purpose);
        }
    }
    // Firm-side receipts were collected in S4 this tick: the amounts actually paid.
    let firm_side = Cents::sum(
        s.firms
            .iter()
            .filter(|f| f.active)
            .map(|f| f.carbon_cost + f.tax_paid),
    );
    let receipts = taxes + firm_side;

    // ---- Committed spending this tick ----
    let mut spend = Deltas::new();
    let mut benefits = Cents::ZERO;
    let mut ubi = Cents::ZERO;
    let mut jg_wages = Cents::ZERO;
    let min_wage = s.policy.min_wage.scale(cfg(s, p).min_wage_multiplier);
    // Each payment is one amount (scaled and clamped as a whole) that the ledger then splits by
    // purpose; the parts always sum to the amount, so the money moved is unchanged.
    type Parts = Vec<(Cents, Purpose)>;
    let mut pay: Vec<(Account, Cents, FlowKind, Parts)> = Vec::new();
    for h in &s.households {
        let members: Vec<&Person> = h.members.iter().map(|m| s.person(*m)).collect();
        // Benefits go to unemployed adults and to retired members.
        let eligible = members
            .iter()
            .filter(|x| x.unemployed() || x.age_band == AgeBand::Retired)
            .count();
        let b = s.policy.benefit.scale(eligible as f64);
        let u = s.policy.ubi.scale(members.len() as f64);
        let jg_people = members.iter().filter(|x| x.jg).count();
        let j = min_wage.scale(jg_people as f64);
        let me = Account::Household(h.id);
        pay.push((
            me,
            b + u,
            FlowKind::Benefit,
            vec![(b, Purpose::Benefits), (u, Purpose::Ubi)],
        ));
        pay.push((me, j, FlowKind::Wage, vec![(j, Purpose::JobGuarantee)]));
        benefits += b;
        ubi += u;
        jg_wages += j;
    }
    let lw = labour_weights(s);
    let roads = s
        .map
        .cells
        .iter()
        .filter(|c| c.kind == TileKind::Road)
        .count() as f64;
    let public_homes = s
        .homes
        .iter()
        .filter(|h| h.owner == HomeOwner::Public)
        .count() as f64;
    let stations = s
        .map
        .cells
        .iter()
        .filter(|c| c.kind == TileKind::FireStation)
        .count() as f64;
    let upkeep = Cents::from_dollars(
        public_homes * gp.housing_upkeep_usd
            + s.plants.len() as f64 * gp.plant_upkeep_usd
            + roads * gp.road_upkeep_usd
            + stations * p.weather.fire_station_upkeep_usd,
    ) + s.policy.resilience_spending;
    let mut nature = s.nature.clone();
    let mut build_cost = Cents::ZERO;
    let (mut next_home, mut next_plant) = (s.next_home_id, s.next_plant_id);
    for (kind, tile) in &s.gov.pending_build {
        if s.map.kind_at(*tile) != Some(TileKind::Empty) {
            // The tile was taken between the order and the build: nothing is built or paid.
            d.events.push(Event::PolicyBlocked {
                reason: BlockReason::TileOccupied,
            });
            continue;
        }
        let cost = match kind {
            InfraKind::PublicHousing => {
                let id = HomeId(next_home);
                next_home += 1;
                let value = Cents::from_dollars(gp.public_housing_cost_usd);
                d.changes.push(Delta::AddHome(Home {
                    id,
                    tile: *tile,
                    owner: HomeOwner::Public,
                    household: None,
                    rent: s.housing.rent_public,
                    value,
                }));
                d.changes.push(Delta::SetTile(
                    *tile,
                    MapTile {
                        kind: TileKind::PublicHousing,
                        id: Some(id.0),
                    },
                ));
                value
            }
            InfraKind::FossilPlant | InfraKind::RenewablePlant => {
                let (pk, tk) = if *kind == InfraKind::FossilPlant {
                    (PlantKind::Fossil, TileKind::FossilPlant)
                } else {
                    (PlantKind::Renewable, TileKind::RenewablePlant)
                };
                let id = PlantId(next_plant);
                next_plant += 1;
                d.changes.push(Delta::AddPlant(Plant {
                    id,
                    kind: pk,
                    tile: *tile,
                    capacity: Units(p.energy.plant_capacity),
                    output: Units::ZERO,
                    emissions: Tonnes::ZERO,
                }));
                d.changes.push(Delta::SetTile(
                    *tile,
                    MapTile {
                        kind: tk,
                        id: Some(id.0),
                    },
                ));
                Cents::from_dollars(gp.plant_cost_usd)
            }
            InfraKind::Road => {
                d.changes.push(Delta::SetTile(
                    *tile,
                    MapTile {
                        kind: TileKind::Road,
                        id: None,
                    },
                ));
                Cents::from_dollars(gp.road_cost_usd)
            }
            InfraKind::FireStation => {
                d.changes.push(Delta::SetTile(
                    *tile,
                    MapTile {
                        kind: TileKind::FireStation,
                        id: None,
                    },
                ));
                Cents::from_dollars(p.weather.fire_station_cost_usd)
            }
        };
        d.events.push(Event::Built {
            kind: *kind,
            tile: *tile,
        });
        build_cost += cost;
        nature.pending_clearing += Units(cost.dollars() * p.nature.forest_clear_per_build);
    }
    let services = upkeep + build_cost;
    for (h, part) in apportion(services, &lw).into_iter().enumerate() {
        pay.push((
            Account::Household(HouseholdId(h as u32)),
            part,
            FlowKind::Wage,
            vec![(upkeep, Purpose::Upkeep), (build_cost, Purpose::Build)],
        ));
    }
    let committed = Cents::sum(pay.iter().map(|x| x.1));

    // ---- Funding: cash and taxes first, then borrowing within the bank's lending limit ----
    // The goods purchases of next month (the G lever) are held back in cash.
    let g_next = s.policy.public_spending.scale(gp.cash_target_months);
    let available = s.gov.cash + taxes;
    let need = committed + g_next - available;
    let mut borrowed = Cents::ZERO;
    if need > Cents::ZERO {
        let kappa = s.policy.bank_capital_ratio.0;
        let max_loans = crate::util::max_loans(s.bank.equity, kappa);
        let headroom = (max_loans - s.bank.loans).max(Cents::ZERO);
        borrowed = need.min(headroom);
        d.money.push(MoneyOp::Lend {
            to: gov,
            amount: borrowed,
        });
    } else {
        let repay = (-need).scale(gp.repay_share).min(s.gov.debt);
        d.money.push(MoneyOp::Repay {
            from: gov,
            amount: repay,
        });
    }
    // Committed payments that the treasury cannot fund are cut pro rata, and reported.
    let funds = available + borrowed;
    let ratio = if committed > funds {
        safe_div(funds.as_f64(), committed.as_f64(), 0.0)
    } else {
        1.0
    };
    let mut paid_total = Cents::ZERO;
    for (to, amt, kind, parts) in pay {
        let a = amt.scale(ratio).min(amt);
        paid_total += a;
        for (part, purpose) in split_by_weights(a, &parts) {
            spend.transfer_as(gov, to, part, kind, purpose);
        }
    }
    let shortfall = committed - paid_total;
    if shortfall > Cents::ZERO {
        d.events.push(Event::SpendingCut { shortfall });
    }
    d.money.extend(spend.money);
    d.changes
        .push(Delta::Gdp(GdpPart::GovServices, services + jg_wages));
    if !s.gov.pending_build.is_empty() {
        d.changes.push(Delta::Nature(nature));
    }

    // ---- Legitimacy ----
    let step = legitimacy_step(s, p, s.bank.bailout_this_tick);
    let l = step.raw;
    d.trace.push(TraceRecord::Legitimacy(step));
    d.changes.push(Delta::Gov(GovPatch {
        legitimacy: Some(Rate(l.clamp(0.0, 1.0))),
        tax_revenue: Some(receipts),
        receipts: Some(receipts),
        spending: Some(paid_total),
        unfunded_add: Some(shortfall),
        pending_build: Some(Vec::new()),
        ..GovPatch::default()
    }));
    d
}

/// The legitimacy update of one tick from the state's indicators (S8), term by term. The sum is
/// taken in the same order as the model always did (before, then wellbeing, unemployment,
/// inequality, debt, bailout, transition), so the numbers are unchanged. `bailout` is whether the
/// bank was bailed out this tick.
pub fn legitimacy_step(s: &State, p: &Params, bailout: bool) -> LegitStep {
    let gp = &p.government;
    let i = &s.indicators;
    let gdp_annual = i.gdp_annual.dollars().max(1.0);
    let debt_input = s.gov.debt.dollars() / gdp_annual;
    let (wellbeing_term, unemployment_term, inequality_term, debt_term) = legitimacy_terms(
        gp,
        i.avg_wellbeing.0,
        i.unemployment_smooth.0,
        i.gini.0,
        debt_input,
    );
    let before = s.gov.legitimacy.0;
    let mut l = before + wellbeing_term - unemployment_term - inequality_term - debt_term;
    let bailout_term = if bailout { gp.legit_bailout } else { 0.0 };
    if bailout {
        l -= gp.legit_bailout;
    }
    let mut transition_term = 0.0;
    if let Some(t) = &s.transition {
        transition_term = cfg(s, p).resistance.max(p.systems.get(t.to).resistance);
        l -= transition_term;
    }
    LegitStep {
        before,
        wellbeing_input: i.avg_wellbeing.0,
        wellbeing_term,
        unemployment_input: i.unemployment_smooth.0,
        unemployment_term: -unemployment_term,
        inequality_input: i.gini.0,
        inequality_term: -inequality_term,
        debt_input,
        debt_term: -debt_term,
        bailout_term: -bailout_term,
        transition_term: -transition_term,
        raw: l,
        after: l.clamp(0.0, 1.0),
    }
}

/// The four indicator terms of the legitimacy update, as magnitudes: `(wellbeing, unemployment,
/// inequality, debt)`. The first is signed (it adds above 0.5 and drains below); the others are
/// drains (zero below their threshold).
pub fn legitimacy_terms(
    gp: &sim_params::GovParams,
    wellbeing: f64,
    unemployment_smooth: f64,
    gini: f64,
    debt_to_gdp: f64,
) -> (f64, f64, f64, f64) {
    (
        gp.legit_wellbeing * (wellbeing - 0.5),
        gp.legit_unemployment * (unemployment_smooth - gp.legit_unemployment_threshold).max(0.0),
        gp.legit_gini * (gini - gp.legit_gini_threshold).max(0.0),
        gp.legit_debt * (debt_to_gdp - 1.0).max(0.0),
    )
}
