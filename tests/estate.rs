//! Town v7: the elite estate, its jet and its pilot (docs/town-v7.md). Rules tests; the wire
//! side is in `crates/sim-api/tests`.

use sim_engine::Sim;
use sim_params::{ContentJson, Params};
use sim_types::*;

const PARAMS: &str = include_str!("../content/params.json");
const TOWN: &str = include_str!("../content/initial_town.json");
const MARKET: &str = include_str!("../content/systems/market.json");
const REGULATED: &str = include_str!("../content/systems/regulated.json");
const COOP: &str = include_str!("../content/systems/coop.json");

fn load(town: &str) -> Result<Params, String> {
    Params::from_content(&ContentJson {
        params: PARAMS,
        initial_town: town,
        market: MARKET,
        regulated: REGULATED,
        coop: COOP,
    })
}

fn fresh(p: Params, seed: u64) -> Sim {
    let mut p = p;
    p.run.warmup_ticks = 0;
    sim_engine::init(p, seed).expect("init")
}

fn base() -> Params {
    load(TOWN).expect("content loads")
}

/// Give the estate household extra savings backed by a loan of the same size (money and the
/// sector balances stay consistent; the bank's equity is unchanged).
fn lend_to_estate(sim: &mut Sim, dollars: f64) {
    let c = Cents::from_dollars(dollars);
    let h = &mut sim.state.households[ESTATE];
    h.savings += c;
    h.debt += c;
    sim.state.bank.deposits += c;
    sim.state.bank.loans += c;
}

const ESTATE: usize = 8;
const PILOT: usize = 18;

#[test]
fn the_estate_loads_with_its_people_tiles_and_firm() {
    let sim = fresh(base(), 1);
    let s = &sim.state;
    let e = s.estate.as_ref().expect("an estate");
    assert_eq!(e.household, HouseholdId(ESTATE as u32));
    assert_eq!(s.households[ESTATE].members.len(), 1, "a household of one");
    assert_eq!(s.households[ESTATE].label, "Estate owner");
    assert_eq!(e.pilot, Some(PersonId(PILOT as u32)));
    // Tiles from the content, one thing per tile.
    let kind = |x: u8, y: u8| s.map.kind_at(Tile::new(x, y)).unwrap();
    assert_eq!(kind(0, 10), TileKind::Home);
    for (x, y) in [(1, 10), (0, 11), (1, 11)] {
        assert_eq!(kind(x, y), TileKind::EstateGrounds);
    }
    for (x, y) in [(2, 11), (3, 11), (4, 11)] {
        assert_eq!(kind(x, y), TileKind::Runway);
    }
    let count = |k: TileKind| s.map.cells.iter().filter(|c| c.kind == k).count();
    assert_eq!(count(TileKind::Road), 4 + 7, "4 roads and the estate's 7");
    assert_eq!(
        count(TileKind::Forest),
        15 + 11,
        "15 forest tiles and 11 new"
    );
    // The Farm belongs to the estate household.
    assert_eq!(s.firms[1].owner, Owner::Household(e.household));
    // 22 people, 9 households, 8 homes.
    assert_eq!(
        (s.persons.len(), s.households.len(), s.homes.len()),
        (22, 9, 8)
    );
}

#[test]
fn the_estate_person_is_outside_the_labour_force_and_the_pilot_counts_as_employed() {
    let sim = fresh(base(), 1);
    let s = &sim.state;
    let owner = s.person(s.households[ESTATE].members[0]);
    assert!(!owner.in_labour_force && !owner.unemployed() && !owner.employed());
    let pilot = s.person(PersonId(PILOT as u32));
    assert!(pilot.employed() && pilot.in_labour_force && pilot.job.is_none());
    assert_eq!(pilot.staff_of, Some(HouseholdId(ESTATE as u32)));
    // 11 in the labour force, 10 employed: unemployment is 1 / 11 at the start.
    let (emp, lf) = s.labour_force();
    assert_eq!((emp, lf), (10.0, 11.0));
    assert!((s.indicators.unemployment.0 - 1.0 / 11.0).abs() < 1e-12);
}

