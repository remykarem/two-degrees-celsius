//! The builder gate of the decision points (`docs/features/decision-points.md` section 11, DP-1).
//!
//! Each decision has a view builder that reads `State` and `Params`. This test takes real mid-run
//! states, serialises state and parameters to JSON, perturbs every numeric or boolean leaf in
//! turn, rebuilds each view and decides, and records which leaves change which choice. Each
//! decision declares the leaves its builder reads (`READS`; array indices are written `[]`).
//! It fails when a leaf outside the declaration changes a choice (an undeclared read), and when a
//! declaration never changes a choice in any sampled state (a dead declaration).
//!
//! It can show: in the sampled states, no undeclared leaf of State or Params changes the choice
//! of the sampled owners. It cannot show: reads behind a branch the samples never take, reads of
//! another array element (indices are collapsed), or reads inside a `decide_*` body (types keep
//! `State` and `Rng` out of those). Arguments `run` computes and passes to a builder (cash after
//! the green step, capital after depreciation, the draw) are fed from the state's own fields here.

use serde_json::Value;
use sim_engine::Plan;
use sim_model::{class, goods, income, investment, labour, util};
use sim_params::{ContentJson, Params};
use sim_types::*;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");

fn load(params: &str) -> Params {
    Params::from_content(&ContentJson {
        params,
        initial_town: TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
    .expect("content loads")
}

/// One line per decision and owner: the view the builder made and the choice, as text. Comparing
/// the view too means a leaf the builder reads shows up even when it does not tip the choice.
fn choices(s: &State, p: &Params) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    let mut put = |k: String, view: String, choice: String| {
        m.insert(k, format!("{view} -> {choice}"));
    };
    let min_wage = s.policy.min_wage.scale(util::cfg(s, p).min_wage_multiplier);
    let unemployment = s.indicators.unemployment_smooth.0;
    for f in s.firms.iter().filter(|f| f.active) {
        let id = f.id.0;
        let mine: Vec<&Person> = util::workers_of(s, f.id).collect();
        let hv = labour::hire_view(s, p, f, &mine);
        let hire = labour::decide_hire(&hv, &p.labour, &p.production);
        put(
            format!("firm_hire/{id}"),
            format!("{hv:?}"),
            format!("{hire:?}"),
        );
        let fv = labour::fire_view(f, mine.len() as f64, &hire);
        let fire = labour::decide_fire(&fv, &p.labour);
        put(
            format!("firm_fire/{id}"),
            format!("{fv:?}"),
            format!("{fire:?}"),
        );
        let wv = labour::wage_view(p, f, unemployment, min_wage);
        let wage = labour::decide_wage(&wv, &p.labour);
        put(
            format!("firm_wage/{id}"),
            format!("{wv:?}"),
            format!("{wage:?}"),
        );
        let pv = goods::price_view(s, p, f);
        let price = goods::decide_price(&pv, &p.goods);
        put(
            format!("firm_price/{id}"),
            format!("{pv:?}"),
            format!("{price:?}"),
        );
        let reserve = util::cash_reserve(s, p, f);
        let gv = investment::green_view(s, p, f, f.cash, reserve);
        let green = investment::decide_green(&gv, &p.invest);
        put(
            format!("firm_green/{id}"),
            format!("{gv:?}"),
            format!("{green:?}"),
        );
        let cv = investment::capex_view(s, p, f, f.capital.0, f.cash, reserve, f.green_plan);
        let capex = investment::decide_capex(&cv, &p.invest);
        put(
            format!("firm_capex/{id}"),
            format!("{cv:?}"),
            format!("{capex:?}"),
        );
        let interest_due = f.debt.scale(util::monthly(f.loan_rate.0));
        let ov = income::payout_view(
            s,
            p,
            f,
            f.profit,
            f.cash,
            util::wage_bill(s, p, f),
            interest_due,
            f.amort.min(f.debt),
        );
        let payout = income::decide_payout(&ov, &p.income, &p.invest);
        put(
            format!("firm_payout/{id}"),
            format!("{ov:?}"),
            format!("{payout:?}"),
        );
    }
    for sector in [Sector::Goods, Sector::Food, Sector::Services] {
        // A draw of 0: a firm is founded whenever the chance is above zero.
        if let Some(v) = class::found_view(s, p, sector, 0.0) {
            let found = class::decide_found(&v, &p.class);
            put(
                format!("household_found_firm/{sector:?}"),
                format!("{v:?}"),
                format!("{found:?}"),
            );
        }
    }
    m
}

/// Every numeric or boolean leaf: its normalised path (array indices as `[]`) and a way to bump it.
fn leaves(v: &Value, path: &str, at: &mut Vec<String>, out: &mut Vec<(String, Vec<String>)>) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                at.push(k.clone());
                leaves(x, &format!("{path}.{k}"), at, out);
                at.pop();
            }
        }
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                at.push(format!("#{i}"));
                leaves(x, &format!("{path}[]"), at, out);
                at.pop();
            }
        }
        Value::Number(_) | Value::Bool(_) => out.push((path.to_string(), at.clone())),
        _ => {}
    }
}

