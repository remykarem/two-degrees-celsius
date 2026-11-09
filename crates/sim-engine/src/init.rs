//! Builds the initial town from `Params::initial`.

use sim_params::Params;
use sim_types::*;

fn rate_f(p: &Params) -> f64 {
    p.initial.policy.base_rate + p.credit.spread_base
}

fn tile(p: [u8; 2]) -> Tile {
    Tile { x: p[0], y: p[1] }
}

/// Fraction of the grid's fossil capacity in total capacity times the fossil factor.
fn grid_intensity(plants: &[Plant], factor: f64) -> f64 {
    let total: f64 = plants.iter().map(|p| p.capacity.0).sum();
    let fossil: f64 = plants
        .iter()
        .filter(|p| p.kind == PlantKind::Fossil)
        .map(|p| p.capacity.0)
        .sum();
    safe_div(fossil, total, 0.0) * factor
}

pub fn initial_state(p: &Params) -> Result<State, String> {
    let t = &p.initial;
    let nf = t.firms.len();

    // People and households.
    let mut persons: Vec<Person> = Vec::new();
    let mut households: Vec<Household> = Vec::new();
    for (i, c) in t.households.iter().enumerate() {
        let hid = HouseholdId(i as u32);
        let mut members = Vec::new();
        for pi in &c.people {
            let id = PersonId(persons.len() as u32);
            members.push(id);
            persons.push(Person {
                id,
                label: pi.label.clone(),
                household: hid,
                age_band: pi.age_band,
                skill: Rate(pi.skill),
                in_labour_force: pi.labour_force,
                reservation: Cents::from_dollars(pi.reservation_usd),
                job: pi.job.map(|f| FirmId(f as u32)),
                jg: false,
                wage: Cents::ZERO,
                commute_tiles: 0,
                commute_minutes: 0,
                effective_hours: Rate(1.0),
                wellbeing: Rate(0.6),
            });
        }
        households.push(Household {
            id: hid,
            label: c.label.clone(),
            members,
            home: c.home.map(|h| HomeId(h as u32)),
            class: c.class,
            savings: Cents::from_dollars(c.savings_usd),
            debt: Cents::from_dollars(c.debt_usd),
            income: IncomeFlows::default(),
            income_hist: Vec::new(),
            last_disposable: Cents::from_dollars(c.initial_income_usd),
            food_met: Rate(1.0),
            consumption: Cents::ZERO,
            wellbeing: Rate(0.6),
            political_power: Rate(0.0),
            footprint: Tonnes::ZERO,
            energy_use: Units::ZERO,
            wealth: Cents::ZERO,
            arrears: false,
            arrears_months: 0,
        });
    }

    // Homes, plants and the map.
    let mut occupant: Vec<Option<HouseholdId>> = vec![None; t.homes.len()];
    for (i, c) in t.households.iter().enumerate() {
        if let Some(h) = c.home {
            occupant[h] = Some(HouseholdId(i as u32));
        }
    }
    let homes: Vec<Home> = t
        .homes
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let owner = match h.owner {
                Some(o) => HomeOwner::Household(HouseholdId(o as u32)),
                None => HomeOwner::Public,
            };
            let rent = match owner {
                HomeOwner::Public => Cents::from_dollars(t.rent_public_usd),
                HomeOwner::Household(_) => Cents::from_dollars(t.rent_private_usd),
            };
            Home {
                id: HomeId(i as u32),
                tile: tile(h.tile),
                owner,
                household: occupant[i],
                rent,
                value: Cents::from_dollars(h.value_usd),
            }
        })
        .collect();
    let plants: Vec<Plant> = t
        .plants
        .iter()
        .enumerate()
        .map(|(i, pl)| Plant {
            id: PlantId(i as u32),
            kind: pl.kind,
            tile: tile(pl.tile),
            capacity: Units(pl.capacity),
            output: Units::ZERO,
            emissions: Tonnes::ZERO,
        })
        .collect();
    let mut map = Map::new(t.map.width, t.map.height);
    let scenery = |map: &mut Map, kind: TileKind, tiles: &[[u8; 2]]| {
        for q in tiles {
            map.set(tile(*q), MapTile { kind, id: None });
        }
    };
    scenery(&mut map, TileKind::Road, &t.map.roads);
    scenery(&mut map, TileKind::Forest, &t.map.forest);
    scenery(&mut map, TileKind::Commons, &t.map.commons);
    scenery(&mut map, TileKind::TownHall, &[t.map.town_hall]);
    scenery(&mut map, TileKind::Bank, &[t.map.bank]);
    for h in &homes {
        let kind = match h.owner {
            HomeOwner::Public => TileKind::PublicHousing,
            HomeOwner::Household(_) => TileKind::Home,
        };
        map.set(
            h.tile,
            MapTile {
                kind,
                id: Some(h.id.0),
            },
        );
    }
    for pl in &plants {
        let kind = match pl.kind {
            PlantKind::Fossil => TileKind::FossilPlant,
            PlantKind::Renewable => TileKind::RenewablePlant,
        };
        map.set(
            pl.tile,
            MapTile {
                kind,
                id: Some(pl.id.0),
            },
        );
    }
    for (i, f) in t.firms.iter().enumerate() {
        if f.active {
            map.set(
                tile(f.tile),
                MapTile {
                    kind: TileKind::Firm,
                    id: Some(i as u32),
                },
            );
        }
    }

    // Firms. TFP and the markup are set below, once the state exists to read pay from.
    let mut firms = Vec::with_capacity(nf);
    for (i, f) in t.firms.iter().enumerate() {
        let n = persons
            .iter()
            .filter(|x| x.job == Some(FirmId(i as u32)))
            .count() as f64;
        let price = Cents::from_dollars(f.price_usd);
        let debt = Cents::from_dollars(f.debt_usd);
        firms.push(Firm {
            id: FirmId(i as u32),
            active: f.active,
            sector: f.sector,
            tile: tile(f.tile),
            owner: Owner::Household(HouseholdId(f.founder as u32)),
            founder: HouseholdId(f.founder as u32),
            capital: Units(f.capital_units),
            cash: Cents::from_dollars(f.cash_usd),
            debt,
            amort: debt.scale(1.0 / p.credit.firm_term_months),
            workers: People(n),
            wage: Cents::from_dollars(f.wage_usd),
            price,
            inventory: Units(f.inventory_units),
            intensity_base: Rate(f.intensity_base),
            energy_per_unit: Rate(f.energy_per_unit),
            green_tech: Rate(0.0),
            tfp: Rate(0.0),
            output: Units(f.initial_output),
            emissions: Tonnes::ZERO,
            footprint: Tonnes::ZERO,
            energy_use: Units::ZERO,
            sales_units: Units(f.initial_output),
            sales_hist: [f.initial_output; 3],
            revenue: price.scale(f.initial_output),
            profit: Cents::ZERO,
            carbon_cost: Cents::ZERO,
            profit_ema: Cents::ZERO,
            unfilled: false,
            over_target_months: 0,
            distress_months: 0,
            wage_shortfall: false,
            plan_invest: Cents::from_dollars(f.plan_invest_usd),
            loan_request: Cents::ZERO,
            loan_rate: Rate(rate_f(p)),
            green_plan: false,
            excess_demand: Rate::ZERO,
            expected_sales: Units(f.initial_output),
            markup: Rate(0.0),
            capital_per_sale: safe_div(f.capital_units, f.initial_output, 0.0),
            capex: Cents::ZERO,
            tax_paid: Cents::ZERO,
            closed_tick: None,
        });
    }
    let forest_health = p.initial.forest / p.nature.forest_max;
    let commons_health = p.initial.commons / p.nature.commons_max;
    let health = 0.5 * (forest_health + commons_health);
    let pol = &t.policy;
    let policy = Policy {
        carbon_price: Cents::from_dollars(pol.carbon_price_usd),
        tax_wage: Rate(pol.tax_wage),
        tax_profit: Rate(pol.tax_profit),
        tax_capital: Rate(pol.tax_capital),
        wealth_tax: Rate(pol.wealth_tax),
        wealth_threshold: Cents::from_dollars(pol.wealth_threshold_usd),
        base_rate: Rate(pol.base_rate),
        min_wage: Cents::from_dollars(pol.min_wage_usd),
        benefit: Cents::from_dollars(pol.benefit_usd),
        ubi: Cents::from_dollars(pol.ubi_usd),
        green_subsidy: Rate(pol.green_subsidy),
        campaign_cap: Rate(pol.campaign_cap),
        emissions_cap: Tonnes(pol.emissions_cap),
        commons_harvest: Rate(pol.commons_harvest),
        bank_capital_ratio: Rate(pol.bank_capital_ratio),
        bailout: pol.bailout,
        public_spending: Cents::from_dollars(pol.public_spending_usd),
        job_guarantee: Rate(pol.job_guarantee),
    };
    let effective = EffectivePolicy {
        carbon_price: policy.carbon_price,
        wealth_tax: policy.wealth_tax,
        tax_profit: policy.tax_profit,
        emissions_cap: policy.emissions_cap,
    };
    let mut s = State {
        schema_version: SCHEMA_VERSION,
        tick: 0,
        turn: 1,
        political_capital: p.turn.pc_start,
        pending: Vec::new(),
        next_pending_id: 1,
        system: EconomicSystem::Market,
        transition: None,
        output_penalty: Rate::ZERO,
        persons,
        households,
        firms,
        homes,
        plants,
        next_home_id: t.homes.len() as u32,
        next_plant_id: t.plants.len() as u32,
        map,
        bank: Bank {
            owner: HouseholdId(t.bank_owner as u32),
            equity: Cents::ZERO,
            loans: Cents::ZERO,
            deposits: Cents::ZERO,
            default_rate: Rate::ZERO,
            failed: false,
            bailout_this_tick: false,
            lent_this_tick: Cents::ZERO,
        },
        gov: Government {
            cash: Cents::from_dollars(t.gov_cash_usd),
            debt: Cents::from_dollars(t.gov_debt_usd),
            legitimacy: Rate(t.legitimacy),
            tax_revenue: Cents::ZERO,
            spending: Cents::ZERO,
            unfunded: Cents::ZERO,
            unfunded_total: Cents::ZERO,
            receipts: Cents::ZERO,
            pending_build: Vec::new(),
        },
        nature: Nature {
            forest: Units(t.forest),
            commons: Units(t.commons),
            commons_access: CommonsAccess::Open,
            cumulative: Tonnes::ZERO,
            net_emissions: Tonnes::ZERO,
            gross_emissions: Tonnes::ZERO,
            sink: Tonnes::ZERO,
            atmosphere: Tonnes(p.nature.atmosphere_start),
            harvest_commons: Units::ZERO,
            harvest_forest: Units::ZERO,
            depletion: Units::ZERO,
            pending_clearing: Units::ZERO,
        },
        housing: Housing {
            rent_private: Cents::from_dollars(t.rent_private_usd),
            rent_public: Cents::from_dollars(t.rent_public_usd),
            occupancy: Rate::ZERO,
            unhoused_households: 0,
        },
        energy: Energy {
            demand: Units::ZERO,
            supply: Units::ZERO,
            delivered: Units::ZERO,
            served: Rate(1.0),
            grid_intensity: Rate::ZERO,
        },
        policy,
        effective,
        union_density: Rate(t.union_density),
        net_lobby: Rate::ZERO,
        indicators: Indicators::default(),
        gdp_parts: GdpParts::default(),
        outcome: None,
    };
    let gi = grid_intensity(&s.plants, p.energy.factor_fossil);
    s.energy.supply = Units(s.plants.iter().map(|x| x.capacity.0).sum());
    s.energy.grid_intensity = Rate(gi);
    let occupied = s.homes.iter().filter(|h| h.household.is_some()).count() as f64;
    s.housing.occupancy = Rate(safe_div(occupied, s.homes.len() as f64, 1.0));
    s.housing.unhoused_households = s.households.iter().filter(|h| h.home.is_none()).count() as u32;
    // Commutes at the start: the cheapest routes from the homes to the workplaces.
    let jobs: Vec<Option<FirmId>> = s.persons.iter().map(|x| x.job).collect();
    for (x, (tiles, minutes)) in sim_model::space::commutes(&s, p, &jobs)
        .into_iter()
        .enumerate()
    {
        let hours = if jobs[x].is_some() {
            sim_model::space::effective_hours(&p.space, minutes)
        } else {
            1.0
        };
        let person = &mut s.persons[x];
        person.commute_tiles = tiles;
        person.commute_minutes = minutes;
        person.effective_hours = Rate(hours);
    }
    // TFP and the price markup are derived from the start: output and price are given.
    for fi in 0..s.firms.len() {
        let f = &s.firms[fi];
        if !f.active {
            continue;
        }
        let l0: f64 = s
            .persons
            .iter()
            .filter(|x| x.job == Some(f.id))
            .map(|x| x.skill.0 * x.effective_hours.0)
            .sum();
        let mut denom =
            libm::pow(f.capital.0, p.production.alpha_k) * libm::pow(l0, p.production.beta_n);
        if f.sector == Sector::Food {
            denom *= libm::pow(health, p.production.farm_health_exp);
        }
        let tfp = if l0 > 0.0 && f.capital.0 > 0.0 {
            t.firms[fi].initial_output / denom
        } else {
            0.0
        };
        let bill = sim_model::util::wage_bill(&s, p, f).as_f64();
        let unit_cost = safe_div(bill, t.firms[fi].initial_output, 0.0);
        let markup = if unit_cost > 0.0 {
            t.firms[fi].price_usd * 100.0 / unit_cost - 1.0
        } else {
            0.0
        };
        s.firms[fi].tfp = Rate(tfp);
        s.firms[fi].markup = Rate(markup);
    }
    // The bank holds only loans: equity = loans - deposits.
    s.bank.loans = s.total_debts();
    s.bank.deposits = s.total_deposits();
    s.bank.equity = s.bank.loans - s.bank.deposits;
    if s.bank.equity.is_negative() {
        return Err(format!(
            "initial town has negative bank equity ({} dollars): loans must exceed deposits",
            s.bank.equity.dollars()
        ));
    }
    let required = s.bank.loans.scale(s.policy.bank_capital_ratio.0);
    if s.bank.equity < required {
        return Err("initial bank equity is below the capital requirement".into());
    }
    Ok(s)
}
