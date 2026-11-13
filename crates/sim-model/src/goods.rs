//! S3 goods and housing. Households buy food (a need), goods and services (discretionary).
//! Prices are cost-plus: each firm closes a share of the gap to `(1 + markup) x unit cost` every
//! month. Excess demand is stored but feeds only `expected_sales` (so hiring and production),
//! never the price. Housing is by home: rents follow occupancy and
//! affordability, a household that cannot pay rent for months loses its home, and a household
//! without a home moves into a vacant one it can afford.

use crate::util::{apportion, capex_available, wage_bill};
use libm::pow;
use sim_metrics::household_wealth;
use sim_params::{GoodsParams, LabourParams, Params};
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::Effect as Fx;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;

// ---- Decision: where a household's (and any buyer's) money goes among the firms of a market ----

pub const HOUSEHOLD_SHOP: DecisionSpec = DecisionSpec {
    id: D::HouseholdShop,
    system: SystemId::Goods,
    inputs: &[Q::ConsumptionBudget, Q::Inventory, Q::Price],
    outputs: &[Q::UnitsSold, Q::DemandUnits],
    effects: &[
        Fx::up(Q::ConsumptionBudget, Q::UnitsSold),
        Fx::up(Q::ConsumptionBudget, Q::DemandUnits),
        Fx::up(Q::Inventory, Q::UnitsSold),
        Fx::up(Q::Inventory, Q::DemandUnits),
        Fx::down(Q::Price, Q::UnitsSold),
        Fx::down(Q::Price, Q::DemandUnits),
    ],
    params: &["goods.price_elasticity", "goods.min_price_usd"],
    rule: "stock_and_price_weighted",
};

/// What buyers see in one market: the money they bring (households, firms buying capital goods
/// and the government all use this split) and each seller's stock and price (cents), in the same
/// order. The slices are borrowed from `ShopInputs`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShopView<'a> {
    pub budget: f64,
    pub stock: &'a [f64],
    pub price: &'a [f64],
}

/// The storage a `ShopView` borrows: one entry per active firm of the sector.
pub struct ShopInputs {
    pub stock: Vec<f64>,
    pub price: Vec<f64>,
}

pub fn shop_inputs(firms: &[&Firm]) -> ShopInputs {
    ShopInputs {
        stock: firms.iter().map(|f| f.inventory.0).collect(),
        price: firms.iter().map(|f| f.price.as_f64()).collect(),
    }
}