fn bump(root: &mut Value, at: &[String]) {
    let mut cur = root;
    for seg in at {
        cur = match seg.strip_prefix('#') {
            Some(i) => &mut cur[i.parse::<usize>().unwrap()],
            None => &mut cur[seg.as_str()],
        };
    }
    *cur = match &*cur {
        Value::Bool(b) => Value::Bool(!b),
        Value::Number(n) if n.is_f64() => {
            serde_json::json!(n.as_f64().unwrap() * 1.7 + 0.31)
        }
        Value::Number(n) if n.is_i64() => {
            serde_json::json!(n.as_i64().unwrap().saturating_mul(2).saturating_add(3))
        }
        Value::Number(n) => {
            serde_json::json!(n.as_u64().unwrap().saturating_mul(2).saturating_add(3))
        }
        other => other.clone(),
    };
}

/// Normalised leaf paths that change each decision's choice, for one state and parameter set.
fn sensitivity(s: &State, p: &Params, params_json: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let base = choices(s, p);
    let mut hit: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut note = |path: &str, now: BTreeMap<String, String>| {
        for (k, v) in &now {
            if base.get(k) != Some(v) {
                let d = k.split('/').next().unwrap().to_string();
                hit.entry(d).or_default().insert(path.to_string());
            }
        }
    };
    let sj = serde_json::to_value(s).unwrap();
    let mut all = Vec::new();
    leaves(&sj, "state", &mut Vec::new(), &mut all);
    for (path, at) in &all {
        let mut v = sj.clone();
        bump(&mut v, at);
        let Ok(s2) = serde_json::from_value::<State>(v) else {
            continue;
        };
        if let Ok(now) = catch_unwind(AssertUnwindSafe(|| choices(&s2, p))) {
            note(path, now);
        }
    }
    let mut all = Vec::new();
    leaves(params_json, "params", &mut Vec::new(), &mut all);
    for (path, at) in &all {
        let mut v = params_json.clone();
        bump(&mut v, at);
        let text = v.to_string();
        let Ok(p2) = catch_unwind(AssertUnwindSafe(|| {
            Params::from_content(&ContentJson {
                params: &text,
                initial_town: TOWN,
                market: MARKET,
                regulated: REGULATED,
                coop: COOP,
            })
        })) else {
            continue;
        };
        let Ok(p2) = p2 else { continue };
        if let Ok(now) = catch_unwind(AssertUnwindSafe(|| choices(s, &p2))) {
            note(path, now);
        }
    }
    hit
}

/// Leaves every decision's sample selection reads: which firms are active, and whose id it is.
const OWNER: &[&str] = &["state.firms[].active", "state.firms[].id"];

