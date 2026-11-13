//! System-level sign probes for the weather, wellbeing, labour and housing mechanisms.
use crate::common::{params, states, try_after};
use sim_model::causal::Node;
use sim_model::sign_probe::{Probe, Report};
use sim_types::*;
use std::sync::OnceLock;

fn p() -> &'static sim_params::Params {
    static P: OnceLock<sim_params::Params> = OnceLock::new();
    P.get_or_init(params)
}

fn mech(m: MechanismId) -> Node {
    Node::Mechanism(m)
}

/// The sample states by index (see `common::states`): 0..3 mid-run, 3 hot, 4 over budget,
/// 5 community, 6 enclosed, 7.. weather seeds, then housing, arrears, floor, homeless.
fn pick(idx: &[usize]) -> Vec<State> {
    let all = states();
    idx.iter().map(|i| all[*i].clone()).collect()
}

fn n_states() -> usize {
    states().len()
}

fn standing_forest(s: &State) -> Vec<Tile> {
    s.map
        .tiles()
        .filter(|t| {
            s.map.kind_at(*t) == Some(TileKind::Forest)
                && !s.weather.fires.iter().any(|f| f.tile == *t)
                && !s.weather.regrowing.iter().any(|g| g.tile == *t)
        })
        .collect()
}

/// Nudge for `fire -> fire`: one more burning tile, on the standing forest tile with the fewest
/// standing forest neighbours (so a burning tile with nobody to spread to is possible).
fn more_fire_loneliest(s: &mut State) {
    let st = standing_forest(s);
    let near = |a: &Tile| {
        st.iter()
            .filter(|b| a.x.abs_diff(b.x) + a.y.abs_diff(b.y) == 1)
            .count()
    };
    if let Some(t) = st.iter().min_by_key(|t| near(t)).copied() {
        s.weather.fires.push(Fire {
            tile: t,
            months_left: 3,
        });
    }
}

// ---- fire_burns_forest (Weather system) ----

/// Nudge: one more burning tile (a fire on the first forest tile not already burning). Read: the
/// forest stock after `weather::run`.
fn more_fire(s: &mut State) {
    let burning: Vec<Tile> = s.weather.fires.iter().map(|f| f.tile).collect();
    if let Some(t) = s
        .map
        .tiles()
        .find(|t| s.map.kind_at(*t) == Some(TileKind::Forest) && !burning.contains(t))
    {
        s.weather.fires.push(Fire {
            tile: t,
            months_left: 3,
        });
    }
}

fn forest_after_weather(s: &State) -> f64 {
    try_after(SystemId::Weather, s, p()).map_or(f64::NAN, |n| n.nature.forest.0)
}

fn probe_fire_burns_forest(r: &mut Report) {
    let mut bases = pick(&[0, 1, 2, 3, 4, 7, 8, 9, 10]);
    // A state with a standing fire already.
    let mut burning = bases[1].clone();
    burning.weather.stress = 0.5;
    more_fire(&mut burning);
    bases.push(burning);
    let probe = Probe {
        node: mech(MechanismId::FireBurnsForest),
        bases,
        nudges: vec![
            (QuantityId::Fire, more_fire as fn(&mut State)),
            // Nudge: the standing forest stock rises.
            (QuantityId::ForestStock, |s: &mut State| {
                s.nature.forest = Units(s.nature.forest.0 * 1.1 + 1.0)
            }),
        ],
        reads: vec![(
            QuantityId::ForestStock,
            forest_after_weather as fn(&State) -> f64,
        )],
    };
    r.check(&probe);
}

// ---- drought_onset (Drought -> Drought) and fire_ignition (the chance) ----

/// Nudge: a drought in progress gets more severe (and lasts at least two more months so it does
/// not end this month). Read: its severity after `weather::run` (0 when none).
fn drought_after(s: &State) -> f64 {
    try_after(SystemId::Weather, s, p())
        .map_or(f64::NAN, |n| n.weather.drought.map_or(0.0, |d| d.severity))
}

fn more_severe(s: &mut State) {
    if let Some(d) = s.weather.drought.as_mut() {
        d.severity += 0.2;
    }
}

