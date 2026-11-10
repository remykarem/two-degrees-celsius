//! Town v4 weather: stress curve, drought, fires, fire stations, forecast, invariants and sign
//! tests (docs/causal-model.md section 11). Probability tests call the pure S11 system many
//! times from one state with a seeded generator, so they are exact in expectation and fixed in
//! outcome.

use sim_engine::{Plan, Sim};
use sim_model::{nature, production, weather};
use sim_params::{ContentJson, Params};
use sim_rng::Rng;
use sim_types::*;

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");
const SCENARIO_B: &str = include_str!("../content/scenarios/baseline_b_sensible.json");

fn params() -> Params {
    Params::from_content(&ContentJson {
        params: PARAMS,
        initial_town: TOWN,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
    .expect("content must load")
}

fn new_sim(seed: u64) -> Sim {
    sim_engine::init(params(), seed).expect("init")
}

fn fresh_sim() -> Sim {
    let mut p = params();
    p.run.warmup_ticks = 0;
    sim_engine::init(p, 1).expect("init")
}

/// Cumulative emissions so that `remaining` of the budget is left.
fn set_remaining(s: &mut State, p: &Params, remaining: f64) {
    s.nature.cumulative = Tonnes(p.nature.budget_b0 * (1.0 - remaining));
}

fn apply_weather(s: &mut State, d: &Deltas) {
    for c in &d.changes {
        match c {
            Delta::Weather(w) => s.weather = w.clone(),
            Delta::Nature(n) => s.nature = n.clone(),
            _ => {}
        }
    }
}

/// Run S11 alone for `ticks`, applying its deltas; counts of (started, spread, burned).
fn evolve(s: &mut State, p: &Params, rng: &mut Rng, ticks: u32) -> (u32, u32, u32) {
    let (mut a, mut b, mut c) = (0, 0, 0);
    for _ in 0..ticks {
        let d = weather::run(s, p, rng);
        for e in &d.events {
            match e {
                Event::FireStarted { .. } => a += 1,
                Event::FireSpread { .. } => b += 1,
                Event::ForestBurned { .. } => c += 1,
                _ => {}
            }
        }
        apply_weather(s, &d);
    }
    (a, b, c)
}

fn forest_tiles(s: &State) -> Vec<Tile> {
    s.map
        .tiles()
        .filter(|t| s.map.kind_at(*t) == Some(TileKind::Forest))
        .collect()
}

fn forest_neighbours(s: &State, t: Tile) -> usize {
    let mut n = 0;
    for (dx, dy) in [(0i32, -1i32), (1, 0), (0, 1), (-1, 0)] {
        let (x, y) = (t.x as i32 + dx, t.y as i32 + dy);
        if x >= 0 && y >= 0 && s.map.kind_at(Tile::new(x as u8, y as u8)) == Some(TileKind::Forest)
        {
            n += 1;
        }
    }
    n
}

/// A forest tile with several forest neighbours.
fn inner_forest_tile(s: &State) -> Tile {
    *forest_tiles(s)
        .iter()
        .max_by_key(|t| forest_neighbours(s, **t))
        .unwrap()
}

/// The empty tile closest to `t`.
fn nearest_empty(s: &State, t: Tile) -> Tile {
    s.map
        .tiles()
        .filter(|x| s.map.kind_at(*x) == Some(TileKind::Empty))
        .min_by_key(|x| (x.chebyshev(t), x.manhattan(t)))
        .unwrap()
}

// ---------- stress ----------

#[test]
fn stress_curve_follows_the_documented_formula() {
    let p = params();
    let w = &p.weather;
    let at = |remaining: f64| {
        sim_metrics::climate_stress(Tonnes(p.nature.budget_b0 * (1.0 - remaining)), &p)
    };
    assert_eq!(at(1.0), 0.0, "full budget: no stress");
    assert_eq!(at(w.onset), 0.0, "no stress at onset");
    assert_eq!(at(w.onset + 0.2), 0.0);
    assert!((at(w.full) - 1.0).abs() < 1e-12, "full stress at 10% left");
    assert_eq!(at(0.0), 1.0, "clamped at one");
    let mid = at((w.onset + w.full) / 2.0);
    assert!(
        (mid - libm::pow(0.5, w.curve)).abs() < 1e-9,
        "midpoint {mid}"
    );
    let (a, b) = (at(0.4), at(0.3));
    assert!(0.0 < a && a < b && b < 1.0, "smooth and rising: {a} {b}");
}

#[test]
fn no_hazards_before_onset() {
    let p = params();
    let mut sim = fresh_sim();
    set_remaining(&mut sim.state, &p, 0.6);
    let mut rng = Rng::new(3, 1);
    let (a, _, _) = evolve(&mut sim.state, &p, &mut rng, 600);
    assert_eq!(a, 0);
    assert!(sim.state.weather.drought.is_none() && sim.state.weather.fires.is_empty());
    assert_eq!(
        weather::forecast(&sim.state, &p),
        (RiskLevel::Low, RiskLevel::Low)
    );
}

#[test]
fn a_new_game_starts_with_calm_weather_even_though_the_warm_up_emits() {
    for seed in 1..=5 {
        let sim = new_sim(seed);
        assert_eq!(sim.state.weather, Weather::default(), "seed {seed}");
        assert_eq!(sim.state.nature.cumulative, Tonnes::ZERO);
        assert_eq!(sim.state.indicators.climate_stress, 0.0);
    }
}

// ---------- fire: ignition, spread, duration ----------

#[test]
fn ignition_frequency_matches_the_formula_and_rises_with_stress() {
    let mut p = params();
    // A drought that starts in the same tick raises dryness and so ignition; with the higher
    // drought odds of the Horizon pass that skews this pure-formula check, so switch it off here.
    p.weather.drought_base = 0.0;
    let mut base = fresh_sim().state;
    let mut rate = |remaining: f64, seed: u64| {
        set_remaining(&mut base, &p, remaining);
        let stress = sim_metrics::climate_stress(base.nature.cumulative, &p);
        let expect = weather::ignition_p(&base, &p, &base.weather, stress);
        let mut rng = Rng::new(seed, 9);
        let n = 20_000;
        let mut hits = 0;
        for _ in 0..n {
            let d = weather::run(&base, &p, &mut rng);
            hits += d
                .events
                .iter()
                .filter(|e| matches!(e, Event::FireStarted { .. }))
                .count();
        }
        (hits as f64 / n as f64, expect)
    };
    let (low_obs, low_exp) = rate(0.4, 11);
    let (high_obs, high_exp) = rate(0.3, 11);
    for (obs, exp) in [(low_obs, low_exp), (high_obs, high_exp)] {
        assert!(exp > 0.0);
        assert!(
            (obs - exp).abs() < 0.15 * exp + 0.002,
            "observed {obs} expected {exp}"
        );
    }
    assert!(
        high_exp > low_exp && high_obs > low_obs,
        "more stress, more fires"
    );
}

#[test]
fn spread_frequency_matches_spread_p() {
    let p = params();
    let mut s = fresh_sim().state;
    set_remaining(&mut s, &p, 0.5); // stress exactly zero: no ignition noise, spread still runs
    let t = inner_forest_tile(&s);
    s.weather.fires.push(Fire {
        tile: t,
        months_left: 3,
    });
    let nb = forest_neighbours(&s, t) as f64;
    assert!(nb >= 2.0);
    let mut rng = Rng::new(5, 2);
    let n = 20_000;
    let mut spreads = 0usize;
    for _ in 0..n {
        let d = weather::run(&s, &p, &mut rng);
        spreads += d
            .events
            .iter()
            .filter(|e| matches!(e, Event::FireSpread { .. }))
            .count();
    }
    let expect = nb * p.weather.spread_p;
    let obs = spreads as f64 / n as f64;
    assert!(
        (obs - expect).abs() < 0.05 * expect,
        "observed {obs} expected {expect}"
    );
}

#[test]
fn a_fire_burns_for_its_duration_then_regrows_then_stands_again() {
    let p = params();
    let mut sim = fresh_sim();
    set_remaining(&mut sim.state, &p, 1.0);
    let t = inner_forest_tile(&sim.state);
    sim.state.weather.fires.push(Fire {
        tile: t,
        months_left: 3,
    });
    // Spread is disabled so the single tile is followed.
    let mut p0 = p.clone();
    p0.weather.spread_p = 0.0;
    sim.params = p0.clone();
    let mut out_at = None;
    let mut forest_before = sim.state.nature.forest.0;
    for k in 1..=3 {
        let out = sim_engine::step(&mut sim).unwrap();
        if out.events.contains(&Event::FireOut { tile: t }) {
            out_at = Some(k);
        } else {
            // While burning the forest shrinks (the logistic growth is far smaller).
            assert!(sim.state.nature.forest.0 < forest_before, "tick {k}");
        }
        forest_before = sim.state.nature.forest.0;
    }
    assert_eq!(
        out_at,
        Some(3),
        "burns two more months, out on the third tick"
    );
    assert!(sim.state.weather.fires.is_empty());
    assert_eq!(sim.state.weather.regrowing.len(), 1);
    for _ in 0..p0.weather.regrow_months {
        sim_engine::step(&mut sim).unwrap();
    }
    assert!(sim.state.weather.regrowing.is_empty(), "standing again");
}

#[test]
fn burning_lowers_forest_and_the_sink_and_raises_net_emissions_once() {
    let p = params();
    let (mut a, mut b) = (fresh_sim(), fresh_sim());
    for sim in [&mut a, &mut b] {
        set_remaining(&mut sim.state, &p, 1.0);
        sim.params.weather.spread_p = 0.0;
    }
    let t = inner_forest_tile(&b.state);
    b.state.weather.fires.push(Fire {
        tile: t,
        months_left: 3,
    });
    let forest0 = b.state.nature.forest.0;
    sim_engine::step(&mut a).unwrap();
    let out = sim_engine::step(&mut b).unwrap();
    assert!(b.state.nature.forest.0 < a.state.nature.forest.0);
    assert!(
        b.state.nature.sink.0 < a.state.nature.sink.0,
        "the sink shrinks"
    );
    assert!(b.state.nature.net_emissions.0 > a.state.nature.net_emissions.0);
    // Sink loss is counted once: the sink is sigma x the forest left, and the trace record is
    // informational (it is not an extra term of net emissions).
    let sigma = p.nature.sink_sigma;
    assert!(b.state.nature.sink.0 < sigma * forest0);
    let loss = out
        .trace
        .iter()
        .find_map(|r| match r {
            TraceRecord::Emissions {
                source: EmissionSource::FireSinkLoss,
                tonnes,
            } => Some(tonnes.0),
            _ => None,
        })
        .expect("fire_sink_loss is traced");
    assert!(loss > 0.0);
    let d_sink = sigma * (a.state.nature.forest.0 - b.state.nature.forest.0);
    assert!(
        (d_sink - loss).abs() < 0.2 * loss + 1e-9,
        "{d_sink} vs {loss}"
    );
    let by_source: f64 = out
        .trace
        .iter()
        .filter_map(|r| match r {
            TraceRecord::Emissions { source, tonnes }
                if *source != EmissionSource::FireSinkLoss =>
            {
                Some(tonnes.0)
            }
            _ => None,
        })
        .sum();
    assert!(
        (by_source - b.state.nature.net_emissions.0).abs() < 1e-6,
        "net emissions equal the sources minus the sink, without the loss record"
    );
}

#[test]
fn fire_stations_reduce_the_burnt_area() {
    let p = params();
    let mut open = fresh_sim().state;
    set_remaining(&mut open, &p, 0.3);
    let t = inner_forest_tile(&open);
    open.weather.fires.push(Fire {
        tile: t,
        months_left: 3,
    });
    let mut covered = open.clone();
    let station = nearest_empty(&covered, t);
    covered.map.set(
        station,
        MapTile {
            kind: TileKind::FireStation,
            id: None,
        },
    );
    assert!(weather::covered(&covered, &p, t));
    assert!(!weather::covered(&open, &p, t));
    let (mut burnt_open, mut burnt_cov) = (0u32, 0u32);
    for seed in 0..400 {
        let (mut a, mut b) = (open.clone(), covered.clone());
        let (_, sa, ba) = evolve(&mut a, &p, &mut Rng::new(seed, 1), 12);
        let (_, sb, bb) = evolve(&mut b, &p, &mut Rng::new(seed, 1), 12);
        burnt_open += ba + sa;
        burnt_cov += bb + sb;
    }
    assert!(
        (burnt_cov as f64) < 0.9 * burnt_open as f64,
        "covered {burnt_cov} vs open {burnt_open} tiles burnt/spread over 400 paired runs"
    );
}

#[test]
fn resilience_spending_lowers_ignition_with_diminishing_returns() {
    let p = params();
    let mut s = fresh_sim().state;
    set_remaining(&mut s, &p, 0.1);
    let at = |s: &mut State, usd: f64| {
        s.policy.resilience_spending = Cents::from_dollars(usd);
        weather::ignition_p(s, &p, &s.weather, 1.0)
    };
    let (p0, p1, p2, p3) = (
        at(&mut s, 0.0),
        at(&mut s, 1000.0),
        at(&mut s, 2000.0),
        at(&mut s, 3000.0),
    );
    assert!(p0 > p1 && p1 > p2 && p2 > p3);
    assert!(
        p0 - p1 > p1 - p2 && p1 - p2 > p2 - p3,
        "diminishing returns"
    );
    assert!(
        p3 > p0 * (1.0 - p.weather.resilience_ignition_cut_max),
        "never below the cap"
    );
}

// ---------- drought ----------

fn with_drought(s: &mut State, severity: f64) {
    s.weather.drought = Some(Drought {
        severity,
        months_left: 100,
    });
}

fn growth_only(sim: &mut Sim) {
    // No harvest: the farm produces nothing, so only regrowth moves the commons.
    for f in &mut sim.state.firms {
        f.output = Units::ZERO;
    }
    sim.state.nature.commons = Units(0.5 * sim.params.nature.commons_max);
}

fn commons_after(sim: &Sim) -> f64 {
    let mut r = Rng::new(1, 1);
    let d = nature::run(&sim.state, &sim.params, &mut r);
    d.changes
        .iter()
        .find_map(|c| match c {
            Delta::Nature(n) => Some(n.commons.0),
            _ => None,
        })
        .unwrap()
}

#[test]
fn drought_slows_commons_regrowth_and_a_community_commons_keeps_part_of_it() {
    let mut calm = fresh_sim();
    growth_only(&mut calm);
    let c0 = calm.state.nature.commons.0;
    let a_c = calm.params.weather.a_c;
    let buffer = calm.params.weather.community_buffer;
    let g_calm = commons_after(&calm) - c0;
    assert!(g_calm > 0.0);

    let mut dry = calm.clone();
    with_drought(&mut dry.state, 0.8);
    let g_dry = commons_after(&dry) - c0;
    assert!(
        (g_dry / g_calm - (1.0 - a_c * 0.8)).abs() < 1e-9,
        "open commons"
    );

    let mut comm = dry.clone();
    comm.state.nature.commons_access = CommonsAccess::Community { cap: Rate(10.0) };
    let g_comm = commons_after(&comm) - c0;
    assert!(
        (g_comm / g_calm - (1.0 - a_c * 0.8 * (1.0 - buffer))).abs() < 1e-9,
        "community commons"
    );
    assert!(g_comm > g_dry, "the community option is a buffer");

    let mut resil = dry.clone();
    resil.state.policy.resilience_spending = Cents::from_dollars(4000.0);
    assert!(
        commons_after(&resil) - c0 > g_dry,
        "resilience lowers drought damage"
    );
}

#[test]
fn drought_cuts_farm_output_and_lifts_electricity_demand() {
    let mut calm = fresh_sim();
    sim_engine::step(&mut calm).unwrap(); // give the firms some state
    let farm = calm
        .state
        .firms
        .iter()
        .find(|f| f.sector == Sector::Food && f.active)
        .unwrap()
        .clone();
    let l = production::effective_labour(&calm.state, &farm).max(1.0);
    let y0 = production::capacity(&calm.state, &calm.params, &farm, l);
    let mut dry = calm.clone();
    with_drought(&mut dry.state, 0.7);
    let y1 = production::capacity(&dry.state, &dry.params, &farm, l);
    let a_f = calm.params.weather.a_f;
    assert!(y0 > 0.0);
    assert!((y1 / y0 - (1.0 - a_f * 0.7)).abs() < 1e-9, "{}", y1 / y0);
    // A goods firm is unaffected by drought.
    let goods = calm
        .state
        .firms
        .iter()
        .find(|f| f.sector == Sector::Goods && f.active)
        .unwrap()
        .clone();
    assert_eq!(
        production::capacity(&calm.state, &calm.params, &goods, l),
        production::capacity(&dry.state, &dry.params, &goods, l)
    );
    let demand = |sim: &Sim| {
        let mut r = Rng::new(1, 1);
        production::run(&sim.state, &sim.params, &mut r)
            .changes
            .iter()
            .find_map(|c| match c {
                Delta::Energy(e) => Some(e.demand.0),
                _ => None,
            })
            .unwrap()
    };
    assert!(
        demand(&dry) > demand(&calm),
        "drought raises electricity demand"
    );
}

#[test]
fn drought_events_start_and_end_and_last_their_months() {
    let p = params();
    let mut s = fresh_sim().state;
    set_remaining(&mut s, &p, 0.1);
    let mut rng = Rng::new(21, 1);
    let mut started: Option<u32> = None;
    let mut tick = 0u32;
    let mut checked = 0;
    while checked < 3 && tick < 20_000 {
        tick += 1;
        let d = weather::run(&s, &p, &mut rng);
        for e in &d.events {
            match e {
                Event::DroughtStarted { severity, months } => {
                    assert!((0.0..=1.0).contains(&severity.0) && *months >= 2);
                    started = Some(tick + months);
                }
                Event::DroughtEnded => {
                    assert_eq!(
                        started,
                        Some(tick),
                        "a drought of N months ends N ticks after it began"
                    );
                    checked += 1;
                }
                _ => {}
            }
        }
        apply_weather(&mut s, &d);
    }
    assert_eq!(checked, 3, "droughts occur at full stress");
}

// ---------- forecast ----------

#[test]
fn forecast_is_deterministic_rises_with_stress_and_uses_no_randomness() {
    let p = params();
    let mut s = fresh_sim().state;
    let levels = |s: &mut State, remaining: f64| {
        set_remaining(s, &p, remaining);
        weather::forecast(s, &p)
    };
    assert_eq!(levels(&mut s, 0.9), (RiskLevel::Low, RiskLevel::Low));
    let mid = levels(&mut s, 0.3);
    let full = levels(&mut s, 0.1);
    assert!(mid.0 >= RiskLevel::Low && full.0 >= mid.0 && full.1 >= mid.1);
    assert!(
        full.0 > RiskLevel::Low,
        "fire risk is flagged at full stress"
    );
    assert_eq!(levels(&mut s, 0.1), full, "same state, same forecast");
    s.weather.drought = Some(Drought {
        severity: 0.5,
        months_left: 3,
    });
    assert_eq!(weather::forecast(&s, &p).1, RiskLevel::High);
    s.weather.fires.push(Fire {
        tile: inner_forest_tile(&s),
        months_left: 2,
    });
    assert_eq!(weather::forecast(&s, &p).0, RiskLevel::High);
}

// ---------- build, lever, wire-independent engine rules ----------

#[test]
fn fire_station_goes_through_the_build_pipeline() {
    let mut sim = fresh_sim();
    let p = sim.params.clone();
    let place = |x, y| Command::Place {
        kind: InfraKind::FireStation,
        tile: Tile::new(x, y),
    };
    let forest = forest_tiles(&sim.state)[0];
    let on_forest = sim_engine::preview(&sim, &[place(forest.x, forest.y)])[0];
    assert!(!on_forest.tile_valid, "no station on forest");
    let t = nearest_empty(&sim.state, forest);
    let cost = sim_engine::preview(&sim, &[place(t.x, t.y)])[0];
    assert!(cost.tile_valid && cost.blocked.is_none());
    assert_eq!(
        cost.money,
        Cents::from_dollars(p.weather.fire_station_cost_usd)
    );
    assert_eq!(cost.lead_time, 2);
    assert!(cost.pc > 0.0);
    let valid = sim_engine::turn::valid_tiles(&sim, InfraKind::FireStation);
    assert!(valid.contains(&t) && !valid.contains(&forest));
    // Submitted: built after its lead time, then paid and maintained by the government.
    sim_engine::submit_turn(&mut sim, &[place(t.x, t.y)]).unwrap();
    for _ in 0..3 {
        sim_engine::submit_turn(&mut sim, &[]).unwrap();
    }
    assert_eq!(sim.state.map.kind_at(t), Some(TileKind::FireStation));
    assert!(
        sim_engine::preview(&sim, &[place(t.x, t.y)])[0].tile_reason
            == Some(BlockReason::TileOccupied)
    );
}

#[test]
fn resilience_spending_is_a_lever_with_a_one_turn_lead_and_is_paid_by_the_government() {
    let mut sim = fresh_sim();
    let c = Command::SetPolicy(PolicyChange::ResilienceSpending(Cents::from_dollars(
        2000.0,
    )));
    let cost = sim_engine::preview(&sim, &[c])[0];
    assert_eq!(cost.lead_time, 1);
    assert!(cost.pc > 0.0 && cost.money == Cents::ZERO);
    sim_engine::submit_turn(&mut sim, &[c]).unwrap();
    assert_eq!(
        sim.state.policy.resilience_spending,
        Cents::ZERO,
        "not yet active"
    );
    let out = sim_engine::submit_turn(&mut sim, &[]).unwrap();
    assert_eq!(
        sim.state.policy.resilience_spending,
        Cents::from_dollars(2000.0)
    );
    let _ = out;
    // It is government spending: services (GDP) include it.
    let with = sim.state.gdp_parts.gov_services;
    let mut plain = fresh_sim();
    sim_engine::submit_turn(&mut plain, &[]).unwrap();
    sim_engine::submit_turn(&mut plain, &[]).unwrap();
    assert!(with > plain.state.gdp_parts.gov_services);
}

// ---------- invariants over full runs ----------

fn check_weather_invariants(sim: &Sim) {
    let s = &sim.state;
    let max = sim.params.nature.forest_max;
    assert!(
        (0.0..=max + 1e-9).contains(&s.nature.forest.0),
        "forest {}",
        s.nature.forest.0
    );
    let mut seen: Vec<Tile> = Vec::new();
    for t in s
        .weather
        .fires
        .iter()
        .map(|f| f.tile)
        .chain(s.weather.regrowing.iter().map(|g| g.tile))
    {
        assert_eq!(
            s.map.kind_at(t),
            Some(TileKind::Forest),
            "fires and regrowth only on forest"
        );
        assert!(!seen.contains(&t), "a tile is in exactly one state");
        seen.push(t);
    }
}

#[test]
fn invariants_hold_over_full_runs_with_weather_and_every_system_check_passes() {
    for (seed, script) in [
        (1u64, Plan::default()),
        (2, Plan::from_json(SCENARIO_B).unwrap()),
        (3, Plan::default()),
    ] {
        let mut sim = new_sim(seed);
        let mut fire_ticks = 0;
        sim_engine::run(&mut sim, 40, &script, |s, _| {
            check_weather_invariants(s);
            if !s.state.weather.fires.is_empty() {
                fire_ticks += 1;
            }
        })
        .unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        println!("seed {seed}: {fire_ticks} burning ticks");
        assert_eq!(sim.state.schema_version, 3);
    }
}

#[test]
fn home_damage_when_enabled_conserves_money() {
    let mut p = params();
    p.weather.home_damage_prob = 1.0;
    p.weather.home_repair_cost_usd = 50.0;
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p, 4).unwrap();
    // A forest tile next to a home, on fire.
    let home = sim.state.homes[0].tile;
    let t = sim
        .state
        .map
        .tiles()
        .find(|x| sim.state.map.kind_at(*x) == Some(TileKind::Empty) && x.chebyshev(home) == 1)
        .expect("an empty tile next to a home");
    sim.state.map.set(
        t,
        MapTile {
            kind: TileKind::Forest,
            id: None,
        },
    );
    sim.state.weather.fires.push(Fire {
        tile: t,
        months_left: 3,
    });
    let mut damaged = 0;
    for _ in 0..2 {
        let out = sim_engine::step(&mut sim).expect("money conserved and flows funded");
        damaged += out
            .events
            .iter()
            .filter(|e| matches!(e, Event::HomeDamaged { .. }))
            .count();
    }
    assert!(
        damaged > 0,
        "homes next to fires get damaged when the option is on"
    );
}

#[test]
fn home_damage_is_off_by_default() {
    assert_eq!(params().weather.home_damage_prob, 0.0);
}

#[test]
fn same_seed_same_weather_different_seeds_diverge() {
    let run = |seed: u64| {
        let mut sim = new_sim(seed);
        let mut events = 0usize;
        sim_engine::run(&mut sim, 40, &Plan::default(), |_, _| {}).unwrap();
        let mut sim = new_sim(seed);
        sim_engine::run(&mut sim, 40, &Plan::default(), |_, o| {
            events += o
                .events
                .iter()
                .filter(|e| matches!(e, Event::FireStarted { .. } | Event::DroughtStarted { .. }))
                .count()
        })
        .unwrap();
        (
            serde_json::to_string(&sim.state.weather).unwrap(),
            sim.rng.parts(),
            events,
        )
    };
    assert_eq!(run(5), run(5), "determinism per seed");
    let all: Vec<_> = (1..=8).map(run).collect();
    let distinct = all
        .iter()
        .map(|r| (r.0.clone(), r.1))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        distinct.len() >= 6,
        "seeds now differ: {} distinct of 8",
        distinct.len()
    );
}

