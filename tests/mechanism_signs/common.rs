//! Shared fixtures of the system-level sign test: real mid-run states (copied from
//! `tests/mechanism_params.rs`, which builds the same samples for the parameter gate), the params,
//! and `after`, which runs one system of `TICK_ORDER` on a state and applies its deltas, so a probe
//! can nudge a field of the state and read a field of the result.
#![allow(dead_code)]

use sim_engine::apply::{apply, ApplyStats, Capture};
use sim_params::{ContentJson, Params};
use sim_rng::Rng;
use sim_types::*;
use std::panic::{catch_unwind, AssertUnwindSafe};

const PARAMS: &str = include_str!("../../content/params.json");
const TOWN: &str = include_str!("../../content/initial_town.json");
const MARKET: &str = include_str!("../../content/systems/market.json");
const REGULATED: &str = include_str!("../../content/systems/regulated.json");
const COOP: &str = include_str!("../../content/systems/coop.json");

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

pub fn params() -> Params {
    load(PARAMS).unwrap()
}

/// The states the systems are run on: mid-run states of an idle plan, and variants that reach
/// branches they do not.
pub fn states() -> Vec<State> {
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

/// Run the system `id` of the tick order on `s` (fixed RNG stream) and apply its deltas.
pub fn after(id: SystemId, s: &State, p: &Params) -> State {
    let f = sim_model::TICK_ORDER
        .iter()
        .find(|(sid, _)| *sid == id)
        .map(|(_, f)| *f)
        .expect("system in the tick order");
    let mut next = s.clone();
    let mut r = Rng::new(7, 3);
    let d = f(s, p, &mut r);
    let (mut ev, mut tr) = (Vec::new(), Vec::new());
    apply(
        &mut next,
        p,
        d,
        &mut ev,
        &mut tr,
        &mut ApplyStats::default(),
        &mut Capture::default(),
    );
    next
}

/// Like `after`, but `None` when the system panics on a sample (a sample the code rejects is skipped).
pub fn try_after(id: SystemId, s: &State, p: &Params) -> Option<State> {
    catch_unwind(AssertUnwindSafe(|| after(id, s, p))).ok()
}

/// How to read the level of an input where the nudge does not change a number the state prints
/// (the elasticity of the response needs a level; `sim_model::sign_probe`). The probe uses the
/// entry of the input it nudges, if there is one.
pub fn levels() -> Vec<sim_model::sign_probe::Read<State>> {
    // `Workers`: the people who have a job (the probes give a jobless person a job).
    let w: fn(&State) -> f64 = |s| s.persons.iter().filter(|x| x.job.is_some()).count() as f64;
    // `Unemployment`: the share of the labour force without a job (a person loses a job).
    let u: fn(&State) -> f64 = |s| {
        let lf = s
            .persons
            .iter()
            .filter(|x| x.in_labour_force)
            .count()
            .max(1) as f64;
        s.persons
            .iter()
            .filter(|x| x.in_labour_force && !x.employed())
            .count() as f64
            / lf
    };
    // `Occupancy`: the share of households that have a home (a homeless one takes a vacant home).
    let o: fn(&State) -> f64 = |s| {
        s.households.iter().filter(|h| h.home.is_some()).count() as f64
            / s.households.len().max(1) as f64
    };
    vec![
        (QuantityId::Workers, w),
        (QuantityId::Unemployment, u),
        (QuantityId::Occupancy, o),
    ]
}
