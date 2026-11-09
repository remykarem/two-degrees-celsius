//! S9 class and mobility. Class is computed from relationship to production, never assigned.
//! Workers with savings may found a firm (becoming capitalists); owners whose firm fails
//! and who are left with negative net wealth fall into the worker cohort.

use crate::government::labour_weights;
use crate::util::apportion;
use sim_metrics::cohort_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Does this cohort own productive assets: a firm (or a co-op it works in), housing or the bank?
pub fn owns_assets(s: &State, h: &Household) -> bool {
    s.bank.owner == h.id
        || s.housing.owner == h.id
        || s.firms.iter().any(|f| {
            f.active
                && match f.owner {
                    Owner::Household(o) => o == h.id,
                    Owner::Workers => h.employment[f.id.idx()].0 > 0.0,
                    Owner::State | Owner::Bank => false,
                }
        })
}

/// The class function (causal model S9), given an income history window.
pub fn classify_with(s: &State, p: &Params, h: &Household, hist: &[(Cents, Cents)]) -> Class {
    let capital: f64 = hist.iter().map(|x| x.0.as_f64()).sum();
    let gross: f64 = hist.iter().map(|x| x.1.as_f64()).sum();
    let capital_share = safe_div(capital, gross, 0.0);
    if owns_assets(s, h) && capital_share > p.class.capital_share_threshold {
        let cap_now = h.income.capital_income().as_f64();
        return if h.income.rent_received.as_f64() > 0.5 * cap_now {
            Class::Rentier
        } else {
            Class::Capitalist
        };
    }
    let lf = h.labour_force().0;
    let employed = h.employed().0;
    if lf > 0.0 && employed >= 0.5 * lf {
        // Average wage of an employed person, from the wages firms pay.
        let wage_bill: f64 = s
            .firms
            .iter()
            .map(|f| f.wage.dollars() * h.employment[f.id.idx()].0 * h.wage_premium.0)
            .sum();
        let wage_pc = wage_bill / employed.max(1e-9);
        if h.skill.0 > p.class.skill_threshold && wage_pc > p.class.skilled_wage_usd {
            Class::Manager
        } else {
            Class::Worker
        }
    } else {
        Class::Dependent
    }
}

/// Class as implied by the stored history (used by the invariant test).
pub fn classify(s: &State, p: &Params, h: &Household) -> Class {
    classify_with(s, p, h, &h.income_hist)
}

fn sector_return(s: &State, p: &Params, sector: Sector) -> Option<f64> {
    let kp = p.invest.capital_price_usd * 100.0;
    let v: Vec<f64> = s
        .firms
        .iter()
        .filter(|f| f.active && f.sector == sector && f.capital.0 > 0.0)
        .map(|f| 12.0 * f.profit.as_f64() / (f.capital.0 * kp))
        .collect();
    if v.is_empty() {
        None
    } else {
        Some(v.iter().sum::<f64>() / v.len() as f64)
    }
}