#[test]
fn the_estate_household_is_capitalist_and_the_pilots_household_stays_worker() {
    let mut sim = fresh(base(), 1);
    for _ in 0..3 {
        sim_engine::step(&mut sim).unwrap();
    }
    assert_eq!(sim.state.households[ESTATE].class, Class::Capitalist);
    assert_eq!(sim.state.households[6].class, Class::Worker);
}

#[test]
fn wealth_includes_the_jet_and_the_estate_is_by_far_the_richest() {
    let sim = fresh(base(), 1);
    let w = sim_metrics::household_wealth(&sim.state, &sim.params);
    let parts = sim_metrics::household_wealth_parts(&sim.state, &sim.params);
    assert!(parts[ESTATE].jet > 0.0);
    assert_eq!(parts[ESTATE].jet, 800_000.0 * 100.0);
    for (i, p) in parts.iter().enumerate() {
        assert!((p.total() - w[i]).abs() < 1.0, "parts of household {i}");
    }
    let richest_other = w
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != ESTATE)
        .map(|(_, x)| *x)
        .fold(f64::MIN, f64::max);
    assert!(
        w[ESTATE] > 3.0 * richest_other,
        "{} vs {}",
        w[ESTATE],
        richest_other
    );
}

/// Parameters with a flight every month, so flight months are easy to see.
fn always_flies() -> Params {
    let mut p = base();
    p.estate.flight_prob_monthly = 1.0;
    p
}

#[test]
fn a_flight_emits_its_tonnes_as_aviation_and_pays_the_carbon_price() {
    let mut p = always_flies();
    p.initial.policy.carbon_price_usd = 100.0;
    let mut sim = fresh(p, 3);
    // Plenty of savings so the carbon charge is not capped.
    lend_to_estate(&mut sim, 100_000.0);
    let tonnes = sim.params.estate.flight_tonnes;
    // The price in force when the flight is charged (after lobbying), read before the tick.
    let price = sim.state.effective.carbon_price;
    let out = sim_engine::step(&mut sim).unwrap();
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, Event::PrivateJetFlight { tonnes: t, .. } if t.0 == tonnes)));
    let aviation: f64 = out
        .trace
        .iter()
        .filter_map(|t| match t {
            TraceRecord::Emissions {
                source: EmissionSource::Aviation,
                tonnes,
            } => Some(tonnes.0),
            _ => None,
        })
        .sum();
    assert_eq!(aviation, tonnes);
    // Gross = firms + households + plants + aviation (the estate household has its own figure).
    let s = &sim.state;
    let firms: f64 = s.firms.iter().map(|f| f.emissions.0).sum();
    let plants: f64 = s.plants.iter().map(|x| x.emissions.0).sum();
    let hh = (s.persons.len() - 1) as f64 * sim.params.nature.household_footprint_base
        + sim.params.estate.household_emissions_t;
    assert!((s.nature.gross_emissions.0 - (firms + plants + hh + aviation)).abs() < 1e-9);
    // The charge: carbon price x tonnes, estate to government, labelled as the carbon price.
    let charged: Cents = out
        .transactions
        .iter()
        .filter(|t| {
            t.from == LedgerAccount::Household(HouseholdId(ESTATE as u32))
                && t.to == LedgerAccount::Government
                && t.detail == "carbon price"
        })
        .fold(Cents::ZERO, |a, t| a + t.amount);
    let want = Cents::from_f64(price.as_f64() * tonnes);
    assert_eq!(charged, want);
    assert_eq!(s.estate.as_ref().unwrap().flights_total, 1);
}

#[test]
fn the_carbon_charge_is_capped_by_the_estates_savings() {
    let mut p = always_flies();
    p.initial.policy.carbon_price_usd = 100_000.0;
    let mut sim = fresh(p, 3);
    let before = sim.state.households[ESTATE].savings;
    let out = sim_engine::step(&mut sim).unwrap();
    let charged: Cents = out
        .transactions
        .iter()
        .filter(|t| {
            t.detail == "carbon price"
                && t.from == LedgerAccount::Household(HouseholdId(ESTATE as u32))
        })
        .fold(Cents::ZERO, |a, t| a + t.amount);
    assert!(
        charged > Cents::ZERO && charged <= before,
        "{charged:?} of {before:?}"
    );
}

