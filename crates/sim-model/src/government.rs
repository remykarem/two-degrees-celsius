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
        let wage_tax = h.income.wages.scale(s.policy.tax_wage.0);
        let cap_tax = h.income.capital_income().scale(s.policy.tax_capital.0);
        // The wealth tax falls on a household's net worth above the threshold.
        let over = (wealth[i] / 100.0 - s.policy.wealth_threshold.dollars()).max(0.0);
        let wealth_tax = Cents::from_dollars(over * s.effective.wealth_tax.0 / 12.0);
        let mut left = h.savings.max(Cents::ZERO);
        for amt in [wage_tax, cap_tax, wealth_tax] {
            let paid = amt.min(left);
            left -= paid;
            taxes += paid;
            d.transfer(me, gov, paid, FlowKind::Tax);
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
    let mut pay: Vec<(Account, Cents, FlowKind)> = Vec::new();
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
        pay.push((me, b + u, FlowKind::Benefit));
        pay.push((me, j, FlowKind::Wage));
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
        let max_loans = s.bank.equity.max(Cents::ZERO).scale(1.0 / kappa);
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
    for (to, amt, kind) in pay {
        let a = amt.scale(ratio).min(amt);
        paid_total += a;
        spend.transfer(gov, to, a, kind);
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
    let i = &s.indicators;
    let gdp_annual = i.gdp_annual.dollars().max(1.0);
    let mut l = s.gov.legitimacy.0 + gp.legit_wellbeing * (i.avg_wellbeing.0 - 0.5)
        - gp.legit_unemployment
            * (i.unemployment_smooth.0 - gp.legit_unemployment_threshold).max(0.0)
        - gp.legit_gini * (i.gini.0 - gp.legit_gini_threshold).max(0.0)
        - gp.legit_debt * (s.gov.debt.dollars() / gdp_annual - 1.0).max(0.0);
    if s.bank.bailout_this_tick {
        l -= gp.legit_bailout;
    }
    if let Some(t) = &s.transition {
        l -= cfg(s, p).resistance.max(p.systems.get(t.to).resistance);
    }
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