/// What `util::wage_bill` (so `cash_reserve` and unit cost) reads: base wage, who works there, and
/// their skill and pay scale.
const WAGE_BILL: &[&str] = &[
    "state.firms[].wage",
    "state.persons[].job",
    "state.persons[].skill",
    "state.policy.min_wage",
    "params.labour.skill_wage_floor",
    "params.labour.skill_wage_ref",
    "params.labour.skill_wage_slope",
];

/// What `util::cash_reserve` adds to the wage bill (an argument `run` computes and passes in).
const RESERVE: &[&str] = &[
    "state.firms[].amort",
    "state.firms[].debt",
    "state.firms[].loan_rate",
    "params.invest.min_cash_months",
];

/// Hire parameters the fire view does not carry (it takes only `workers_wanted` from the choice).
const FIRE_IGNORES: &[&str] = &[
    "params.labour.max_hire_per_month",
    "params.labour.min_target_workers",
    "params.class.skill_threshold",
];

/// Declared, but only reachable across a threshold the sampled states do not straddle, so the
/// dead-declaration check skips them.
const DORMANT: &[&str] = &[
    "params.invest.green_payback_years",
    "params.invest.green_max",
    "params.class.startup_min_excess_demand",
];

const HIRE: &[&str] = &[
    "state.firms[].capital",
    "state.firms[].expected_sales",
    "state.firms[].inventory",
    "state.firms[].tfp",
    "state.persons[].effective_hours",
    "state.persons[].job",
    "state.persons[].skill",
    "state.nature.commons",
    "state.nature.cumulative",
    "state.nature.forest",
    "state.output_penalty",
    "params.labour.growth_aspiration",
    "params.labour.inventory_adjust",
    "params.labour.inventory_cover",
    "params.labour.services_inventory_cover",
    "params.labour.max_hire_per_month",
    "params.labour.min_target_workers",
    "params.class.skill_threshold",
    "params.nature.budget_b0",
    "params.nature.commons_max",
    "params.nature.forest_max",
    "params.production.alpha_k",
    "params.production.beta_n",
    "params.production.climate_damage",
    "params.production.farm_health_exp",
];

/// The decisions this test covers. `firm_exit` has no `State` builder (its view is made of values
/// `credit::run` computes from this month's payments), so only its per-decision test covers it.
const COVERED: &[&str] = &[
    "firm_hire",
    "firm_fire",
    "firm_wage",
    "firm_price",
    "firm_green",
    "firm_capex",
    "firm_payout",
    "household_found_firm",
];

