//! S1 labour market: named jobs. Each unemployed adult applies to firms with vacancies,
//! the cheapest first (ties by skill). Firing removes named people. Wages, skill and the
//! job guarantee follow.
//!
//! The firm's three labour decisions (hire, fire, wage) are pure `decide_*` functions over a
//! read-only view built only from `State` and `Params` (`docs/features/decision-points.md`).
//! `run` builds the view, decides, then applies the choice as deltas. Matching unemployed people
//! to vacancies and picking whom to fire are mechanics and stay in `run`.

use crate::production::{capacity_factors, CapacityFactors};
use crate::util::{cfg, workers_of};
use libm::pow;
use sim_params::{LabourParams, Params, ProductionParams};
use sim_rng::Rng;
use sim_types::DecisionId as D;
use sim_types::Effect as Fx;
use sim_types::MechanismId as M;
use sim_types::QuantityId as Q;
use sim_types::*;
use std::cmp::Ordering;

/// A firm's monthly wage change: the trend plus a response to unemployment against the
/// reference rate, capped each month. Wages, unlike goods prices, respond to slack.
pub fn wage_step(lp: &sim_params::LabourParams, trend: f64, unemployment: f64) -> f64 {
    (trend + lp.wage_k * (lp.u_ref - unemployment)).clamp(-lp.wage_cap, lp.wage_cap)
}

// ---- Decision: how many workers to want, and the vacancies to post ----

pub const FIRM_HIRE: DecisionSpec = DecisionSpec {
    id: D::FirmHire,
    system: SystemId::Labour,
    inputs: &[
        Q::ExpectedSales,
        Q::Inventory,
        Q::InventoryCover,
        Q::Workers,
        Q::SkilledWorkers,
        Q::EffectiveLabour,
        Q::CapitalOutput,
        Q::WeatherFactor,
        Q::ResourceFactor,
        Q::TransitionFactor,
    ],
    outputs: &[Q::WantedWorkers, Q::Vacancies],
    effects: &[
        Fx::up(Q::ExpectedSales, Q::WantedWorkers),
        Fx::up(Q::ExpectedSales, Q::Vacancies),
        Fx::down(Q::Inventory, Q::WantedWorkers),
        Fx::down(Q::Inventory, Q::Vacancies),
        Fx::up(Q::InventoryCover, Q::WantedWorkers),
        Fx::up(Q::InventoryCover, Q::Vacancies),
        Fx::none(Q::Workers, Q::WantedWorkers, "the workers a firm wants depend on output needed and productivity, not on how many it has"),
        Fx::down(Q::Workers, Q::Vacancies),
        Fx::none(Q::SkilledWorkers, Q::WantedWorkers, "the workers wanted depend on output needed, not on how many are skilled"),
        Fx::none(Q::SkilledWorkers, Q::Vacancies, "skilled workers change which vacancies are skilled, not how many in all"),
        Fx::down(Q::EffectiveLabour, Q::WantedWorkers),
        Fx::down(Q::EffectiveLabour, Q::Vacancies),
        Fx::down(Q::CapitalOutput, Q::WantedWorkers),
        Fx::down(Q::CapitalOutput, Q::Vacancies),
        Fx::down(Q::WeatherFactor, Q::WantedWorkers),
        Fx::down(Q::WeatherFactor, Q::Vacancies),
        Fx::down(Q::ResourceFactor, Q::WantedWorkers),
        Fx::down(Q::ResourceFactor, Q::Vacancies),
        Fx::down(Q::TransitionFactor, Q::WantedWorkers),
        Fx::down(Q::TransitionFactor, Q::Vacancies),
    ],
    params: &[
        "labour.growth_aspiration",
        "labour.inventory_cover",
        "labour.services_inventory_cover",
        "labour.inventory_adjust",
        "labour.hire_slack",
        "labour.min_target_workers",
        "labour.skilled_share",
        "labour.default_effective_per_worker",
        "labour.max_hire_per_month",
        "production.beta_n",
        "class.skill_threshold",
    ],
    rule: "inventory_cover_target",
};

