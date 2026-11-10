//! S3 goods and housing. Households buy food (a need), goods and services (discretionary);
//! prices follow unit cost and excess demand. Housing is by home: rents follow occupancy and
//! affordability, a household that cannot pay rent for months loses its home, and a household
//! without a home moves into a vacant one it can afford.

use crate::util::{apportion, capex_available, wage_bill};
use libm::pow;
use sim_metrics::household_wealth;
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

/// Evictions, moves and rents (monthly). Returns the events and deltas for housing.
fn housing(s: &State, p: &Params, d: &mut Deltas) {
    let gp = &p.goods;
    // Working copy of who lives where.
    let mut occupant: Vec<(HomeId, Option<HouseholdId>)> =
        s.homes.iter().map(|h| (h.id, h.household)).collect();
    let mut home_of: Vec<Option<HomeId>> = s.households.iter().map(|h| h.home).collect();
    let mut evicted_now: Vec<HouseholdId> = Vec::new();

    // Evictions: unpaid rent for too long loses the home. Owner-occupiers cannot be evicted.
    for h in &s.households {
        let Some(hid) = h.home else { continue };
        let Some(home) = s.home(hid) else { continue };
        let rented = home.owner != HomeOwner::Household(h.id);
        if rented && h.arrears_months >= gp.evict_after_months {
            if let Some(o) = occupant.iter_mut().find(|o| o.0 == hid) {
                o.1 = None;
            }
            home_of[h.id.idx()] = None;
            evicted_now.push(h.id);
            d.changes.push(Delta::Home(
                hid,
                HomePatch {
                    household: Some(None),
                    ..HomePatch::default()
                },
            ));
            d.household(
                h.id,
                HouseholdPatch {
                    home: Some(None),
                    arrears_months: Some(0),
                    ..HouseholdPatch::default()
                },
            );
            d.events.push(Event::HouseholdEvicted { household: h.id });
        }
    }

    // Moves: households without a home, richest first, take the cheapest vacant home whose
    // rent is within their means. A household evicted this month does not move at once.
    let mut seekers: Vec<&Household> = s
        .households
        .iter()
        .filter(|h| home_of[h.id.idx()].is_none() && !evicted_now.contains(&h.id))
        .collect();
    seekers.sort_by(|a, b| {
        b.last_disposable
            .cmp(&a.last_disposable)
            .then(a.id.cmp(&b.id))
    });
    for h in seekers {
        let afford = h.last_disposable.scale(gp.move_rent_share);
        let mut best: Option<(&Home, Cents)> = None;
        for home in &s.homes {
            if !occupant.iter().any(|o| o.0 == home.id && o.1.is_none()) {
                continue;
            }
            let rent = if home.owner == HomeOwner::Household(h.id) {
                Cents::ZERO
            } else {
                home.rent
            };
            if rent > afford {
                continue;
            }
            if best.is_none_or(|(b, br)| rent < br || (rent == br && home.id < b.id)) {
                best = Some((home, rent));
            }
        }
        if let Some((home, _)) = best {
            if let Some(o) = occupant.iter_mut().find(|o| o.0 == home.id) {
                o.1 = Some(h.id);
            }
            home_of[h.id.idx()] = Some(home.id);
            d.changes.push(Delta::Home(
                home.id,
                HomePatch {
                    household: Some(Some(h.id)),
                    ..HomePatch::default()
                },
            ));
            d.household(
                h.id,
                HouseholdPatch {
                    home: Some(Some(home.id)),
                    arrears_months: Some(0),
                    ..HouseholdPatch::default()
                },
            );
            d.events.push(Event::HouseholdMoved {
                household: h.id,
                home: home.id,
            });
        }
    }

    // Rents: rise while every home is taken and renters can bear it, fall otherwise.
    let occupied = occupant.iter().filter(|o| o.1.is_some()).count();
    let occupancy = safe_div(occupied as f64, s.homes.len() as f64, 1.0);
    let burdens: Vec<f64> = s
        .homes
        .iter()
        .filter(|h| matches!(h.owner, HomeOwner::Household(o) if Some(o) != h.household))
        .filter_map(|h| h.household)
        .map(|hh| {
            safe_div(
                s.housing.rent_private.as_f64(),
                s.households[hh.idx()].last_disposable.as_f64(),
                1.0,
            )
        })
        .collect();
    let burden = if burdens.is_empty() {
        0.0
    } else {
        burdens.iter().sum::<f64>() / burdens.len() as f64
    };
    let rent_f = s.housing.rent_private.as_f64();
    let rent_new = if occupancy >= gp.occupancy_trigger && burden < gp.rent_burden_cap {
        rent_f * (1.0 + gp.rent_up)
    } else {
        rent_f * (1.0 - gp.rent_down)
    };
    let min_rent = Cents::from_dollars(gp.min_rent_usd);
    let rent_private = Cents::from_f64(rent_new).max(min_rent);
    let mut housing = s.housing.clone();
    housing.rent_private = rent_private;
    housing.occupancy = Rate(occupancy);
    housing.unhoused_households = home_of.iter().filter(|h| h.is_none()).count() as u32;
    d.changes.push(Delta::Housing(housing));
    for home in &s.homes {
        let rent = match home.owner {
            HomeOwner::Public => s.housing.rent_public,
            HomeOwner::Household(_) => rent_private,
        };
        if rent != home.rent {
            d.changes.push(Delta::Home(
                home.id,
                HomePatch {
                    rent: Some(rent),
                    ..HomePatch::default()
                },
            ));
        }
    }
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let gp = &p.goods;
    let n_h = s.households.len();

    housing(s, p, &mut d);

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
    let wealth = household_wealth(s, p);
    let mut food_budget = vec![0.0; n_h];
    let mut goods_budget = vec![0.0; n_h];
    let mut services_budget = vec![0.0; n_h];
    let mut need_units = vec![0.0; n_h];
    for (i, h) in s.households.iter().enumerate() {
        need_units[i] = h.members.len() as f64 * gp.food_need_units_pc;
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
        let discretionary = wish.min(savings - food_budget[i]).max(0.0);
        services_budget[i] = discretionary * gp.services_share;
        goods_budget[i] = discretionary - services_budget[i];
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
    // fund is reported, never silently dropped. They go to goods and services.
    let wanted = s.policy.public_spending.max(Cents::ZERO);
    let gov_budget_c = wanted.min(s.gov.cash.max(Cents::ZERO));
    let gov_budget = gov_budget_c.as_f64();
    let gov_services = gov_budget * gp.gov_services_share;
    let gov_goods = gov_budget - gov_services;
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
    for sector in [Sector::Food, Sector::Goods, Sector::Services] {
        let firms: Vec<&Firm> = s
            .firms
            .iter()
            .filter(|f| f.active && f.sector == sector)
            .collect();
        if firms.is_empty() {
            continue;
        }
        // Buyers in order: households, firms (capital goods), government.
        let mut buyers: Vec<f64> = Vec::with_capacity(n_h + n_f + 1);
        match sector {
            Sector::Food => {
                buyers.extend(&food_budget);
                buyers.extend(std::iter::repeat_n(0.0, n_f + 1));
            }
            Sector::Goods => {
                buyers.extend(&goods_budget);
                buyers.extend(&capex_budget);
                buyers.push(gov_goods);
            }
            Sector::Services => {
                buyers.extend(&services_budget);
                buyers.extend(std::iter::repeat_n(0.0, n_f));
                buyers.push(gov_services);
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
                    let hid = HouseholdId(b as u32);
                    d.transfer_as(
                        Account::Household(hid),
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                        Purpose::Purchase(sector),
                    );
                    if *part > Cents::ZERO {
                        d.events.push(Event::Purchase {
                            household: hid,
                            firm: f.id,
                            amount: *part,
                        });
                    }
                    consumption[b] += *part;
                    if sector == Sector::Food {
                        parts_food_units[b] += part.as_f64() / price;
                    }
                } else if b < n_h + n_f {
                    let buyer = FirmId((b - n_h) as u32);
                    d.transfer_as(
                        Account::Firm(buyer),
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                        Purpose::CapitalGoods,
                    );
                    capex_paid[buyer.idx()] += *part;
                } else {
                    d.transfer_as(
                        Account::Government,
                        Account::Firm(f.id),
                        *part,
                        FlowKind::Purchase,
                        Purpose::PublicPurchases,
                    );
                }
            }
            // Price follows unit cost plus a markup, closing a share of the gap each month.
            let unit_cost = safe_div(
                (wage_bill(s, p, f) + f.carbon_cost).as_f64(),
                f.output.0,
                0.0,
            );
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
                ..HouseholdPatch::default()
            },
        );
    }
    d
}
