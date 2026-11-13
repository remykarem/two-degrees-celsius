//! Parameter gate for the mechanism specs (`docs/features/causality-graph.md` stage 1, C5/C7).
//!
//! A `MechanismSpec` declares the quantities it reads and writes and the `params.json` leaves it
//! uses, next to the plain code it describes. This test perturbs every numeric leaf of
//! `params.json` in turn, runs the real system function (`TICK_ORDER`) on mid-run states and on
//! variants that reach branches the plain run does not (a stressed climate, a drought, a fire,
//! a fenced-off or community commons, a transition under way), and records which leaves change
//! the system's output.
//!
//! It proves: every parameter a mechanism declares is read by its system (a typo, or a parameter
//! the code does not read, fails), unless it is listed in `DORMANT` with the reason. It reports,
//! per system, the parameters the system reads that no decision or mechanism of that system
//! declares (`UNDECLARED`, printed with `--nocapture`); the test fails if that list grows beyond
//! the committed one.
//!
//! It does NOT prove: that the declared quantity inputs and outputs are what the code reads
//! (the state is not perturbed per quantity: a quantity is an aggregate of many fields, and one
//! system writes several mechanisms' outputs together), that a declared param is read by *this*
//! mechanism rather than another one of the same system, reads behind a branch no sample takes,
//! the system files (`systems/*.json`, DP-12), enums, or the RNG. The quantity lists are checked
//! only for being well formed (`causal.rs` tests), and for what the wire serves.

use serde_json::Value;
use sim_model::decisions::{DECISIONS, MECHANISMS};
use sim_model::TICK_ORDER;
use sim_params::{ContentJson, Params};
use sim_rng::Rng;
use sim_types::*;
use std::collections::{BTreeMap, BTreeSet};
use std::panic::{catch_unwind, AssertUnwindSafe};

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");

fn load(params: &str) -> Option<Params> {
    catch_unwind(AssertUnwindSafe(|| {
        Params::from_content(&ContentJson {
            params,
            initial_town: TOWN,
            market: MARKET,
            regulated: REGULATED,
            coop: COOP,
        })
    }))
    .ok()?
    .ok()
}

/// Declared parameters that no sampled state makes a difference to, with the reason.
const DORMANT: &[(&str, &str)] = &[
    // Fire station cover changes the spread odds and the burn time of a fire next to a station.
    // The sampled states build a station beside a forest tile, but no sampled draw spread a fire
    // across the covered tile (a random outcome, the DP-12 blind spot), so these three are
    // declared from the code (`covered`, `spread_p`, `fire_duration`) and proven only by reading.
    (
        "weather.station_spread_cut",
        "needs a spread draw next to a station",
    ),
    (
        "weather.station_duration_cut",
        "needs a new fire next to a station",
    ),
    ("weather.coverage_radius", "needs a fire next to a station"),
];

/// Parameters a system reads that no decision or mechanism of that system declares, as the gate
/// found them. Shrink it by declaring them; the test fails if it grows. Empty entries mean the
/// system has none.
const UNDECLARED: &[(&str, &[&str])] = &[
    // Read through helpers shared with decisions of other systems (the wage bill and the cash
    // reserve are used by shopping and investing), so they belong to those decisions' specs.
    (
        "Goods",
        &[
            "invest.capital_price_usd",
            "invest.min_cash_months",
            "labour.skill_wage_floor",
            "labour.skill_wage_ref",
            "labour.skill_wage_slope",
            // The government-services share of GDP: a figure kept for the GDP indicator.
            "goods.gov_services_share",
        ],
    ),
    (
        "Investment",
        &[
            "invest.min_cash_months",
            "labour.skill_wage_floor",
            "labour.skill_wage_ref",
            "labour.skill_wage_slope",
        ],
    ),
    // The fee a fenced-off commons charges is a money flow (accounting) no mechanism describes;
    // the forest-shrank threshold only raises an event.
    (
        "Nature",
        &["nature.enclosure_fee_usd", "nature.forest_shrank_share"],
    ),
    // Only decides whether the "rationed" event fires.
    ("Production", &["energy.rationed_below"]),
    // Only the tonnes a fire adds to the emissions trace (not to any state).
    // The repair bill of a home a fire damages is a money flow no mechanism describes.
    (
        "Weather",
        &["nature.sink_sigma", "weather.home_damage_prob"],
    ),
    // The wealth record (household_wealth) is a stored figure no mechanism describes.
    ("Wellbeing", &["invest.capital_price_usd"]),
];