/// What a firm sees when it sizes its workforce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HireView {
    pub expected_sales: f64,
    pub inventory: f64,
    /// Months of sales held as stock (services hold none).
    pub inventory_cover: f64,
    /// Headcount now.
    pub workers: f64,
    pub skilled_workers: f64,
    /// Average effective labour (skill x hours) per worker.
    pub effective_labour: f64,
    /// What one unit of labour would make now, as the factors capacity multiplies: capital and
    /// technology, weather, resources and the transition (`production::CapacityFactors`).
    pub capacity: CapacityFactors,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HireChoice {
    /// Workers needed, in people (before the hiring slack and the minimum).
    pub workers_wanted: f64,
    pub skilled_vacancies: f64,
    pub general_vacancies: f64,
    /// The most it takes on this month.
    pub max_hires: u32,
}

pub fn hire_view(s: &State, p: &Params, f: &Firm, mine: &[&Person]) -> HireView {
    let lp = &p.labour;
    let n_now = mine.len() as f64;
    let avg_eff = if mine.is_empty() {
        lp.default_effective_per_worker
    } else {
        mine.iter()
            .map(|x| x.skill.0 * x.effective_hours.0)
            .sum::<f64>()
            / n_now
    };
    HireView {
        expected_sales: f.expected_sales.0,
        inventory: f.inventory.0,
        inventory_cover: if f.sector == Sector::Services {
            lp.services_inventory_cover
        } else {
            lp.inventory_cover
        },
        workers: n_now,
        skilled_workers: mine
            .iter()
            .filter(|x| x.skill.0 >= p.class.skill_threshold)
            .count() as f64,
        effective_labour: avg_eff,
        capacity: capacity_factors(s, p, f, 1.0),
    }
}

/// How much output the firm wants, so how many effective hours of labour and how many people. A
/// firm posts vacancies up to the next whole person once the gap exceeds `hire_slack`; the
/// skilled tier gets what it is short of.
pub fn decide_hire(v: &HireView, lp: &LabourParams, pp: &ProductionParams) -> HireChoice {
    let expected = v.expected_sales * (1.0 + lp.growth_aspiration);
    let needed =
        (expected + lp.inventory_adjust * (v.inventory_cover * expected - v.inventory)).max(0.0);
    let output_capacity = v.capacity.product();
    let l_need = if output_capacity > 0.0 {
        pow(needed / output_capacity, 1.0 / pp.beta_n)
    } else {
        0.0
    };
    let x = l_need / v.effective_labour.max(crate::util::TINY_DIVISOR);
    let target = (x - lp.hire_slack).ceil().max(lp.min_target_workers);
    let target_skilled = (lp.skilled_share * target).round();
    let gap = (target - v.workers).max(0.0);
    let skilled_vacancies = (target_skilled - v.skilled_workers).max(0.0).min(gap);
    HireChoice {
        workers_wanted: x,
        skilled_vacancies,
        general_vacancies: gap - skilled_vacancies,
        max_hires: lp.max_hire_per_month,
    }
}

// ---- Decision: fire after a sustained surplus ----