impl ShopInputs {
    pub fn view(&self, budget: f64) -> ShopView<'_> {
        ShopView {
            budget,
            stock: &self.stock,
            price: &self.price,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopChoice {
    pub sold: Vec<f64>,
    pub spent: Vec<f64>,
    pub demand_units: Vec<f64>,
}

/// Split a sector's total budget (cents) over its firms. Cheaper firms with more stock
/// get more; stock limits are respected over two passes.
pub fn decide_shop(v: &ShopView, gp: &GoodsParams) -> ShopChoice {
    let elasticity = gp.price_elasticity;
    let min_price = gp.min_price_usd * 100.0;
    let budget = v.budget;
    let n = v.stock.len();
    let mut a = ShopChoice {
        sold: vec![0.0; n],
        spent: vec![0.0; n],
        demand_units: vec![0.0; n],
    };
    if n == 0 || budget <= 0.0 {
        return a;
    }
    let avail0: Vec<f64> = v.stock.to_vec();
    let prices: Vec<f64> = v.price.iter().map(|p| p.max(min_price)).collect();
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
    d.trace.push(TraceRecord::RentBurden {
        burden: if burdens.is_empty() {
            0.0
        } else {
            burdens.iter().sum::<f64>() / burdens.len() as f64
        },
        renters: burdens.len() as u32,
    });
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

/// The price a firm moves toward: unit cost plus its markup (cents, any unit that unit cost uses).
/// Demand never enters: this is the whole price rule of the goods markets.
pub fn price_target(markup: f64, unit_cost: f64) -> f64 {
    (1.0 + markup) * unit_cost
}

// ---- Decision: the price ----

pub const FIRM_PRICE: DecisionSpec = DecisionSpec {
    id: D::FirmPrice,
    system: SystemId::Goods,
    inputs: &[Q::Price, Q::Markup, Q::UnitCost],
    outputs: &[Q::Price],
    effects: &[
        Fx::up(Q::Price, Q::Price),
        Fx::up(Q::Markup, Q::Price),
        Fx::up(Q::UnitCost, Q::Price),
    ],
    params: &["goods.price_adjust", "goods.min_price_usd"],
    rule: "cost_plus",
};

/// What a firm sees when it sets its price. `markup` is firm state (not a parameter).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PriceView {
    pub price: Cents,
    pub markup: f64,
    /// Wages plus carbon cost per unit made (0 when it made nothing).
    pub unit_cost: f64,
}

pub fn price_view(s: &State, p: &Params, f: &Firm) -> PriceView {
    PriceView {
        price: f.price,
        markup: f.markup.0,
        unit_cost: safe_div(
            (wage_bill(s, p, f) + f.carbon_cost).as_f64(),
            f.output.0,
            0.0,
        ),
    }
}

/// Close a share of the gap to `(1 + markup) x unit cost` each month; never below the floor.
pub fn decide_price(v: &PriceView, gp: &GoodsParams) -> Cents {
    let min_price = gp.min_price_usd * 100.0;
    let price = v.price.as_f64().max(min_price);
    let target = price_target(v.markup, v.unit_cost);
    if v.unit_cost > 0.0 {
        Cents::from_f64(price + gp.price_adjust * (target - price)).max(Cents::from_f64(min_price))
    } else {
        v.price
    }
}

// ---- Decision: how much a household spends, and on what ----

pub const HOUSEHOLD_SPEND: DecisionSpec = DecisionSpec {
    id: D::HouseholdSpend,
    system: SystemId::Goods,
    inputs: &[
        Q::HouseholdSize,
        Q::FoodPrice,
        Q::Savings,
        Q::LastDisposable,
        Q::SpendingPropensity,
        Q::NetWealth,
        Q::WealthSpendingRate,
    ],
    outputs: &[Q::FoodNeed, Q::ConsumptionBudget],
    effects: &[
        Fx::up(Q::HouseholdSize, Q::FoodNeed),
        Fx::up(Q::HouseholdSize, Q::ConsumptionBudget),
        Fx::none(Q::FoodPrice, Q::FoodNeed, "the food need is in units (people times need per person); its price only changes what it costs"),
        Fx::up(Q::FoodPrice, Q::ConsumptionBudget),
        Fx::none(Q::Savings, Q::FoodNeed, "the food need depends on household size, not on savings"),
        Fx::up(Q::Savings, Q::ConsumptionBudget),
        Fx::none(Q::LastDisposable, Q::FoodNeed, "the food need depends on household size, not on income"),
        Fx::up(Q::LastDisposable, Q::ConsumptionBudget),
        Fx::none(Q::SpendingPropensity, Q::FoodNeed, "the food need depends on household size, not on the propensity"),
        Fx::up(Q::SpendingPropensity, Q::ConsumptionBudget),
        Fx::none(Q::NetWealth, Q::FoodNeed, "the food need depends on household size, not on wealth"),
        Fx::up(Q::NetWealth, Q::ConsumptionBudget),
        Fx::none(Q::WealthSpendingRate, Q::FoodNeed, "the food need depends on household size, not on the wealth spending rate"),
        Fx::up(Q::WealthSpendingRate, Q::ConsumptionBudget),
    ],
    params: &[
        "goods.food_need_units_pc",
        "goods.services_share",
        "goods.wealth_consumption_rate",
        "goods.propensity.capitalist",
        "goods.propensity.rentier",
        "goods.propensity.manager",
        "goods.propensity.worker",
        "goods.propensity.dependent",
    ],
    rule: "need_first_then_propensity",
};

/// What a household sees when it plans the month's spending. The class picks the propensity and
/// whether net wealth is spent (`wealth_spending_rate` is 0 for the classes that do not).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpendView {
    pub household_size: u32,
    /// Cents per unit of food (the sector's average price).
    pub food_price: f64,
    pub savings: Cents,
    pub last_disposable: Cents,
    pub spending_propensity: f64,
    /// Cents.
    pub net_wealth: f64,
    pub wealth_spending_rate: f64,
}

/// `net_wealth` and `food_price` are computed once for all households by `run` and passed in.
pub fn spend_view(p: &Params, h: &Household, food_price: f64, net_wealth: f64) -> SpendView {
    let gp = &p.goods;
    SpendView {
        household_size: h.members.len() as u32,
        food_price,
        savings: h.savings,
        last_disposable: h.last_disposable,
        spending_propensity: gp.propensity.get(h.class),
        net_wealth,
        // Wealth effect: capital-owning classes also consume out of net wealth.
        wealth_spending_rate: if matches!(h.class, Class::Capitalist | Class::Rentier) {
            gp.wealth_consumption_rate
        } else {
            0.0
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpendChoice {
    /// Units of food the household needs this month.
    pub food_need_units: f64,
    /// Cents.
    pub food_budget: f64,
    pub goods_budget: f64,
    pub services_budget: f64,
}

/// Food first (what the need costs, up to savings), then a propensity share of what is left of
/// disposable income plus a slice of net wealth, within savings, split into goods and services.
pub fn decide_spend(v: &SpendView, gp: &GoodsParams) -> SpendChoice {
    let food_need_units = v.household_size as f64 * gp.food_need_units_pc;
    let need_cost = food_need_units * v.food_price;
    let savings = v.savings.as_f64();
    let food_budget = need_cost.min(savings);
    let disposable = v.last_disposable.as_f64().max(0.0);
    let wealth_spend = v.wealth_spending_rate * v.net_wealth.max(0.0);
    let wish = v.spending_propensity * (disposable - need_cost).max(0.0) + wealth_spend;
    let discretionary = wish.min(savings - food_budget).max(0.0);
    let services_budget = discretionary * gp.services_share;
    SpendChoice {
        food_need_units,
        food_budget,
        goods_budget: discretionary - services_budget,
        services_budget,
    }
}

// ---- Decision: what a firm expects to sell ----

pub const FIRM_EXPECT: DecisionSpec = DecisionSpec {
    id: D::FirmExpect,
    system: SystemId::Goods,
    inputs: &[Q::DemandUnits, Q::ExpectedSales],
    outputs: &[Q::ExpectedSales],
    effects: &[
        Fx::up(Q::DemandUnits, Q::ExpectedSales),
        Fx::up(Q::ExpectedSales, Q::ExpectedSales),
    ],
    params: &["labour.expected_smoothing"],
    rule: "exponential_smoothing",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpectView {
    /// What buyers wanted this month, in units (more than was sold when stock ran short).
    pub demand_units: f64,
    pub expected_sales: f64,
}

pub fn expect_view(f: &Firm, demand_units: f64) -> ExpectView {
    ExpectView {
        demand_units,
        expected_sales: f.expected_sales.0,
    }
}

/// Next month's expected sales: a smoothing of this month's demand into the old expectation.
/// Hiring reads it, so this is the link from households' spending to a firm's workforce.
pub fn decide_expect(v: &ExpectView, lp: &LabourParams) -> Units {
    let lam = lp.expected_smoothing;
    Units(lam * v.demand_units + (1.0 - lam) * v.expected_sales)
}

// ---- Mechanisms (causality graph stage 1): declarations of the plain code below; they change no behaviour. ----

/// What households buy moves money from their savings to the firm, takes units out of its stock and counts as consumption (`goods::run`).
pub const GOODS_SETTLEMENT: MechanismSpec = MechanismSpec {
    id: M::GoodsSettlement,
    system: SystemId::Goods,
    inputs: &[Q::UnitsSold, Q::Price],
    outputs: &[Q::Revenue, Q::Inventory, Q::Savings, Q::Consumption],
    effects: &[
        Fx::up(Q::UnitsSold, Q::Revenue),
        Fx::down(Q::UnitsSold, Q::Inventory),
        Fx::down(Q::UnitsSold, Q::Savings),
        Fx::up(Q::UnitsSold, Q::Consumption),
        Fx::up(Q::Price, Q::Revenue),
        Fx::none(
            Q::Price,
            Q::Inventory,
            "selling a unit takes one unit of stock whatever it costs",
        ),
        Fx::down(Q::Price, Q::Savings),
        Fx::up(Q::Price, Q::Consumption),
    ],
    params: &[],
    rule: "units_times_price",
};

/// A firm's unit cost is its wage bill plus its carbon charge, per unit made (`price_view`).
pub const UNIT_COST_BUILDUP: MechanismSpec = MechanismSpec {
    id: M::UnitCostBuildup,
    system: SystemId::Goods,
    inputs: &[Q::Wage, Q::Workers, Q::CarbonPrice, Q::Footprint, Q::Output],
    outputs: &[Q::UnitCost],
    effects: &[
        Fx::up(Q::Wage, Q::UnitCost),
        Fx::up(Q::Workers, Q::UnitCost),
        Fx::up(Q::CarbonPrice, Q::UnitCost),
        Fx::up(Q::Footprint, Q::UnitCost),
        Fx::down(Q::Output, Q::UnitCost),
    ],
    params: &[],
    rule: "wages_plus_carbon_per_unit",
};

/// Rent rises while every home is taken and renters can bear it, and falls otherwise (`goods::housing`).
pub const HOUSING_RENT: MechanismSpec = MechanismSpec {
    id: M::HousingRent,
    system: SystemId::Goods,
    inputs: &[Q::Occupancy, Q::LastDisposable, Q::Rent],
    outputs: &[Q::Rent],
    effects: &[
        Fx::up(Q::Occupancy, Q::Rent),
        Fx::up(Q::LastDisposable, Q::Rent),
        Fx::varies(Q::Rent, Q::Rent, "each month's rent is a step up or down from this one, but a higher rent also lowers the next step up because renters' burden reaches its cap"),
    ],
    params: &[
        "goods.occupancy_trigger",
        "goods.rent_burden_cap",
        "goods.rent_up",
        "goods.rent_down",
        "goods.min_rent_usd",
    ],
    rule: "rent_follows_occupancy_and_burden",
};

/// Households move to a home they can afford; unpaid rent for too long loses the home (`goods::housing`).
pub const HOUSING_MOVES: MechanismSpec = MechanismSpec {
    id: M::HousingMoves,
    system: SystemId::Goods,
    inputs: &[Q::Rent, Q::LastDisposable, Q::Savings],
    outputs: &[Q::Occupancy],
    effects: &[
        Fx::down(Q::Rent, Q::Occupancy),
        Fx::up(Q::LastDisposable, Q::Occupancy),
        Fx::none(Q::Savings, Q::Occupancy, "a household moves in if its rent is within a share of last month's disposable income; savings are not read"),
    ],
    params: &["goods.move_rent_share", "goods.evict_after_months"],
    rule: "move_evict_by_affordability",
};

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let gp = &p.goods;
    let n_h = s.households.len();

    housing(s, p, &mut d);

    // ---- Prices and budgets ----
    let p_food = sim_metrics::sector_price(s, Sector::Food);
    let wealth = household_wealth(s, p);
    let mut food_budget = vec![0.0; n_h];
    let mut goods_budget = vec![0.0; n_h];
    let mut services_budget = vec![0.0; n_h];
    let mut need_units = vec![0.0; n_h];
    for (i, h) in s.households.iter().enumerate() {
        let c = decide_spend(&spend_view(p, h, p_food, wealth[i]), gp);
        need_units[i] = c.food_need_units;
        food_budget[i] = c.food_budget;
        goods_budget[i] = c.goods_budget;
        services_budget[i] = c.services_budget;
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
        let shop = shop_inputs(&firms);
        let alloc = decide_shop(&shop.view(budget), gp);
        for (i, f) in firms.iter().enumerate() {
            let price = f.price.as_f64().max(min_price);
            let revenue = Cents(alloc.spent[i].floor() as i64);
            let sold_units = alloc.sold[i];
            let parts = apportion(revenue, &buyers);
            for (b, part) in parts.iter().enumerate() {
                if *part > Cents::ZERO && revenue > Cents::ZERO {
                    let buyer = if b < n_h {
                        TradeBuyer::Household(HouseholdId(b as u32))
                    } else if b < n_h + n_f {
                        TradeBuyer::Firm(FirmId((b - n_h) as u32))
                    } else {
                        TradeBuyer::Government
                    };
                    d.trace.push(TraceRecord::Trade {
                        sector,
                        buyer,
                        firm: f.id,
                        units: sold_units * part.as_f64() / revenue.as_f64(),
                        unit_price: price,
                        amount: *part,
                    });
                }
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
            let new_price = decide_price(&price_view(s, p, f), gp);
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
            let expected = decide_expect(&expect_view(f, alloc.demand_units[i]), &p.labour);
            let h = f.sales_hist;
            d.trace.push(TraceRecord::GoodsFirm {
                firm: f.id,
                demand_units: alloc.demand_units[i],
                stock_units: f.inventory.0,
                sold_units,
            });
            d.firm(
                f.id,
                FirmPatch {
                    inventory: Some(Units((f.inventory.0 - sold_units).max(0.0))),
                    price: Some(new_price),
                    sales_hist: Some([sold_units, h[0], h[1]]),
                    sales_units: Some(Units(sold_units)),
                    revenue: Some(revenue),
                    excess_demand: Some(Rate(excess)),
                    expected_sales: Some(expected),
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
