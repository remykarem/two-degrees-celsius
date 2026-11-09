//! S3 goods and housing market: households buy food (a need) and goods (discretionary),
//! prices adjust to excess demand, rents adjust to occupancy.

use crate::util::{apportion, capex_available, paid_workers};
use libm::pow;
use sim_metrics::cohort_wealth;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

struct Allocation {
    sold: Vec<f64>,
    spent: Vec<f64>,
    demand_units: Vec<f64>,
}

/// Split a sector's total budget (cents) over its firms. Cheaper firms with more stock
/// get more; stock limits are respected over two passes.
fn allocate(firms: &[&Firm], budget: f64, elasticity: f64, min_price: f64) -> Allocation {
    let n = firms.len();
    let mut a = Allocation {
        sold: vec![0.0; n],
        spent: vec![0.0; n],
        demand_units: vec![0.0; n],
    };
    if n == 0 || budget <= 0.0 {
        return a;
    }
    let avail0: Vec<f64> = firms.iter().map(|f| f.inventory.0).collect();
    let prices: Vec<f64> = firms
        .iter()
        .map(|f| f.price.as_f64().max(min_price))
        .collect();
    let tot_avail: f64 = avail0.iter().sum();
    let pbar = if tot_avail > 0.0 {
        avail0.iter().zip(&prices).map(|(q, p)| q * p).sum::<f64>() / tot_avail
    } else {
        prices.iter().sum::<f64>() / n as f64
    };
    let mut remaining_budget = budget;
    for pass in 0..2 {
        let avail: Vec<f64> = (0..n).map(|i| (avail0[i] - a.sold[i]).max(0.0)).collect();
        let w: Vec<f64> = (0..n)
            .map(|i| {
                let base = if pass == 0 && tot_avail <= 0.0 {
                    1.0
                } else {
                    avail[i]
                };
                base * pow(pbar / prices[i], elasticity)
            })
            .collect();
        let wsum: f64 = w.iter().sum();
        if wsum <= 0.0 {
            break;
        }
        let pass_budget = remaining_budget;
        let mut used = 0.0;
        for i in 0..n {
            let share = pass_budget * w[i] / wsum;
            if pass == 0 {
                a.demand_units[i] = share / prices[i];
            }
            let units = (share / prices[i]).min(avail[i]);
            a.sold[i] += units;
            a.spent[i] += units * prices[i];
            used += units * prices[i];
        }
        remaining_budget -= used;
        if remaining_budget < 1.0 {
            break;
        }
    }
    a
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let gp = &p.goods;
    let n_h = s.households.len();

    // ---- Housing ----
    let renters: Vec<f64> = s
        .households
        .iter()
        .map(|h| h.weight.0 * h.renter_share.0)
        .collect();
    let renters_total: f64 = renters.iter().sum();
    let rental_cap = s.housing.private_cap.0 + s.housing.public_cap.0;
    let occupancy = safe_div(
        renters_total,
        rental_cap,
        if renters_total > 0.0 { 9.99 } else { 0.0 },
    );
    let unhoused = if occupancy > 1.0 {
        1.0 - 1.0 / occupancy
    } else {
        0.0
    };
    let rent_f = s.housing.rent_private.as_f64();
    let rent_new = if occupancy > gp.occupancy_trigger {
        rent_f * (1.0 + gp.rent_up)
    } else {
        rent_f * (1.0 - gp.rent_down)
    };
    let min_rent = Cents::from_dollars(gp.min_rent_usd);
    let mut housing = s.housing.clone();
    housing.rent_private = Cents::from_f64(rent_new).max(min_rent);
    housing.private_cap = Units(s.housing.private_cap.0 * (1.0 - gp.housing_depreciation));
    housing.occupancy = Rate(occupancy);
    housing.housed_share = Rate(1.0 - unhoused * safe_div(renters_total, s.total_people().0, 0.0));
    d.changes.push(Delta::Housing(housing));

    // ---- Prices and budgets ----
    let avg_price = |sector: Sector| -> f64 {
        let fs: Vec<&Firm> = s
            .firms
            .iter()
            .filter(|f| f.active && f.sector == sector)
            .collect();
        let q: f64 = fs.iter().map(|f| f.inventory.0).sum();
        if q > 0.0 {
            fs.iter()
                .map(|f| f.inventory.0 * f.price.as_f64())
                .sum::<f64>()
                / q
        } else if !fs.is_empty() {
            fs.iter().map(|f| f.price.as_f64()).sum::<f64>() / fs.len() as f64
        } else {
            0.0
        }
    };
    let p_food = avg_price(Sector::Food);
    let wealth = cohort_wealth(s, p);
    let mut food_budget = vec![0.0; n_h];
    let mut goods_budget = vec![0.0; n_h];
    let mut need_units = vec![0.0; n_h];
    for (i, h) in s.households.iter().enumerate() {
        need_units[i] = h.weight.0 * gp.food_need_units_pc;
        let need_cost = need_units[i] * p_food;
        let savings = h.savings.as_f64();
        food_budget[i] = need_cost.min(savings);
        let c = gp.propensity.get(h.class);
        let disposable = h.last_disposable.as_f64().max(0.0);
        // Wealth effect: capital-owning classes also consume out of net wealth.
        let wealth_spend = if matches!(h.class, Class::Capitalist | Class::Rentier) {
            gp.wealth_consumption_rate * wealth[i].max(0.0)
        } else {
            0.0
        };
        let wish = c * (disposable - need_cost).max(0.0) + wealth_spend;
        goods_budget[i] = wish.min(savings - food_budget[i]).max(0.0);
    }
    // Firms buy capital goods from the goods sector (investment is demand).
    let capex_budget: Vec<f64> = s
        .firms
        .iter()
        .map(|f| {
            if f.active {
                capex_available(s, p, f).min(f.plan_invest).as_f64()
            } else {
                0.0
            }
        })
        .collect();
    // Government purchases are the player's G lever, funded in S8; any part the treasury cannot
    // fund is reported, never silently dropped.
    let wanted = s.policy.public_spending.max(Cents::ZERO);
    let gov_budget_c = wanted.min(s.gov.cash.max(Cents::ZERO));
    let gov_budget = gov_budget_c.as_f64();
    let unfunded = wanted - gov_budget_c;
    if unfunded > Cents::ZERO {
        d.events.push(Event::SpendingCut {
            shortfall: unfunded,
        });
    }
    d.changes.push(Delta::Gov(GovPatch {
        unfunded: Some(unfunded),
        unfunded_add: Some(unfunded),
        ..GovPatch::default()
    }));

    // ---- Markets ----
    let n_f = s.firms.len();
    let mut parts_food_units = vec![0.0; n_h];
    let mut consumption = vec![Cents::ZERO; n_h];
    let mut capex_paid = vec![Cents::ZERO; n_f];
    let min_price = gp.min_price_usd * 100.0;
    for sector in [Sector::Food, Sector::Goods] {
        let firms: Vec<&Firm> = s
            .firms
            .iter()
            .filter(|f| f.active && f.sector == sector)
            .collect();
        if firms.is_empty() {
            continue;
        }
        // Buyers in order: household cohorts, firms (capital goods), government.
        let mut buyers: Vec<f64> = Vec::with_capacity(n_h + n_f + 1);
        match sector {
            Sector::Food => {
                buyers.extend(&food_budget);
                buyers.extend(std::iter::repeat_n(0.0, n_f + 1));
            }
            Sector::Goods => {
                buyers.extend(&goods_budget);
                buyers.extend(&capex_budget);
                buyers.push(gov_budget);
            }
        }
        let budget: f64 = buyers.iter().sum();
        let alloc = allocate(&firms, budget, gp.price_elasticity, min_price);
        for (i, f) in firms.iter().enumerate() {
            let price = f.price.as_f64().max(min_price);
            let revenue = Cents(alloc.spent[i].floor() as i64);
            let sold_units = alloc.sold[i];
            let parts = apportion(revenue, &buyers);
            for (b, part) in parts.iter().enumerate() {
                if b < n_h {
                    d.transfer(
                        Account::Household(HouseholdId(b as u32)),
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                    );
                    consumption[b] += *part;
                    if sector == Sector::Food {
                        parts_food_units[b] += part.as_f64() / price;
                    }
                } else if b < n_h + n_f {
                    let buyer = FirmId((b - n_h) as u32);
                    d.transfer(
                        Account::Firm(buyer),
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                    );
                    capex_paid[buyer.idx()] += *part;
                } else {
                    d.transfer(
                        Account::Government,
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                    );
                }
            }
            // Price follows unit cost plus a markup, closing a share of the gap each month.
            let wage_bill = f.wage.as_f64() * paid_workers(s, f);
            let unit_cost = safe_div(wage_bill + f.carbon_cost.as_f64(), f.output.0, 0.0);
            let target = (1.0 + f.markup.0) * unit_cost;
            let new_price = if unit_cost > 0.0 {
                Cents::from_f64(price + gp.price_adjust * (target - price))
                    .max(Cents::from_f64(min_price))
            } else {
                f.price
            };
            let supply = if f.output.0 > 0.0 {
                f.output.0
            } else {
                f.inventory.0
            };
            let excess = if supply > 0.0 {
                (alloc.demand_units[i] - supply) / supply
            } else {
                1.0
            };
            let lam = p.labour.expected_smoothing;
            let expected = lam * alloc.demand_units[i] + (1.0 - lam) * f.expected_sales.0;
            let h = f.sales_hist;
            d.firm(
                f.id,
                FirmPatch {
                    inventory: Some(Units((f.inventory.0 - sold_units).max(0.0))),
                    price: Some(new_price),
                    sales_hist: Some([sold_units, h[0], h[1]]),
                    sales_units: Some(Units(sold_units)),
                    revenue: Some(revenue),
                    excess_demand: Some(Rate(excess)),
                    expected_sales: Some(Units(expected)),
                    ..FirmPatch::default()
                },
            );
        }
    }
    for f in s.firms.iter().filter(|f| f.active) {
        d.firm(
            f.id,
            FirmPatch {
                capex: Some(capex_paid[f.id.idx()]),
                ..FirmPatch::default()
            },
        );
    }
    // Inactive firms record zero sales.
    for f in s.firms.iter().filter(|f| !f.active) {
        d.firm(
            f.id,
            FirmPatch {
                revenue: Some(Cents::ZERO),
                sales_units: Some(Units::ZERO),
                ..FirmPatch::default()
            },
        );
    }
    for (i, h) in s.households.iter().enumerate() {
        let met = if need_units[i] > 0.0 {
            (parts_food_units[i] / need_units[i]).clamp(0.0, 1.0)
        } else {
            1.0
        };
        d.household(
            h.id,
            HouseholdPatch {
                food_met: Some(Rate(met)),
                consumption: Some(consumption[i]),
                housed_share: Some(Rate(1.0 - h.renter_share.0 * unhoused)),
                ..HouseholdPatch::default()
            },
        );
    }
    d
}