pub const FIRM_FIRE: DecisionSpec = DecisionSpec {
    id: D::FirmFire,
    system: SystemId::Labour,
    inputs: &[Q::Workers, Q::WantedWorkers, Q::OverTargetMonths],
    outputs: &[Q::Workers, Q::OverTargetMonths],
    effects: &[
        Fx::up(Q::Workers, Q::Workers),
        Fx::up(Q::Workers, Q::OverTargetMonths),
        Fx::up(Q::WantedWorkers, Q::Workers),
        Fx::down(Q::WantedWorkers, Q::OverTargetMonths),
        Fx::down(Q::OverTargetMonths, Q::Workers),
        Fx::varies(Q::OverTargetMonths, Q::OverTargetMonths, "the counter adds a month while the surplus lasts, but the month the firm fires it resets to zero"),
    ],
    params: &[
        "labour.min_target_workers",
        "labour.fire_slack",
        "labour.fire_after_months",
        "labour.max_fire_per_month",
    ],
    rule: "sustained_surplus",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireView {
    pub workers: f64,
    /// The hire decision's `workers_wanted`.
    pub workers_wanted: f64,
    pub over_target_months: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireChoice {
    pub over_target_months: u32,
    /// How many to let go this month (the least skilled go first, chosen in `run`).
    pub to_fire: u32,
}

pub fn fire_view(f: &Firm, workers: f64, hire: &HireChoice) -> FireView {
    FireView {
        workers,
        workers_wanted: hire.workers_wanted,
        over_target_months: f.over_target_months,
    }
}

pub fn decide_fire(v: &FireView, lp: &LabourParams) -> FireChoice {
    let mut over = v.over_target_months;
    if v.workers - v.workers_wanted.max(lp.min_target_workers) >= lp.fire_slack {
        over += 1;
    } else {
        over = 0;
    }
    let mut to_fire = 0;
    if over >= lp.fire_after_months {
        let surplus = (v.workers - v.workers_wanted.max(lp.min_target_workers))
            .max(0.0)
            .floor() as u32;
        to_fire = surplus.min(lp.max_fire_per_month);
        over = 0;
    }
    FireChoice {
        over_target_months: over,
        to_fire,
    }
}

// ---- Decision: the wage ----

pub const FIRM_WAGE: DecisionSpec = DecisionSpec {
    id: D::FirmWage,
    system: SystemId::Labour,
    inputs: &[Q::Wage, Q::MinWage, Q::Unemployment, Q::SectorTfpGrowth],
    outputs: &[Q::Wage],
    effects: &[
        Fx::up(Q::Wage, Q::Wage),
        Fx::up(Q::MinWage, Q::Wage),
        Fx::down(Q::Unemployment, Q::Wage),
        Fx::up(Q::SectorTfpGrowth, Q::Wage),
    ],
    params: &[
        "labour.wage_k",
        "labour.u_ref",
        "labour.wage_cap",
        "labour.inflation_anchor",
        "labour.productivity_passthrough",
        "production.tfp_growth_goods",
        "production.tfp_growth_food",
        "production.tfp_growth_services",
    ],
    rule: "trend_plus_slack",
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WageView {
    pub wage: Cents,
    pub min_wage: Cents,
    /// The town's smoothed unemployment rate.
    pub unemployment: f64,
    /// The sector's productivity growth per month.
    pub sector_tfp_growth: f64,
}

pub fn wage_view(p: &Params, f: &Firm, unemployment: f64, min_wage: Cents) -> WageView {
    WageView {
        wage: f.wage,
        min_wage,
        unemployment,
        sector_tfp_growth: match f.sector {
            Sector::Goods => p.production.tfp_growth_goods,
            Sector::Food => p.production.tfp_growth_food,
            Sector::Services => p.production.tfp_growth_services,
        },
    }
}

/// w *= 1 + trend + k (u_ref - u), capped each month. The trend is the sector's productivity
/// growth, so that wages and prices do not drift apart.
pub fn decide_wage(v: &WageView, lp: &LabourParams) -> Cents {
    let trend = lp.inflation_anchor + lp.productivity_passthrough * v.sector_tfp_growth;
    let step = wage_step(lp, trend, v.unemployment);
    Cents::from_f64(v.wage.as_f64() * (1.0 + step)).max(v.min_wage)
}

// ---- Mechanisms (causality graph stage 1): declarations of the plain code below; they change no behaviour. ----

/// Posted vacancies are filled from the jobless in order of reservation wage (`labour::run`).
pub const LABOUR_MATCHING: MechanismSpec = MechanismSpec {
    id: M::LabourMatching,
    system: SystemId::Labour,
    inputs: &[Q::Vacancies],
    outputs: &[Q::Workers],
    effects: &[Fx::up(Q::Vacancies, Q::Workers).unverified("the arithmetic is inline in labour::run over per-firm or per-household state and sim-model tests cannot build a State; no pure function to nudge (matching the jobless to vacancies)")],
    params: &[],
    rule: "fill_vacancies_from_jobless",
};

/// Unemployment is the share of the labour force without a job (`sim_metrics`).
pub const UNEMPLOYMENT_RATE: MechanismSpec = MechanismSpec {
    id: M::UnemploymentRate,
    system: SystemId::Metrics,
    inputs: &[Q::Workers],
    outputs: &[Q::Unemployment],
    effects: &[Fx::down(Q::Workers, Q::Unemployment).unverified(
        "sim_metrics::unemployment reads the people on a State, which sim-model tests cannot build",
    )],
    params: &[],
    rule: "jobless_share_of_labour_force",
};

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let c = cfg(s, p);
    let lp = &p.labour;
    let unemployment = s.indicators.unemployment_smooth.0;
    let min_wage = s.policy.min_wage.scale(c.min_wage_multiplier);
    let skilled = |x: &Person| x.skill.0 >= p.class.skill_threshold;
    let mut job: Vec<Option<FirmId>> = s.persons.iter().map(|x| x.job).collect();

    for f in s.firms.iter().filter(|f| f.active) {
        let mine: Vec<&Person> = workers_of(s, f.id).collect();
        let n_now = mine.len() as f64;

        // Decide: how many workers, and the vacancies.
        let hire = decide_hire(&hire_view(s, p, f, &mine), lp, &p.production);
        let (skilled_vacancies, general_vacancies) =
            (hire.skilled_vacancies, hire.general_vacancies);

        // Hire: the cheapest unemployed people of the right tier, up to the monthly cap.
        let mut unfilled = false;
        let posted = (skilled_vacancies + general_vacancies) as u32;
        let mut open_after = 0u32;
        let mut hires_left = hire.max_hires;
        let mut hired_now = 0.0;
        for (want_skilled, vac) in [(true, skilled_vacancies), (false, general_vacancies)] {
            let mut vacancies = vac as u32;
            let mut pool: Vec<usize> = s
                .persons
                .iter()
                .filter(|x| x.in_labour_force && job[x.id.idx()].is_none() && x.staff_of.is_none())
                .filter(|x| skilled(x) == want_skilled)
                .map(|x| x.id.idx())
                .collect();
            pool.sort_by(|&a, &b| {
                let (pa, pb) = (&s.persons[a], &s.persons[b]);
                pa.reservation
                    .cmp(&pb.reservation)
                    .then(
                        pb.skill
                            .0
                            .partial_cmp(&pa.skill.0)
                            .unwrap_or(Ordering::Equal),
                    )
                    .then(a.cmp(&b))
            });
            for i in pool {
                if vacancies == 0 || hires_left == 0 {
                    break;
                }
                job[i] = Some(f.id);
                vacancies -= 1;
                hires_left -= 1;
                hired_now += 1.0;
                d.events.push(Event::PersonHired {
                    person: PersonId(i as u32),
                    firm: f.id,
                });
            }
            if vacancies > 0 {
                unfilled = true;
            }
            open_after += vacancies;
        }
        d.trace.push(TraceRecord::Vacancies {
            firm: f.id,
            posted,
            filled: posted.saturating_sub(open_after),
        });

        // Fire after a sustained surplus: the least skilled go first.
        let fire = decide_fire(&fire_view(f, n_now, &hire), lp);
        let over = fire.over_target_months;
        let mut fired_now = 0.0;
        if fire.to_fire > 0 {
            let mut to_fire = fire.to_fire;
            let mut order: Vec<&Person> = mine.clone();
            order.sort_by(|a, b| {
                a.skill
                    .0
                    .partial_cmp(&b.skill.0)
                    .unwrap_or(Ordering::Equal)
                    .then(b.id.cmp(&a.id))
            });
            for x in order {
                if to_fire == 0 {
                    break;
                }
                job[x.id.idx()] = None;
                to_fire -= 1;
                fired_now += 1.0;
                d.events.push(Event::PersonFired {
                    person: x.id,
                    firm: f.id,
                });
            }
        }

        // Decide the wage.
        let wage = decide_wage(&wage_view(p, f, unemployment, min_wage), lp);
        d.firm(
            f.id,
            FirmPatch {
                wage: Some(wage),
                unfilled: Some(unfilled),
                over_target_months: Some(over),
                workers: Some(People(n_now + hired_now - fired_now)),
                ..FirmPatch::default()
            },
        );
    }

    // Job guarantee: the government employs a share of those left without a firm job. Current
    // members stay while they are still unemployed; the share is topped up by person id.
    let jobless: Vec<usize> = s
        .persons
        .iter()
        .filter(|x| x.in_labour_force && job[x.id.idx()].is_none() && x.staff_of.is_none())
        .map(|x| x.id.idx())
        .collect();
    let want_jg = (s.policy.job_guarantee.0 * jobless.len() as f64).round() as usize;
    let mut in_jg = vec![false; s.persons.len()];
    let mut taken = 0;
    for pass in 0..2 {
        for &i in &jobless {
            if taken >= want_jg {
                break;
            }
            let member = s.persons[i].jg;
            if (pass == 0) == member && !in_jg[i] {
                in_jg[i] = true;
                taken += 1;
            }
        }
    }

    // Commutes follow the final jobs: the cheapest route from home to workplace over the map.
    let commute = crate::space::commutes(s, p, &job);

    // Person updates: jobs, guarantee, skill drift (employment builds skill, idleness erodes it).
    for x in &s.persons {
        let i = x.id.idx();
        let working = job[i].is_some() || in_jg[i] || x.staff_of.is_some();
        let mut patch = PersonPatch::default();
        if job[i] != x.job {
            patch.job = Some(job[i]);
        }
        if in_jg[i] != x.jg {
            patch.jg = Some(in_jg[i]);
        }
        let (tiles, minutes) = commute[i];
        if tiles != x.commute_tiles {
            patch.commute_tiles = Some(tiles);
        }
        if minutes != x.commute_minutes {
            patch.commute_minutes = Some(minutes);
        }
        let hours = if job[i].is_some() || x.staff_of.is_some() {
            crate::space::effective_hours(&p.space, minutes)
        } else {
            1.0
        };
        if hours != x.effective_hours.0 {
            patch.effective_hours = Some(Rate(hours));
        }
        if let Some(f) = job[i] {
            d.events.push(Event::Commuted {
                person: x.id,
                firm: f,
                tiles,
                minutes,
            });
        }
        if x.in_labour_force {
            let skill = x.skill.0
                * (1.0
                    + if working {
                        lp.skill_gain
                    } else {
                        -lp.skill_loss
                    });
            patch.skill = Some(Rate(skill.clamp(lp.skill_min, lp.skill_max)));
        }
        if patch != PersonPatch::default() {
            d.changes.push(Delta::Person(x.id, patch));
        }
    }
    d
}

#[cfg(test)]
mod decision_tests {
    use super::*;
    use crate::decision_tests::{check_view_only, params, Field};

    #[test]
    fn hire_reads_only_its_declared_inputs() {
        let p = params();
        let base = |sales, inv, cover, workers, skilled| HireView {
            expected_sales: sales,
            inventory: inv,
            inventory_cover: cover,
            workers,
            skilled_workers: skilled,
            effective_labour: 1.0,
            capacity: CapacityFactors {
                capital_output: 2.0,
                farm_health: 1.0,
                climate: 1.0,
                drought: 1.0,
                community: 1.0,
                smoke: 1.0,
                transition: 1.0,
            },
        };
        let bases = [
            base(100.0, 10.0, 1.0, 2.0, 0.0),
            base(400.0, 300.0, 0.0, 10.0, 1.0),
            base(900.0, 0.0, 2.0, 1.0, 0.0),
        ];
        let HireView {
            expected_sales: _,
            inventory: _,
            inventory_cover: _,
            workers: _,
            skilled_workers: _,
            effective_labour: _,
            capacity: _,
        } = bases[0];
        let CapacityFactors {
            capital_output: _,
            farm_health: _,
            climate: _,
            drought: _,
            community: _,
            smoke: _,
            transition: _,
        } = bases[0].capacity;
        let fields: [Field<HireView>; 13] = [
            ("expected_sales", Q::ExpectedSales, |v| {
                v.expected_sales = v.expected_sales * 2.0 + 50.0
            }),
            ("inventory", Q::Inventory, |v| {
                v.inventory = v.inventory * 2.0 + 50.0
            }),
            ("inventory_cover", Q::InventoryCover, |v| {
                v.inventory_cover += 1.0
            }),
            ("workers", Q::Workers, |v| v.workers += 5.0),
            ("skilled_workers", Q::SkilledWorkers, |v| {
                v.skilled_workers += 3.0
            }),
            ("effective_labour", Q::EffectiveLabour, |v| {
                v.effective_labour *= 2.0
            }),
            ("capital_output", Q::CapitalOutput, |v| {
                v.capacity.capital_output *= 2.0
            }),
            ("farm_health", Q::ResourceFactor, |v| {
                v.capacity.farm_health *= 0.5
            }),
            ("climate", Q::WeatherFactor, |v| v.capacity.climate *= 0.5),
            ("drought", Q::WeatherFactor, |v| v.capacity.drought *= 0.5),
            ("community", Q::ResourceFactor, |v| {
                v.capacity.community *= 0.5
            }),
            ("smoke", Q::WeatherFactor, |v| v.capacity.smoke *= 0.5),
            ("transition", Q::TransitionFactor, |v| {
                v.capacity.transition *= 0.5
            }),
        ];
        check_view_only(&FIRM_HIRE, &bases, &fields, |v| {
            decide_hire(v, &p.labour, &p.production)
        });
    }

    #[test]
    fn fire_reads_only_its_declared_inputs() {
        let p = params();
        let lp = &p.labour;
        let base = |w, want, over| FireView {
            workers: w,
            workers_wanted: want,
            over_target_months: over,
        };
        let bases = [
            base(20.0, 5.0, lp.fire_after_months.saturating_sub(1)),
            base(20.0, 5.0, 0),
            base(2.0, 2.0, 3),
        ];
        let FireView {
            workers: _,
            workers_wanted: _,
            over_target_months: _,
        } = bases[0];
        let fields: [Field<FireView>; 3] = [
            ("workers", Q::Workers, |v| v.workers -= 15.0),
            ("workers_wanted", Q::WantedWorkers, |v| {
                v.workers_wanted += 30.0
            }),
            ("over_target_months", Q::OverTargetMonths, |v| {
                v.over_target_months += 1
            }),
        ];
        check_view_only(&FIRM_FIRE, &bases, &fields, |v| decide_fire(v, lp));
    }

    #[test]
    fn wage_reads_only_its_declared_inputs() {
        let p = params();
        let lp = &p.labour;
        let base = |wage: i64, min: i64, u, g| WageView {
            wage: Cents(wage),
            min_wage: Cents(min),
            unemployment: u,
            sector_tfp_growth: g,
        };
        let bases = [
            base(300_000, 100_000, 0.05, 0.001),
            base(300_000, 400_000, 0.05, 0.001),
            base(300_000, 100_000, lp.u_ref, 0.0),
        ];
        let WageView {
            wage: _,
            min_wage: _,
            unemployment: _,
            sector_tfp_growth: _,
        } = bases[0];
        let fields: [Field<WageView>; 4] = [
            ("wage", Q::Wage, |v| v.wage = Cents(v.wage.0 * 2)),
            ("min_wage", Q::MinWage, |v| {
                v.min_wage = Cents(v.min_wage.0 + 200_000)
            }),
            ("unemployment", Q::Unemployment, |v| v.unemployment += 0.01),
            ("sector_tfp_growth", Q::SectorTfpGrowth, |v| {
                v.sector_tfp_growth += 0.0005
            }),
        ];
        check_view_only(&FIRM_WAGE, &bases, &fields, |v| decide_wage(v, lp));
    }
}