fn probe_drought_onset(r: &mut Report) {
    let mut bases = pick(&[3, 4, 5, 6]);
    for b in &mut bases {
        b.weather.drought = Some(Drought {
            severity: 0.4,
            months_left: 4,
        });
    }
    let probe = Probe {
        node: mech(MechanismId::DroughtOnset),
        bases,
        nudges: vec![(QuantityId::Drought, more_severe as fn(&mut State))],
        reads: vec![(QuantityId::Drought, drought_after as fn(&State) -> f64)],
    };
    r.check(&probe);
}

/// Read, for every pair: the odds that fire reaches the standing forest, ignition plus spread
/// summed over every standing forest tile (`weather::tile_fire_risk`, which calls `ignition_p`
/// and `spread_p`), so a burning tile's two effects (fewer standing tiles to ignite, more spread)
/// both show. This is the chance, not a drawn outcome.
fn fire_reach(s: &State) -> f64 {
    s.map
        .tiles()
        .filter_map(|t| sim_model::weather::tile_fire_risk(s, p(), t))
        .sum()
}

fn probe_fire_ignition(r: &mut Report) {
    let n = n_states();
    let mut bases = pick(&[3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, n - 4, 1]);
    for b in &mut bases {
        b.weather.drought = Some(Drought {
            severity: 0.3,
            months_left: 3,
        });
        b.weather.fires.clear();
        b.weather.regrowing.clear();
        b.nature.cumulative = Tonnes(700.0);
        b.nature.commons = Units(b.nature.commons.0.min(300.0));
    }
    // Variants where only two far-apart forest tiles still stand: the fire has nobody to spread to.
    let mut sparse = bases.clone();
    for b in &mut sparse {
        let all: Vec<Tile> = standing_forest(b);
        if all.len() > 2 {
            let (first, last) = (all[0], all[all.len() - 1]);
            for t in all.iter().filter(|t| **t != first && **t != last) {
                b.weather.regrowing.push(Regrowing {
                    tile: *t,
                    months_left: 5,
                });
            }
        }
    }
    bases.extend(sparse);
    let probe = Probe {
        node: mech(MechanismId::FireIgnition),
        bases,
        nudges: vec![
            // Climate stress rises through the cumulative emissions it is computed from.
            (
                QuantityId::ClimateStress,
                (|s: &mut State| s.nature.cumulative = Tonnes(s.nature.cumulative.0 + 30.0))
                    as fn(&mut State),
            ),
            // A drought in progress is more severe.
            (QuantityId::Drought, |s: &mut State| {
                if let Some(d) = s.weather.drought.as_mut() {
                    d.severity += 0.3;
                }
            }),
            // The commons are healthier.
            (QuantityId::CommonsStock, |s: &mut State| {
                s.nature.commons = Units(s.nature.commons.0 * 1.3 + 5.0)
            }),
            // More resilience spending.
            (QuantityId::ResilienceSpending, |s: &mut State| {
                s.policy.resilience_spending = Cents(s.policy.resilience_spending.0 + 200_000)
            }),
            // One more burning tile.
            (QuantityId::Fire, more_fire_loneliest),
        ],
        reads: vec![(QuantityId::Fire, fire_reach as fn(&State) -> f64)],
    };
    r.check(&probe);
}

// ---- wellbeing_score (Wellbeing system) ----

/// Read for every wellbeing pair: the sum of every person's wellbeing after `wellbeing::run`
/// (the town aggregate is the mean of persons, so the sum moves the same way).
fn wellbeing_sum(s: &State) -> f64 {
    try_after(SystemId::Wellbeing, s, p())
        .map_or(f64::NAN, |n| n.persons.iter().map(|x| x.wellbeing.0).sum())
}

fn nudge_consumption(s: &mut State) {
    // Consumption: every household spent 20 per cent (and a dollar) more last month.
    for h in &mut s.households {
        h.consumption = Cents(h.consumption.0 * 12 / 10 + 100);
    }
}