fn walk(v: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                path.push(k.clone());
                walk(x, path, out);
                path.pop();
            }
        }
        Value::Number(_) => out.push(path.clone()),
        _ => {}
    }
}

/// Three bumps per leaf (a large rise, a cut, a small rise): a large one can leave the valid
/// range (a share above 1) and then the parameters do not load.
fn bump(root: &mut Value, at: &[String], how: usize) {
    let mut cur = root;
    for seg in at {
        cur = &mut cur[seg.as_str()];
    }
    *cur = match &*cur {
        Value::Number(n) if n.is_f64() => {
            let x = n.as_f64().unwrap();
            serde_json::json!([x * 1.7 + 0.31, x * 0.6, x * 1.1][how])
        }
        Value::Number(n) if n.is_i64() => {
            let x = n.as_i64().unwrap();
            serde_json::json!([x.saturating_mul(2).saturating_add(3), x / 2, x + 1][how])
        }
        Value::Number(n) => {
            let x = n.as_u64().unwrap();
            serde_json::json!([x.saturating_mul(2).saturating_add(3), x / 2, x + 1][how])
        }
        other => other.clone(),
    };
}

/// The states the systems are run on: mid-run states of an idle plan, and variants that reach
/// branches they do not.
fn states() -> Vec<State> {
    let p = load(PARAMS).unwrap();
    let mut sim = sim_engine::init(p, 42).unwrap();
    let plan = sim_engine::Plan {
        name: "idle".into(),
        turns: vec![],
    };
    let mut out = Vec::new();
    for turns in [3u32, 8, 14] {
        sim_engine::run(&mut sim, turns, &plan, |_, _| {}).unwrap();
        out.push(sim.state.clone());
    }
    let base = out[1].clone();
    // A stressed climate with a drought and fires (one on a firm, one on a home, so smoke
    // reaches both), resilience spending, a flight and a transition under way; a depleted commons
    // so the food cap binds, and the budget used past the cap on the climate ratio.
    let mut hot = base.clone();
    hot.nature.cumulative = Tonnes(780.0);
    hot.weather.stress = 0.9;
    hot.weather.drought = Some(Drought {
        severity: 0.6,
        months_left: 3,
    });
    let mut fires = Vec::new();
    if let Some(t) = hot
        .map
        .tiles()
        .find(|t| hot.map.kind_at(*t) == Some(TileKind::Forest))
    {
        fires.push(Fire {
            tile: t,
            months_left: 2,
        });
    }
    fires.push(Fire {
        tile: hot.firms[0].tile,
        months_left: 2,
    });
    if let Some(h) = hot.homes.first() {
        fires.push(Fire {
            tile: h.tile,
            months_left: 2,
        });
    }
    hot.weather.fires = fires;
    hot.policy.resilience_spending = Cents(300_000);
    hot.output_penalty = Rate(0.1);
    hot.nature.commons = Units(40.0);
    hot.nature.forest = Units(60.0);
    if let Some(e) = hot.estate.as_mut() {
        e.jet.flying_this_month = true;
    }
    out.push(hot.clone());
    let mut over = hot.clone();
    over.nature.cumulative = Tonnes(1500.0);
    out.push(over);
    let mut community = hot.clone();
    community.nature.commons = Units(400.0);
    community.nature.forest = Units(1500.0);
    community.nature.commons_access = CommonsAccess::Community { cap: Rate(0.01) };
    out.push(community);
    let mut enclosed = hot.clone();
    enclosed.nature.commons_access = CommonsAccess::Enclosed(HouseholdId(0));
    out.push(enclosed);
    // The weather's own stream, several seeds: a drought or fire starts only on some draws.
    // A fire station beside a forest tile, so the cover parameters matter.
    let forest_cell = base
        .map
        .cells
        .iter()
        .position(|c| c.kind == TileKind::Forest);
    for k in 1..=150u64 {
        let mut w = base.clone();
        w.nature.cumulative = Tonnes(700.0);
        w.weather.stress = 0.8;
        w.weather.rng = RngStream {
            state: k.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            inc: 2 * k + 1,
        };
        if let Some(i) = forest_cell.filter(|i| *i > 0) {
            w.map.cells[i - 1].kind = TileKind::FireStation;
        }
        w.policy.resilience_spending = Cents(300_000);
        if let Some(i) = forest_cell.filter(|_| k % 2 == 1) {
            w.weather.fires = vec![Fire {
                tile: Tile {
                    x: (i % w.map.width as usize) as u8,
                    y: (i / w.map.width as usize) as u8,
                },
                months_left: 3,
            }];
        }
        if k % 3 == 0 {
            w.weather.drought = Some(Drought {
                severity: 0.4,
                months_left: 2,
            });
        }
        out.push(w);
    }
    // Housing: a rent above the floor, rent unpaid for months, a household without a home and
    // one that is rich (so rent does not take too much of its income).
    let mut housing = base.clone();
    housing.housing.rent_private = Cents(90_000);
    for h in &mut housing.households {
        h.last_disposable = Cents(50_000_000);
    }
    out.push(housing.clone());
    let mut arrears = housing.clone();
    for h in &mut arrears.households {
        h.arrears_months = 3;
        h.savings = Cents::ZERO;
    }
    out.push(arrears);
    // Rent at the floor with renters who cannot bear it, so it would fall below it.
    let mut floor = housing.clone();
    floor.housing.rent_private = Cents(2_000);
    for h in &mut floor.households {
        h.last_disposable = Cents::ZERO;
    }
    out.push(floor);
    // Homeless households that can just afford the rent of a free home.
    let mut homeless = housing;
    for h in homeless.households.iter_mut().skip(1) {
        h.home = None;
        h.last_disposable = Cents(300_000);
    }
    for (i, home) in homeless.homes.iter_mut().enumerate() {
        if i > 0 {
            home.household = None;
            home.rent = Cents(90_000);
        }
    }
    out.push(homeless);
    out
}