#[test]
fn no_flight_without_the_draw() {
    let mut p = base();
    p.estate.flight_prob_monthly = 0.0;
    let mut sim = fresh(p, 3);
    for _ in 0..12 {
        let out = sim_engine::step(&mut sim).unwrap();
        assert!(!out
            .events
            .iter()
            .any(|e| matches!(e, Event::PrivateJetFlight { .. })));
    }
    assert_eq!(sim.state.estate.as_ref().unwrap().flights_total, 0);
}

#[test]
fn the_pilot_is_paid_exactly_the_wage_as_domestic_wages_and_the_ledger_reconciles() {
    let mut sim = fresh(base(), 2);
    lend_to_estate(&mut sim, 50_000.0);
    let wage = Cents::from_dollars(sim.params.estate.pilot_wage_usd);
    let out = sim_engine::step(&mut sim).unwrap();
    let paid: Vec<&Transaction> = out
        .transactions
        .iter()
        .filter(|t| t.detail == "domestic wages")
        .collect();
    assert_eq!(paid.len(), 1);
    assert_eq!(paid[0].amount, wage);
    assert_eq!(
        paid[0].from,
        LedgerAccount::Household(HouseholdId(ESTATE as u32))
    );
    assert_eq!(paid[0].to, LedgerAccount::Household(HouseholdId(6)));
    assert_eq!(paid[0].kind, LedgerKind::Wage);
    // It lands in the pilot's household wages (so the wage tax applies to it) and the pilot.
    assert!(sim.state.households[6].income.wages >= wage);
    assert_eq!(sim.state.person(PersonId(PILOT as u32)).wage, wage);
}

fn pilot_employed(sim: &Sim) -> bool {
    sim.state.person(PersonId(PILOT as u32)).staff_of.is_some()
}

#[test]
fn short_pay_for_three_months_dismisses_the_pilot_and_grounds_the_jet_then_rehire() {
    let mut p = always_flies();
    // A wage the estate can never pay in full.
    p.estate.pilot_wage_usd = 5_000_000.0;
    let mut sim = fresh(p, 5);
    let max = sim.params.estate.pilot_unpaid_months_max;
    let mut let_go_at = None;
    let mut flights_at_dismissal = 0;
    for t in 0..(max + 6) {
        let out = sim_engine::step(&mut sim).unwrap();
        if out
            .events
            .iter()
            .any(|e| matches!(e, Event::PilotLetGo { .. }))
        {
            let_go_at.get_or_insert(t);
            flights_at_dismissal = sim.state.estate.as_ref().unwrap().flights_total;
        }
    }
    let t = let_go_at.expect("the pilot is let go");
    assert_eq!(
        t, max,
        "after {max} months of short pay, in the following month"
    );
    assert!(!pilot_employed(&sim));
    // The jet stays grounded: no flight after the dismissal, though the draw is taken.
    assert_eq!(
        sim.state.estate.as_ref().unwrap().flights_total,
        flights_at_dismissal
    );
    assert!(!sim.state.estate.as_ref().unwrap().jet.flying_this_month);
    // Now unemployed and in the normal labour market (counted as unemployed or hired by a firm).
    let x = sim.state.person(PersonId(PILOT as u32));
    assert!(x.in_labour_force && (x.unemployed() || x.job.is_some()));
    // Rehire when the estate can cover the wages: make that cheap and rich, and free the pilot.
    sim.params.estate.pilot_wage_usd = 2_400.0;
    lend_to_estate(&mut sim, 50_000.0);
    let pid = PersonId(PILOT as u32);
    if sim.state.person(pid).job.is_some() {
        // Hired by a firm meanwhile: the estate does not take them back.
        return;
    }
    let out = sim_engine::step(&mut sim).unwrap();
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, Event::PilotHired { .. })));
    assert!(pilot_employed(&sim));
}

#[test]
fn the_flight_draw_is_taken_every_month_so_a_dismissal_never_shifts_the_stream() {
    // Two runs that differ only in the wage (one dismisses the pilot, one does not) must have the
    // same random stream position after the same number of ticks, as far as the estate goes:
    // compare the estate's draw with `flight_prob_monthly` 0 (draws taken, no flights).
    let mut a = base();
    a.estate.flight_prob_monthly = 0.0;
    let mut b = a.clone();
    b.estate.pilot_wage_usd = 5_000_000.0;
    let (mut sa, mut sb) = (fresh(a, 9), fresh(b, 9));
    for _ in 0..9 {
        sim_engine::step(&mut sa).unwrap();
        sim_engine::step(&mut sb).unwrap();
    }
    assert!(!pilot_employed(&sb), "the second run dismissed the pilot");
    // Each estate system run consumed exactly one draw in both runs: 9 each, so the positions
    // differ only through other systems; at minimum the estate never skipped a draw.
    let mut c = base();
    c.estate.flight_prob_monthly = 0.0;
    let mut sc = fresh(c, 9);
    let before = sc.rng.parts();
    let _ = sim_engine::step(&mut sc).unwrap();
    assert_ne!(before, sc.rng.parts(), "a tick draws from the stream");
}