// ---------- developer override and visibility (Horizon pass) ----------

#[test]
fn force_stress_override_sets_stress_and_brings_weather_at_once() {
    assert!(params().weather.force_stress.is_none(), "off by default");
    let mut p = params();
    p.weather.force_stress = Some(0.7);
    p.run.warmup_ticks = 0;
    let mut sim = sim_engine::init(p.clone(), 5).unwrap();
    assert_eq!(
        sim_metrics::climate_stress(sim.state.nature.cumulative, &p),
        0.7
    );
    let mut events = 0;
    sim_engine::run(&mut sim, 6, &Plan::default(), |_, o| {
        events += o
            .events
            .iter()
            .filter(|e| matches!(e, Event::FireStarted { .. } | Event::DroughtStarted { .. }))
            .count();
    })
    .unwrap();
    assert_eq!(sim.state.indicators.climate_stress, 0.7);
    assert!(events > 0, "forced stress 0.7 shows weather within 6 turns");
    p.weather.force_stress = Some(1.5);
    assert!(p.validate().is_err(), "force_stress must be in [0, 1]");
}

/// Visibility: in a no-action game the player sees weather soon after onset (50% remaining) and
/// keeps seeing it. Measured over seeds 1..=20 until the budget is exhausted.
#[test]
fn a_no_action_game_shows_weather_within_three_turns_of_onset_in_nine_seeds_in_ten() {
    let (mut seen, mut totals) = (0, vec![]);
    for seed in 1..=20u64 {
        let mut sim = new_sim(seed);
        let b0 = sim.params.nature.budget_b0;
        let (mut onset, mut first, mut n, mut over) = (None, None, 0u32, false);
        let turns = sim.params.run.turns;
        sim_engine::run(&mut sim, turns, &Plan::default(), |s, o| {
            if over {
                return;
            }
            if onset.is_none() && s.state.nature.cumulative.0 >= 0.5 * b0 {
                onset = Some(s.state.tick);
            }
            for e in &o.events {
                if matches!(e, Event::FireStarted { .. } | Event::DroughtStarted { .. }) {
                    n += 1;
                    first.get_or_insert(s.state.tick);
                }
            }
            over = s.state.nature.cumulative.0 >= b0;
        })
        .unwrap();
        if let (Some(o), Some(f)) = (onset, first) {
            if f <= o + 9 {
                seen += 1;
            }
        }
        totals.push(n);
    }
    let mean = totals.iter().sum::<u32>() as f64 / 20.0;
    println!("events seen within 3 turns of onset: {seen} of 20; events per game {totals:?} (mean {mean:.1})");
    assert!(
        seen >= 18,
        "weather visible within 3 turns of onset in {seen} of 20 seeds"
    );
    assert!(
        (5.0..=14.0).contains(&mean),
        "events per no-action game {mean}: steady, not a flood"
    );
}