fn nudge_unemployment(s: &mut State) {
    // Unemployment: the first employed person in the labour force loses the job.
    if let Some(x) = s
        .persons
        .iter_mut()
        .find(|x| x.in_labour_force && x.employed())
    {
        x.job = None;
        x.jg = false;
    }
}

fn fill_vacant_home(s: &mut State) {
    // Occupancy: the first homeless household is in the first vacant home at the start.
    let vacant = s.homes.iter().position(|h| h.household.is_none());
    let homeless = s.households.iter().position(|h| h.home.is_none());
    if let (Some(v), Some(h)) = (vacant, homeless) {
        s.homes[v].household = Some(s.households[h].id);
        s.households[h].home = Some(s.homes[v].id);
    }
}

fn nudge_savings(s: &mut State) {
    // Savings: every household holds 50 per cent (and 50 dollars) more.
    for h in &mut s.households {
        h.savings = Cents(h.savings.0 * 3 / 2 + 5_000);
    }
}

fn fire_beside_home(s: &mut State) {
    // Fire: a tile burns beside the first home (smoke).
    if let Some(h) = s.homes.first() {
        let t = h.tile;
        s.weather.fires.push(Fire {
            tile: t,
            months_left: 3,
        });
    }
}

fn probe_wellbeing(r: &mut Report) {
    let n = n_states();
    let mut bases = pick(&[0, 1, 2, 3, 4, 5, 7, 8, n - 4, n - 1]);
    // Pull every person's current wellbeing off the bounds so the smoothed target is not pinned.
    for b in &mut bases {
        for x in &mut b.persons {
            x.wellbeing = Rate(0.5);
        }
    }
    let probe = Probe {
        node: mech(MechanismId::WellbeingScore),
        bases,
        nudges: vec![
            (QuantityId::Consumption, nudge_consumption as fn(&mut State)),
            (QuantityId::Unemployment, nudge_unemployment),
            (QuantityId::Occupancy, fill_vacant_home),
            // Forest stock: 20 per cent more standing forest.
            (QuantityId::ForestStock, |s: &mut State| {
                s.nature.forest = Units(s.nature.forest.0 * 1.2)
            }),
            // Gross emissions: five tonnes more this month.
            (QuantityId::GrossEmissions, |s: &mut State| {
                s.nature.gross_emissions = Tonnes(s.nature.gross_emissions.0 + 5.0)
            }),
            (QuantityId::Savings, nudge_savings),
            (QuantityId::Fire, fire_beside_home),
        ],
        reads: vec![(QuantityId::Wellbeing, wellbeing_sum as fn(&State) -> f64)],
    };
    r.check(&probe);
}

// ---- last_disposable_record (Wellbeing system copies the income record) ----

/// Read: the sum of every household's recorded disposable income after `wellbeing::run`.
fn last_disposable_sum(s: &State) -> f64 {
    try_after(SystemId::Wellbeing, s, p()).map_or(f64::NAN, |n| {
        n.households
            .iter()
            .map(|h| h.last_disposable.0 as f64)
            .sum()
    })
}

fn probe_last_disposable(r: &mut Report) {
    let probe = Probe {
        node: mech(MechanismId::LastDisposableRecord),
        bases: pick(&[0, 1, 2, 3]),
        nudges: vec![
            // Wage: wages in the household's income record rise.
            (
                QuantityId::Wage,
                (|s: &mut State| {
                    for h in &mut s.households {
                        h.income.wages = Cents(h.income.wages.0 + 100_000);
                    }
                }) as fn(&mut State),
            ),
            // Capital income: dividends in the income record rise.
            (QuantityId::CapitalIncome, |s: &mut State| {
                for h in &mut s.households {
                    h.income.dividends = Cents(h.income.dividends.0 + 100_000);
                }
            }),
            // Rent: rent paid in the income record rises.
            (QuantityId::Rent, |s: &mut State| {
                for h in &mut s.households {
                    h.income.rent_paid = Cents(h.income.rent_paid.0 + 100_000);
                }
            }),
        ],
        reads: vec![(
            QuantityId::LastDisposable,
            last_disposable_sum as fn(&State) -> f64,
        )],
    };
    r.check(&probe);
}