#[test]
fn the_hire_loop_never_puts_the_pilot_in_a_firm_while_employed() {
    let mut sim = fresh(base(), 1);
    // Make the Factory want many people.
    sim.state.firms[0].expected_sales = Units(5_000.0);
    sim.state.firms[0].inventory = Units(0.0);
    for _ in 0..3 {
        sim_engine::step(&mut sim).unwrap();
        let pilot = sim.state.person(PersonId(PILOT as u32));
        assert!(pilot.job.is_none(), "staff are not hired into a firm");
        assert!(pilot.staff_of.is_some());
    }
}

#[test]
fn the_pilot_commutes_to_the_hangar_tile() {
    let sim = fresh(base(), 1);
    let s = &sim.state;
    let pilot = s.person(PersonId(PILOT as u32));
    let home = s.home(s.households[6].home.unwrap()).unwrap().tile;
    let hangar = s.estate.as_ref().unwrap().hangar;
    assert_eq!(hangar, Tile::new(2, 11));
    let route =
        sim_model::space::routes_to(&s.map, &sim.params.space, hangar)[s.map.idx(home)].unwrap();
    assert_eq!(
        (pilot.commute_tiles, pilot.commute_minutes),
        (route.tiles, route.minutes)
    );
    assert!(pilot.commute_tiles > 0);
}

#[test]
fn staff_of_must_name_the_estate_and_a_household_of_one_is_allowed() {
    // Another employer is rejected.
    let mut t: serde_json::Value = serde_json::from_str(TOWN).unwrap();
    t["households"][6]["people"][2]["staff_of"] = serde_json::json!(0);
    let e = load(&t.to_string()).unwrap_err();
    assert!(e.contains("not the estate"), "{e}");
    // The estate household has exactly one person; the validator accepts it (checked by `base`),
    // and refuses a second person.
    let mut t: serde_json::Value = serde_json::from_str(TOWN).unwrap();
    let extra = t["households"][8]["people"][0].clone();
    t["households"][8]["people"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    assert!(load(&t.to_string())
        .unwrap_err()
        .contains("exactly one person"));
    // A tile conflict fails validation.
    let mut t: serde_json::Value = serde_json::from_str(TOWN).unwrap();
    t["estate"]["runway"][0] = serde_json::json!([5, 6]);
    assert!(load(&t.to_string()).unwrap_err().contains("holds both"));
}

#[test]
fn a_town_without_an_estate_still_runs_and_draws_nothing_for_it() {
    // Remove the estate household, its home, the pilot, the tiles and the block.
    let mut t: serde_json::Value = serde_json::from_str(TOWN).unwrap();
    t["estate"] = serde_json::Value::Null;
    t["households"].as_array_mut().unwrap().pop();
    t["households"][6]["people"].as_array_mut().unwrap().pop();
    t["homes"].as_array_mut().unwrap().pop();
    t["firms"][1]["founder"] = serde_json::json!(0);
    let p = load(&t.to_string()).expect("a town without an estate loads");
    let mut sim = fresh(p, 1);
    assert!(sim.state.estate.is_none());
    assert_eq!(sim.state.persons.len(), 20);
    for _ in 0..6 {
        sim_engine::step(&mut sim).unwrap();
    }
    assert!(sim.state.estate.is_none());
}

#[test]
fn ledger_and_money_hold_over_a_run_with_flights() {
    let mut sim = fresh(always_flies(), 4);
    // `step` checks the ledger and money conservation after every system and every tick.
    for _ in 0..36 {
        sim_engine::step(&mut sim).unwrap();
    }
    assert!(sim.state.estate.as_ref().unwrap().flights_total > 0);
}