// ---------- baselines with weather: numbers for docs/tuning-log.md ----------

#[derive(Default, Debug)]
struct Tally {
    half_turn: f64,
    exhausted_turn: f64,
    fires: u32,
    droughts: u32,
    burnt: u32,
    farm_low: f64,
}

fn tally(script: &Plan, seed: u64) -> Tally {
    tally_with(script, seed, false)
}

fn tally_with(script: &Plan, seed: u64, weather_off: bool) -> Tally {
    let mut p = params();
    if weather_off {
        p.weather.fire_base = 0.0;
        p.weather.drought_base = 0.0;
    }
    let mut sim = sim_engine::init(p, seed).unwrap();
    let b0 = sim.params.nature.budget_b0;
    let mut t = Tally {
        farm_low: 1.0,
        ..Tally::default()
    };
    let mut done = false;
    let mut half = None;
    sim_engine::run(&mut sim, 40, script, |s, o| {
        if done {
            return;
        }
        if half.is_none() && s.state.nature.cumulative.0 >= 0.5 * b0 {
            half = Some(s.state.tick);
            t.half_turn = s.state.tick as f64 / 3.0;
        }
        for e in &o.events {
            match e {
                Event::FireStarted { .. } => t.fires += 1,
                Event::DroughtStarted { .. } => t.droughts += 1,
                Event::ForestBurned { .. } => t.burnt += 1,
                _ => {}
            }
        }
        t.farm_low = t.farm_low.min(s.state.indicators.farm_health);
        if s.state.nature.cumulative.0 >= b0 {
            t.exhausted_turn = s.state.tick as f64 / 3.0;
            done = true;
        }
    })
    .unwrap();
    t
}