pub fn run(s: &State, p: &Params, r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let cp = &p.class;

    // ---- Class labels ----
    for h in &s.households {
        let mut hist = h.income_hist.clone();
        hist.push((h.income.capital_income(), h.income.gross_income()));
        if hist.len() > 6 {
            hist.remove(0);
        }
        let class = classify_with(s, p, h, &hist);
        if class != h.class {
            d.events.push(Event::ClassChanged { household: h.id, to: class });
        }
        d.household(
            h.id,
            HouseholdPatch {
                class: Some(class),
                push_income_hist: Some((h.income.capital_income(), h.income.gross_income())),
                ..HouseholdPatch::default()
            },
        );
    }

    // ---- Down: owners of firms that failed this tick, with negative net wealth ----
    let wealth = cohort_wealth(s, p);
    let mut touched: Vec<HouseholdId> = Vec::new();
    for f in s.firms.iter().filter(|f| f.closed_tick == Some(s.tick) && !f.active) {
        let Owner::Household(o) = f.owner else { continue };
        let h = &s.households[o.idx()];
        if touched.contains(&o) || wealth[o.idx()] >= 0.0 {
            continue;
        }
        let dest = s
            .households
            .iter()
            .filter(|x| x.class == Class::Worker && x.id != o && !touched.contains(&x.id))
            .max_by(|a, b| a.weight.0.partial_cmp(&b.weight.0).unwrap_or(std::cmp::Ordering::Equal));
        let Some(dest) = dest else { continue };
        let people = (cp.fall_fraction * h.weight.0).min(h.weight.0 - cp.min_cohort_weight);
        if people > 0.0 {
            move_people(&mut d, s, h, dest, people, false);
            touched.push(o);
            touched.push(dest.id);
        }
    }

    // ---- Up: workers or managers with savings found a firm in a dormant slot ----
    let base_rate = s.policy.base_rate.0 + p.credit.spread_base;
    let per_founder = cp.startup_cost_usd / cp.founders_per_firm + cp.startup_buffer_usd;
    let source = s
        .households
        .iter()
        .filter(|h| matches!(h.class, Class::Worker | Class::Manager) && !touched.contains(&h.id))
        .filter(|h| h.savings.dollars() / h.weight.0 > per_founder)
        .max_by(|a, b| {
            (a.savings.as_f64() / a.weight.0)
                .partial_cmp(&(b.savings.as_f64() / b.weight.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    let owner = s
        .households
        .iter()
        .filter(|h| h.class == Class::Capitalist && !touched.contains(&h.id))
        .max_by(|a, b| a.weight.0.partial_cmp(&b.weight.0).unwrap_or(std::cmp::Ordering::Equal));
    // Best dormant slot: the one whose sector has the highest return and a model firm to copy.
    let slot = s
        .firms
        .iter()
        .filter(|f| !f.active)
        .filter_map(|f| sector_return(s, p, f.sector).map(|ret| (f, ret)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if let (Some(src), Some(own), Some((slot, ret))) = (source, owner, slot) {
        let pairs = (ret - base_rate).max(0.0);
        let expected_founders = cp.mobility_rate * src.weight.0 * pairs;
        let prob = expected_founders / cp.founders_per_firm;
        let roll = r.next_f64();
        let startup = Cents::from_dollars(cp.startup_cost_usd);
        let founders = cp.founders_per_firm;
        let moved_savings = src.savings.scale(founders / src.weight.0);
        if roll < prob
            && src.weight.0 - founders >= cp.min_cohort_weight
            && own.savings + moved_savings >= startup
        {
            move_people(&mut d, s, src, own, founders, true);
            // Fund the firm: part cash, part construction of its first capital.
            let capital_part = startup.scale(cp.startup_capital_share);
            let cash_part = startup - capital_part;
            d.transfer(Account::Household(own.id), Account::Firm(slot.id), cash_part, FlowKind::Neutral);
            let lw = labour_weights(s);
            for (h, part) in apportion(capital_part, &lw).into_iter().enumerate() {
                d.transfer(
                    Account::Household(own.id),
                    Account::Household(HouseholdId(h as u32)),
                    part,
                    FlowKind::Wage,
                );
            }
            d.changes.push(Delta::Gdp(GdpPart::Investment, capital_part));
            // Copy prices, wages and technology from the sector's incumbents.
            let peers: Vec<&Firm> =
                s.firms.iter().filter(|f| f.active && f.sector == slot.sector).collect();
            let n = peers.len() as f64;
            let avg = |g: &dyn Fn(&Firm) -> f64| peers.iter().map(|f| g(f)).sum::<f64>() / n;
            let sales0 = 0.15 * avg(&|f| f.sales_hist.iter().sum::<f64>() / 3.0);
            d.firm(
                slot.id,
                FirmPatch {
                    active: Some(true),
                    owner: Some(Owner::Household(own.id)),
                    capital: Some(Units(capital_part.as_f64() / (p.invest.capital_price_usd * 100.0))),
                    wage: Some(Cents::from_f64(avg(&|f| f.wage.as_f64()))),
                    price: Some(Cents::from_f64(avg(&|f| f.price.as_f64()))),
                    inventory: Some(Units(sales0)),
                    tfp: Some(Rate(avg(&|f| f.tfp.0))),
                    green_tech: Some(Rate(avg(&|f| f.green_tech.0))),
                    sales_hist: Some([sales0; 3]),
                    loan_rate: Some(Rate(base_rate)),
                    closed_tick: Some(None),
                    distress_months: Some(0),
                    over_target_months: Some(0),
                    ..FirmPatch::default()
                },
            );
            d.events.push(Event::FirmFounded { firm: slot.id });
        }
    }
    d
}

/// Move `people` from one cohort to another, with their savings and debt per head.
/// Their jobs go with them when `scale_jobs` is set (workers who leave to found firms).
fn move_people(d: &mut Deltas, s: &State, from: &Household, to: &Household, people: f64, scale_jobs: bool) {
    let share = people / from.weight.0;
    d.transfer(
        Account::Household(from.id),
        Account::Household(to.id),
        from.savings.scale(share),
        FlowKind::Neutral,
    );
    d.money.push(MoneyOp::MoveDebt { from: from.id, to: to.id, amount: from.debt.scale(share) });
    d.household(from.id, HouseholdPatch { weight: Some(from.weight - People(people)), ..HouseholdPatch::default() });
    d.household(to.id, HouseholdPatch { weight: Some(to.weight + People(people)), ..HouseholdPatch::default() });
    if scale_jobs {
        for f in &s.firms {
            let e = from.employment[f.id.idx()].0;
            if e > 0.0 {
                d.changes.push(Delta::Employment { h: from.id, f: f.id, n: People(e * (1.0 - share)) });
            }
        }
    }
}
