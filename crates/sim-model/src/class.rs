//! S9 class and mobility. Class is computed per household from its relationship to production,
//! never assigned. A household with enough savings may found a firm in the dormant slot
//! (becoming an owner); a household whose firm fails simply loses that ownership, and the
//! derived class follows.

use crate::util::{apportion, labour_weights, person_pay};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::QuantityId as Q;
use sim_types::*;

/// Does this household own productive assets: a firm (or a co-op it works in), the bank, or a
/// home it rents out to someone else?
pub fn owns_assets(s: &State, h: &Household) -> bool {
    s.bank.owner == h.id
        || s.homes
            .iter()
            .any(|x| x.owner == HomeOwner::Household(h.id) && x.household != Some(h.id))
        || s.firms.iter().any(|f| {
            f.active
                && match f.owner {
                    Owner::Household(o) => o == h.id,
                    Owner::Workers => h.members.iter().any(|m| s.person(*m).job == Some(f.id)),
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
        return if h.income.rent_received.as_f64() > p.class.rentier_share_threshold * cap_now {
            Class::Rentier
        } else {
            Class::Capitalist
        };
    }
    let working: Vec<&Person> = h
        .members
        .iter()
        .map(|m| s.person(*m))
        .filter(|x| x.employed())
        .collect();
    if working.is_empty() {
        return Class::Dependent;
    }
    // Average pay of the household's workers, against the going base wage.
    let min_wage = s.policy.min_wage.dollars();
    let pay: f64 = working
        .iter()
        .map(|x| match x.job {
            Some(f) => person_pay(s, p, x, s.firms[f.idx()].wage).dollars(),
            // Domestic staff: what they were actually paid this month.
            None if x.staff_of.is_some() => x.wage.dollars(),
            None => min_wage,
        })
        .sum::<f64>()
        / working.len() as f64;
    let (num, den) = s
        .firms
        .iter()
        .filter(|f| f.active)
        .fold((0.0, 0.0), |(n, d), f| {
            (n + f.wage.dollars() * f.workers.0, d + f.workers.0)
        });
    let base_wage = safe_div(num, den, 0.0);
    let top_skill = working.iter().map(|x| x.skill.0).fold(0.0, f64::max);
    // Skilled means paid well above the going base wage, so the test follows the wage level.
    if top_skill > p.class.skill_threshold && pay > p.class.skilled_wage_ratio * base_wage {
        Class::Manager
    } else {
        Class::Worker
    }
}

/// Class as implied by the stored history (used by the invariant test).
pub fn classify(s: &State, p: &Params, h: &Household) -> Class {
    classify_with(s, p, h, &h.income_hist)
}

/// Average return on capital of a sector's active firms, if demand there exceeds supply enough
/// to leave room for another firm.
pub fn sector_return(s: &State, p: &Params, sector: Sector) -> Option<f64> {
    let kp = p.invest.capital_price_usd * 100.0;
    let firms: Vec<&Firm> = s
        .firms
        .iter()
        .filter(|f| f.active && f.sector == sector && f.capital.0 > 0.0)
        .collect();
    if firms.is_empty() {
        return None;
    }
    let n = firms.len() as f64;
    let excess = firms.iter().map(|f| f.excess_demand.0).sum::<f64>() / n;
    if excess < p.class.startup_min_excess_demand {
        return None;
    }
    Some(
        firms
            .iter()
            .map(|f| crate::util::MONTHS_PER_YEAR * f.profit.as_f64() / (f.capital.0 * kp))
            .sum::<f64>()
            / n,
    )
}

// ---- Decision: a household founds a firm ----

pub const HOUSEHOLD_FOUND_FIRM: DecisionSpec = DecisionSpec {
    id: D::HouseholdFoundFirm,
    system: SystemId::Class,
    inputs: &[Q::SectorReturn, Q::BaseRate, Q::RandomDraw],
    outputs: &[Q::NewFirm],
    params: &["class.mobility_rate"],
    rule: "return_over_loan_rate",
};

/// Who may found (a Worker or Manager household with the savings) and which slot are matching,
/// done in `run`; this is the go or no-go for that household and slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoundView {
    /// Average return on capital in the slot's sector.
    pub sector_return: f64,
    /// What borrowing costs: the base rate plus the bank's base spread.
    pub base_rate: f64,
    /// This month's uniform draw in [0, 1), taken by `run` whether or not a firm can be founded.
    pub random_draw: f64,
}

/// The view of a founding decision for a slot in `sector` with this month's draw.
pub fn found_view(s: &State, p: &Params, sector: Sector, random_draw: f64) -> Option<FoundView> {
    Some(FoundView {
        sector_return: sector_return(s, p, sector)?,
        base_rate: s.policy.base_rate.0 + p.credit.spread_base,
        random_draw,
    })
}

pub fn decide_found(v: &FoundView, cp: &sim_params::ClassParams) -> bool {
    let prob = (cp.mobility_rate * (v.sector_return - v.base_rate).max(0.0)).clamp(0.0, 1.0);
    v.random_draw < prob
}

pub fn run(s: &State, p: &Params, r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let cp = &p.class;

    // ---- Class labels ----
    for h in &s.households {
        let mut hist = h.income_hist.clone();
        hist.push((h.income.capital_income(), h.income.gross_income()));
        if hist.len() > cp.history_months {
            hist.remove(0);
        }
        let class = classify_with(s, p, h, &hist);
        if class != h.class {
            d.events.push(Event::ClassChanged {
                household: h.id,
                to: class,
            });
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

    // ---- Up: a worker or manager household with savings founds a firm in a dormant slot ----
    let base_rate = s.policy.base_rate.0 + p.credit.spread_base;
    let need = Cents::from_dollars(cp.startup_cost_usd + cp.startup_buffer_usd);
    let source = s
        .households
        .iter()
        .filter(|h| matches!(h.class, Class::Worker | Class::Manager) && h.savings >= need)
        .max_by_key(|h| (h.savings, std::cmp::Reverse(h.id)));
    // Best dormant slot: the one whose sector has the highest return and a model firm to copy.
    let slot = s
        .firms
        .iter()
        .filter(|f| !f.active)
        .filter_map(|f| sector_return(s, p, f.sector).map(|ret| (f, ret)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    // The site: the slot's reserved tile if it is still empty, else the first empty tile.
    let site = |f: &Firm| {
        if s.map.kind_at(f.tile) == Some(TileKind::Empty) {
            Some(f.tile)
        } else {
            s.map
                .tiles()
                .find(|t| s.map.kind_at(*t) == Some(TileKind::Empty))
        }
    };
    // The draw is always taken (like the estate's), so whether a firm could be founded this month
    // never shifts the shared stream for the draws that follow.
    let roll = r.next_f64();
    if let (Some(src), Some((slot, ret))) = (source, slot) {
        let found = decide_found(
            &FoundView {
                sector_return: ret,
                base_rate,
                random_draw: roll,
            },
            cp,
        );
        let startup = Cents::from_dollars(cp.startup_cost_usd);
        if let (true, Some(tile)) = (found, site(slot)) {
            // Fund the firm: part cash, part construction of its first capital.
            let capital_part = startup.scale(cp.startup_capital_share);
            let cash_part = startup - capital_part;
            let me = Account::Household(src.id);
            d.transfer_as(
                me,
                Account::Firm(slot.id),
                cash_part,
                FlowKind::Neutral,
                Purpose::StartupFunding,
            );
            for (h, part) in apportion(capital_part, &labour_weights(s))
                .into_iter()
                .enumerate()
            {
                d.transfer_as(
                    me,
                    Account::Household(HouseholdId(h as u32)),
                    part,
                    FlowKind::Wage,
                    Purpose::StartupWorks,
                );
            }
            d.changes
                .push(Delta::Gdp(GdpPart::Investment, capital_part));
            // Copy prices, wages and technology from the sector's incumbents.
            let peers: Vec<&Firm> = s
                .firms
                .iter()
                .filter(|f| f.active && f.sector == slot.sector)
                .collect();
            let n = peers.len() as f64;
            let avg = |g: &dyn Fn(&Firm) -> f64| peers.iter().map(|f| g(f)).sum::<f64>() / n;
            let sales0 = p.class.new_firm_sales_share * avg(&|f| f.expected_sales.0);
            // The ruin of an earlier firm in this slot is cleared before the new one is built.
            if tile != slot.tile && s.map.kind_at(slot.tile) == Some(TileKind::Firm) {
                d.changes.push(Delta::SetTile(slot.tile, MapTile::EMPTY));
            }
            d.changes.push(Delta::SetTile(
                tile,
                MapTile {
                    kind: TileKind::Firm,
                    id: Some(slot.id.0),
                },
            ));
            d.firm(
                slot.id,
                FirmPatch {
                    active: Some(true),
                    tile: Some(tile),
                    owner: Some(Owner::Household(src.id)),
                    capital: Some(Units(
                        capital_part.as_f64() / (p.invest.capital_price_usd * 100.0),
                    )),
                    wage: Some(Cents::from_f64(avg(&|f| f.wage.as_f64()))),
                    price: Some(Cents::from_f64(avg(&|f| f.price.as_f64()))),
                    inventory: Some(Units(sales0)),
                    tfp: Some(Rate(avg(&|f| f.tfp.0))),
                    green_tech: Some(Rate(avg(&|f| f.green_tech.0))),
                    sales_hist: Some([sales0; 3]),
                    expected_sales: Some(Units(sales0)),
                    markup: Some(Rate(avg(&|f| f.markup.0))),
                    capital_per_sale: Some(avg(&|f| f.capital_per_sale)),
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