/// Prints baselines A-D over seeds 1..=20 (until the budget is exhausted, where the game ends).
/// Run with `cargo test -p sim-engine --test weather weather_baselines -- --nocapture`.
#[test]
fn weather_baselines_over_twenty_seeds() {
    let scripts = [
        ("A", Plan::default()),
        ("B", Plan::from_json(SCENARIO_B).unwrap()),
        (
            "C",
            Plan::from_json(include_str!("../content/scenarios/baseline_c_neglect.json")).unwrap(),
        ),
        (
            "D",
            Plan::from_json(include_str!(
                "../content/scenarios/baseline_d_carbon_only.json"
            ))
            .unwrap(),
        ),
    ];
    for (name, plan) in &scripts {
        let ts: Vec<Tally> = (1..=20u64).map(|seed| tally(plan, seed)).collect();
        let n = ts.len() as f64;
        let mean = |f: &dyn Fn(&Tally) -> f64| ts.iter().map(f).sum::<f64>() / n;
        println!(
            "{name}: half turn {:.1}, exhausted turn {:.1}, fires {:.2}, droughts {:.2}, burnt tiles {:.2}, farm health low {:.2}; runs with a fire {} of 20, with a drought {} of 20",
            mean(&|t| t.half_turn),
            mean(&|t| t.exhausted_turn),
            mean(&|t| t.fires as f64),
            mean(&|t| t.droughts as f64),
            mean(&|t| t.burnt as f64),
            mean(&|t| t.farm_low),
            ts.iter().filter(|t| t.fires > 0).count(),
            ts.iter().filter(|t| t.droughts > 0).count(),
        );
        let off: Vec<Tally> = (1..=20u64)
            .map(|seed| tally_with(plan, seed, true))
            .collect();
        println!(
            "{name} with the weather switched off: exhausted turn {:.1}, farm health low {:.2}",
            off.iter().map(|t| t.exhausted_turn).sum::<f64>() / n,
            off.iter().map(|t| t.farm_low).sum::<f64>() / n,
        );
        assert!(ts.iter().all(|t| t.farm_low > 0.0));
    }
}