// ---- unemployment_rate (sim_metrics, no system) and labour_matching (Labour system) ----

fn unemployment_rate(s: &State) -> f64 {
    sim_metrics::unemployment(s).2 .0
}

fn take_a_job(s: &mut State) {
    // Workers: the first jobless person in the labour force takes a job at the first active firm.
    let firm = s.firms.iter().find(|f| f.active).map(|f| f.id);
    if let (Some(f), Some(x)) = (
        firm,
        s.persons
            .iter_mut()
            .find(|x| x.in_labour_force && !x.employed()),
    ) {
        x.job = Some(f);
    }
}

fn probe_unemployment_rate(r: &mut Report) {
    let probe = Probe {
        node: mech(MechanismId::UnemploymentRate),
        bases: pick(&[0, 1, 2, 3]),
        nudges: vec![(QuantityId::Workers, take_a_job as fn(&mut State))],
        reads: vec![(
            QuantityId::Unemployment,
            unemployment_rate as fn(&State) -> f64,
        )],
    };
    r.check(&probe);
}

/// Read: persons with a job after `labour::run` (the matching fills vacancies from the jobless).
fn employed_after_labour(s: &State) -> f64 {
    try_after(SystemId::Labour, s, p()).map_or(f64::NAN, |n| {
        n.persons.iter().filter(|x| x.employed()).count() as f64
    })
}

fn more_expected_sales(s: &mut State) {
    // Vacancies are posted by the hire decision from expected sales: raise every active firm's
    // expected sales so the gap between wanted and present workers (the vacancies) grows. The
    // pair is therefore measured as hire plus matching together; the hire part is `+` too.
    for f in s.firms.iter_mut().filter(|f| f.active) {
        f.expected_sales = Units(f.expected_sales.0 * 3.0 + 50.0);
    }
}

fn probe_labour_matching(r: &mut Report) {
    let probe = Probe {
        node: mech(MechanismId::LabourMatching),
        bases: pick(&[0, 1, 2, 3, 4]),
        nudges: vec![(QuantityId::Vacancies, more_expected_sales as fn(&mut State))],
        reads: vec![(
            QuantityId::Workers,
            employed_after_labour as fn(&State) -> f64,
        )],
    };
    r.check(&probe);
}

// ---- housing_rent and housing_moves (Goods system, `housing`) ----

fn rent_after(s: &State) -> f64 {
    try_after(SystemId::Goods, s, p()).map_or(f64::NAN, |n| n.housing.rent_private.0 as f64)
}

fn occupancy_after(s: &State) -> f64 {
    try_after(SystemId::Goods, s, p()).map_or(f64::NAN, |n| n.housing.occupancy.0)
}

fn set_housing(s: &mut State, rent: i64, disposable: i64) {
    s.housing.rent_private = Cents(rent);
    for h in &mut s.households {
        h.last_disposable = Cents(disposable);
        h.arrears_months = 0;
    }
}

/// Empty every home but the first and put every other household out, with the given disposable
/// income and free-home rent.
fn homeless(s: &mut State, disposable: i64, rent: i64) {
    for h in s.households.iter_mut().skip(1) {
        h.home = None;
        h.last_disposable = Cents(disposable);
    }
    for (i, home) in s.homes.iter_mut().enumerate() {
        if i > 0 {
            home.household = None;
            home.rent = Cents(rent);
        }
    }
}

/// Make household 1 the tenant of home 1 (owned by household 0), so the town has a renter whose
/// burden (rent over disposable income) the rent rule reads. Everyone else keeps their place.
fn with_a_renter(s: &mut State) {
    let (owner, tenant, home) = (s.households[0].id, s.households[1].id, s.homes[1].id);
    if let Some(old) = s.homes[1].household {
        s.households[old.idx()].home = None;
    }
    if let Some(old) = s.households[1].home {
        s.homes[old.idx()].household = None;
    }
    s.homes[1].owner = HomeOwner::Household(owner);
    s.homes[1].household = Some(tenant);
    s.households[1].home = Some(home);
}

