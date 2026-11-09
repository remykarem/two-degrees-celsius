//! Builds the initial town from `Params::initial`.

use sim_params::Params;
use sim_types::*;

fn rate_f(p: &Params) -> f64 {
    p.initial.policy.base_rate + p.credit.spread_base
}

pub fn initial_state(p: &Params) -> Result<State, String> {
    let t = &p.initial;
    let nf = t.firms.len();
    let households: Vec<Household> = t
        .cohorts
        .iter()
        .enumerate()
        .map(|(i, c)| Household {
            id: HouseholdId(i as u32),
            label: c.label.clone(),
            class: c.class,
            weight: People(c.weight),
            labour_share: Rate(c.labour_share),
            reservation: Cents::from_dollars(c.reservation_usd),
            wage_premium: Rate(c.wage_premium),
            nonlabour_benefit: c.nonlabour_benefit,
            renter_share: Rate(c.renter_share),
            home_value: Cents::from_dollars(c.home_value_usd),
            savings: Cents::from_dollars(c.savings_usd),
            debt: Cents::from_dollars(c.debt_usd),
            skill: Rate(c.skill),
            employment: t.firms.iter().map(|f| People(f.employment[i])).collect(),
            income: IncomeFlows::default(),
            income_hist: Vec::new(),
            last_disposable: Cents::from_dollars(c.initial_income_usd),
            food_met: Rate(1.0),
            housed_share: Rate(1.0),
            consumption: Cents::ZERO,
            wellbeing: Rate(0.6),
            political_power: Rate(0.0),
            footprint: Tonnes::ZERO,
            wealth_pc: Cents::ZERO,
            arrears: false,
        })
        .collect();
    let forest_health = p.initial.forest / p.nature.forest_max;
    let commons_health = p.initial.commons / p.nature.commons_max;
    let health = 0.5 * (forest_health + commons_health);
    let mut firms = Vec::with_capacity(nf);
    for (i, f) in t.firms.iter().enumerate() {
        let n: f64 = f.employment.iter().sum();
        let price = Cents::from_dollars(f.price_usd);
        let tfp = if f.active && n > 0.0 && f.capital_units > 0.0 {
            let mut denom = libm::pow(f.capital_units, p.production.alpha_k)
                * libm::pow(n, p.production.beta_n);
            if f.sector == Sector::Food {
                denom *= libm::pow(health, p.production.farm_health_exp);
            }
            f.initial_output / denom
        } else {
            // Dormant slots use the sector's first active firm's productivity when founded.
            0.0
        };
        let loan_rate = rate_f(p);
        let debt = Cents::from_dollars(f.debt_usd);
        firms.push(Firm {
            id: FirmId(i as u32),
            active: f.active,
            sector: f.sector,
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
            green_tech: Rate(0.0),
            tfp: Rate(tfp),
            output: Units(f.initial_output),
            emissions: Tonnes::ZERO,
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
            loan_rate: Rate(loan_rate),
            green_plan: false,
            excess_demand: Rate::ZERO,
            closed_tick: None,
        });
    }
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
        public_spending: Rate(pol.public_spending),
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
        households,
        firms,
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
            purchases_budget: Cents::from_dollars(t.gov_purchases_usd),
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
            owner: HouseholdId(1.min(t.cohorts.len() as u32 - 1)),
            private_cap: Units(t.private_housing),
            public_cap: Units(t.public_housing),
            rent_private: Cents::from_dollars(t.rent_private_usd),
            rent_public: Cents::from_dollars(t.rent_public_usd),
            occupancy: Rate::ZERO,
            housed_share: Rate(1.0),
        },
        energy: Energy {
            fossil_cap: Units(t.fossil_capacity),
            renewable_cap: Units(t.renewable_capacity),
        },
        policy,
        effective,
        union_density: Rate(t.union_density),
        net_lobby: Rate::ZERO,
        indicators: Indicators::default(),
        gdp_parts: GdpParts::default(),
        outcome: None,
    };
    // Housing owner: the first cohort whose initial class is Rentier, else cohort 0.
    if let Some(h) = s.households.iter().find(|h| h.class == Class::Rentier) {
        s.housing.owner = h.id;
    } else {
        s.housing.owner = HouseholdId(0);
    }
    let renters: f64 = s
        .households
        .iter()
        .map(|h| h.weight.0 * h.renter_share.0)
        .sum();
    let rental_cap = s.housing.private_cap.0 + s.housing.public_cap.0;
    s.housing.occupancy = Rate(sim_types::safe_div(renters, rental_cap, 1.0));
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