/// The leaves each decision's builder and function are declared to read. A declared path covers
/// every path that starts with it.
fn reads(decision: &str) -> Vec<&'static str> {
    let own: &[&str] = match decision {
        "firm_hire" => HIRE,
        // The fire view carries the hire choice's `workers_wanted`.
        "firm_fire" => &[
            "state.firms[].over_target_months",
            "params.labour.fire_slack",
        ],
        "firm_wage" => &[
            "state.firms[].wage",
            "state.indicators.unemployment_smooth",
            "state.policy.min_wage",
            "params.labour.inflation_anchor",
            "params.labour.productivity_passthrough",
            "params.labour.u_ref",
            "params.labour.wage_k",
            "params.production.tfp_growth_food",
            "params.production.tfp_growth_goods",
            "params.production.tfp_growth_services",
        ],
        "firm_price" => &[
            "state.firms[].markup",
            "state.firms[].output",
            "state.firms[].price",
            "state.firms[].carbon_cost",
            "params.goods.price_adjust",
        ],
        "firm_green" => &[
            "state.effective.emissions_cap",
            "state.effective.carbon_price",
            "state.firms[].footprint",
            "state.firms[].green_tech",
            "state.firms[].revenue",
            "state.firms[].cash",
            "state.gov.cash",
            "state.policy.green_subsidy",
            "params.invest.green_spend_share",
            "params.invest.green_cost_share",
            "params.invest.green_payback_years",
            "params.invest.green_step",
            "params.invest.green_max",
        ],
        "firm_capex" => &[
            "state.firms[].capital",
            "state.firms[].capital_per_sale",
            "state.firms[].cash",
            "state.firms[].distress_months",
            "state.firms[].expected_sales",
            "state.firms[].green_plan",
            "state.firms[].profit",
            "state.firms[].profit_ema",
            "state.firms[].wage_shortfall",
            "params.invest.accelerator",
            "params.invest.capital_price_usd",
            "params.invest.depreciation",
            "params.invest.return_scale",
            "params.invest.smoothing",
        ],
        "firm_payout" => &[
            "state.firms[].amort",
            "state.firms[].debt",
            "state.firms[].loan_rate",
            "state.firms[].revenue",
            "state.policy.green_subsidy",
            "state.firms[].cash",
            "state.firms[].green_plan",
            "state.firms[].profit",
            "params.income.cash_reserve_months",
            "params.income.excess_cash_months",
            "params.income.excess_payout_rate",
        ],
        "household_found_firm" => &[
            "state.firms[].excess_demand",
            "state.firms[].capital",
            "state.firms[].profit",
            "state.policy.base_rate",
            "params.credit.spread_base",
            "params.class.startup_min_excess_demand",
            "params.invest.capital_price_usd",
        ],
        _ => &[],
    };
    let mut v: Vec<&'static str> = own.to_vec();
    v.extend_from_slice(OWNER);
    match decision {
        "firm_fire" => v.extend(HIRE.iter().filter(|x| !FIRE_IGNORES.contains(x))),
        "firm_price" | "firm_payout" => v.extend_from_slice(WAGE_BILL),
        "firm_capex" => {
            v.extend_from_slice(WAGE_BILL);
            v.extend_from_slice(RESERVE);
        }
        "firm_green" => {
            v.extend_from_slice(WAGE_BILL);
            v.extend_from_slice(RESERVE);
        }
        _ => {}
    }
    v
}

/// Sampled variants that reach branches the plain run does not: demand left over in every sector
/// (so a firm can be founded) and a carbon price with plenty of cash (so a green step is bought).
fn variants(s: &State) -> Vec<State> {
    let mut demand = s.clone();
    for f in &mut demand.firms {
        f.excess_demand = Rate(1.0);
    }
    let mut carbon = s.clone();
    carbon.effective.carbon_price = Cents(5_000);
    for f in &mut carbon.firms {
        f.footprint = Tonnes(50.0);
        f.cash = Cents(2_000_000_000);
        f.revenue = Cents(10_000_000);
    }
    vec![s.clone(), demand, carbon]
}

#[test]
fn builders_read_only_what_they_declare() {
    let p = load(PARAMS);
    let params_json: Value = serde_json::from_str(PARAMS).unwrap();
    let mut sim = sim_engine::init(p.clone(), 42).unwrap();
    let plan = Plan {
        name: "idle".into(),
        turns: vec![],
    };
    let mut seen: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for turns in [3u32, 6, 12] {
        sim_engine::run(&mut sim, turns, &plan, |_, _| {}).unwrap();
        for state in variants(&sim.state) {
            for (d, set) in sensitivity(&state, &sim.params, &params_json) {
                seen.entry(d).or_default().extend(set);
            }
        }
    }
    let mut problems = Vec::new();
    for (d, set) in &seen {
        let decl = reads(d);
        for path in set {
            if !decl.iter().any(|x| path.starts_with(x)) {
                problems.push(format!("{d}: undeclared read {path}"));
            }
        }
    }
    for d in COVERED {
        for x in reads(d) {
            let used = seen
                .get(*d)
                .is_some_and(|s| s.iter().any(|p| p.starts_with(x)));
            if !used && !OWNER.contains(&x) && !DORMANT.contains(&x) {
                problems.push(format!("{d}: declared {x} never changed a choice"));
            }
        }
    }
    if std::env::var("DISCOVER").is_ok() {
        for (d, set) in &seen {
            println!("{d}: {set:#?}");
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