/// States for the rent probes: a rich renter (burden far below the cap), a renter whose burden
/// sits just under the cap (a 0.1 per cent higher rent crosses it), a renter over the cap, and the
/// same with one household out of a home it cannot afford (so occupancy is below the trigger
/// until the nudge puts it in).
fn housing_bases() -> Vec<State> {
    let n = n_states();
    let mut out = Vec::new();
    let cap = p().goods.rent_burden_cap;
    for k in [n - 4, 1, 2] {
        let mut base = pick(&[k]).remove(0);
        with_a_renter(&mut base);
        for disposable in [
            50_000_000,
            (90_000.0 / (cap * 0.9995)) as i64,
            (90_000.0 / (cap * 1.3)) as i64,
        ] {
            let mut a = base.clone();
            // Every renter has this disposable income, so the mean burden is rent over it.
            set_housing(&mut a, 90_000, disposable);
            out.push(a.clone());
            // A home stands empty, at a rent every household is too poor for, so occupancy is
            // below the trigger until the nudge puts someone in.
            let mut c = a;
            if let Some(old) = c.homes[2].household {
                c.households[old.idx()].home = None;
            }
            c.homes[2].household = None;
            // Public, so no household can take it back at no rent as its owner.
            c.homes[2].owner = HomeOwner::Public;
            c.homes[2].rent = Cents(90_000_000);
            out.push(c);
        }
    }
    out
}

fn probe_housing_rent(r: &mut Report) {
    let probe = Probe {
        node: mech(MechanismId::HousingRent),
        bases: housing_bases(),
        nudges: vec![
            (QuantityId::Occupancy, fill_vacant_home as fn(&mut State)),
            // Last disposable income: every household has 50 per cent more.
            (QuantityId::LastDisposable, |s: &mut State| {
                for h in &mut s.households {
                    h.last_disposable = Cents(h.last_disposable.0 * 3 / 2);
                }
            }),
            // Rent: the private rent is 0.1 per cent higher.
            (QuantityId::Rent, |s: &mut State| {
                s.housing.rent_private = Cents(s.housing.rent_private.0 * 1001 / 1000)
            }),
        ],
        reads: vec![(QuantityId::Rent, rent_after as fn(&State) -> f64)],
    };
    r.check(&probe);
}

fn probe_housing_moves(r: &mut Report) {
    let n = n_states();
    // Homeless households who can just afford a free home at the base rent, and some who cannot yet.
    let mut bases = Vec::new();
    for k in [n - 1, n - 4] {
        let mut b = pick(&[k]).remove(0);
        homeless(&mut b, 300_000, 90_000);
        bases.push(b.clone());
        homeless(&mut b, 100_000, 90_000);
        bases.push(b);
    }
    let probe = Probe {
        node: mech(MechanismId::HousingMoves),
        bases,
        nudges: vec![
            // Rent: every vacant home's rent doubles.
            (
                QuantityId::Rent,
                (|s: &mut State| {
                    for h in s.homes.iter_mut().filter(|h| h.household.is_none()) {
                        h.rent = Cents(h.rent.0 * 2);
                    }
                }) as fn(&mut State),
            ),
            // Last disposable income: every household has 3 times as much.
            (QuantityId::LastDisposable, |s: &mut State| {
                for h in &mut s.households {
                    h.last_disposable = Cents(h.last_disposable.0 * 3);
                }
            }),
            // Savings: every household holds twice as much (moves do not read it).
            (QuantityId::Savings, |s: &mut State| {
                for h in &mut s.households {
                    h.savings = Cents(h.savings.0 * 2 + 100_000);
                }
            }),
        ],
        reads: vec![(QuantityId::Occupancy, occupancy_after as fn(&State) -> f64)],
    };
    r.check(&probe);
}

pub fn probe_all(r: &mut Report) {
    probe_wellbeing(r);
    probe_last_disposable(r);
    probe_unemployment_rate(r);
    probe_labour_matching(r);
    probe_housing_rent(r);
    probe_housing_moves(r);
    probe_fire_burns_forest(r);
    probe_drought_onset(r);
    probe_fire_ignition(r);
}
