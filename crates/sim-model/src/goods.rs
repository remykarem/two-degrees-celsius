//! S3 goods and housing market: households buy food (a need) and goods (discretionary),
//! prices adjust to excess demand, rents adjust to occupancy.

use crate::util::apportion;
use libm::pow;
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
fn allocate(firms: &[&Firm], budget: f64, elasticity: f64) -> Allocation {
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
    let prices: Vec<f64> = firms.iter().map(|f| f.price.as_f64().max(1.0)).collect();
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
        let wish = c * (disposable - need_cost).max(0.0) + gp.wealth_consumption_rate * savings;
        goods_budget[i] = wish.min(savings - food_budget[i]).max(0.0);
    }
    let gov_budget = s
        .gov
        .purchases_budget
        .min(s.gov.cash)
        .max(Cents::ZERO)
        .as_f64();

    // ---- Markets ----
    let mut parts_food_units = vec![0.0; n_h];
    let mut consumption = vec![Cents::ZERO; n_h];
    for sector in [Sector::Food, Sector::Goods] {
        let firms: Vec<&Firm> = s
            .firms
            .iter()
            .filter(|f| f.active && f.sector == sector)
            .collect();
        if firms.is_empty() {
            continue;
        }
        let (hb, gb): (&Vec<f64>, f64) = match sector {
            Sector::Food => (&food_budget, 0.0),
            Sector::Goods => (&goods_budget, gov_budget),
        };
        let mut buyers: Vec<f64> = hb.clone();
        buyers.push(gb);
        let budget: f64 = buyers.iter().sum();
        let alloc = allocate(&firms, budget, gp.price_elasticity);
        for (i, f) in firms.iter().enumerate() {
            let price = f.price.as_f64().max(1.0);
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
                } else {
                    d.transfer(
                        Account::Government,
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                    );
                }
            }
            // Price signal: demand against the month's supply.
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
            let adj = (gp.price_adjust * excess).clamp(-gp.price_band, gp.price_band);
            let new_price = Cents::from_f64(price * (1.0 + adj)).max(Cents(1));
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
                    ..FirmPatch::default()
                },
            );
        }
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