fn run_system(f: sim_model::SystemFn, s: &State, p: &Params) -> Option<String> {
    catch_unwind(AssertUnwindSafe(|| {
        let mut r = Rng::new(7, 3);
        format!("{:?}", f(s, p, &mut r))
    }))
    .ok()
}

#[test]
fn declared_mechanism_params_are_read_by_their_system() {
    let params_json: Value = serde_json::from_str(PARAMS).unwrap();
    let mut leaves = Vec::new();
    walk(&params_json, &mut Vec::new(), &mut leaves);
    let p = load(PARAMS).unwrap();
    // The Policy and Metrics systems have no mechanism with parameters; skip them.
    let systems: BTreeSet<SystemId> = MECHANISMS
        .iter()
        .filter(|m| !m.params.is_empty())
        .map(|m| m.system)
        .collect();
    let mut read: BTreeMap<SystemId, BTreeSet<String>> = BTreeMap::new();
    let samples = states();
    for sys in &systems {
        let f = TICK_ORDER
            .iter()
            .find(|(id, _)| id == sys)
            .unwrap_or_else(|| panic!("{sys:?} is not a tick system"))
            .1;
        let bases: Vec<Option<String>> = samples.iter().map(|s| run_system(f, s, &p)).collect();
        for at in &leaves {
            for how in 0..3 {
                let mut v = params_json.clone();
                bump(&mut v, at, how);
                let Some(p2) = load(&v.to_string()) else {
                    continue;
                };
                let moved = samples
                    .iter()
                    .zip(&bases)
                    .any(|(s, b)| b.is_some() && run_system(f, s, &p2) != *b);
                if moved {
                    read.entry(*sys).or_default().insert(at.join("."));
                    break;
                }
            }
        }
    }
    let mut problems = Vec::new();
    let mut checked = 0;
    for m in MECHANISMS {
        for path in m.params {
            checked += 1;
            let used = read.get(&m.system).is_some_and(|s| s.contains(*path));
            let dormant = DORMANT.iter().any(|(d, _)| d == path);
            if !used && !dormant {
                problems.push(format!(
                    "{}: declared {path} but {:?} never changed with it",
                    m.id.as_str(),
                    m.system
                ));
            }
        }
    }
    // Parameters a system reads that nothing of that system declares.
    let mut found: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (sys, set) in &read {
        for path in set {
            let declared = DECISIONS
                .iter()
                .filter(|d| d.system == *sys)
                .any(|d| d.params.contains(&path.as_str()))
                || MECHANISMS
                    .iter()
                    .filter(|m| m.system == *sys)
                    .any(|m| m.params.contains(&path.as_str()));
            if !declared {
                found
                    .entry(format!("{sys:?}"))
                    .or_default()
                    .insert(path.clone());
            }
        }
    }
    println!("declared mechanism params checked: {checked}");
    for (sys, set) in &found {
        println!("UNDECLARED {sys} ({}): {set:?}", set.len());
    }
    for (sys, set) in &found {
        let allowed: &[&str] = UNDECLARED
            .iter()
            .find(|(s, _)| s == sys)
            .map_or(&[], |(_, a)| a);
        for path in set {
            if !allowed.contains(&path.as_str()) {
                problems.push(format!("{sys}: reads {path}, declared by nothing"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
