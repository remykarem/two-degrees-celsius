//! S1 labour market: named jobs. Each unemployed adult applies to firms with vacancies,
//! the cheapest first (ties by skill). Firing removes named people. Wages, skill and the
//! job guarantee follow.

use crate::production::capacity;
use crate::util::{cfg, workers_of};
use libm::pow;
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;
use std::cmp::Ordering;

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
        let avg_eff = if mine.is_empty() {
            lp.default_effective_per_worker
        } else {
            mine.iter()
                .map(|x| x.skill.0 * x.effective_hours.0)
                .sum::<f64>()
                / n_now
        };

        // How much output the firm wants, and so how many effective hours of labour.
        let expected = f.expected_sales.0 * (1.0 + lp.growth_aspiration);
        let cover = if f.sector == Sector::Services {
            lp.services_inventory_cover
        } else {
            lp.inventory_cover
        };
        let needed = (expected + lp.inventory_adjust * (cover * expected - f.inventory.0)).max(0.0);
        let one_unit = capacity(s, p, f, 1.0);
        let l_need = if one_unit > 0.0 {
            pow(needed / one_unit, 1.0 / p.production.beta_n)
        } else {
            0.0
        };
        // Workers needed, in people. A firm hires up to the next whole person once the gap
        // exceeds `hire_slack`, and fires only when it is `fire_slack` people overstaffed.
        let x = l_need / avg_eff.max(1e-9);
        let target = (x - lp.hire_slack).ceil().max(lp.min_target_workers);
        let target_skilled = (lp.skilled_share * target).round();
        let have_skilled = mine.iter().filter(|x| skilled(x)).count() as f64;
        // Vacancies: the headcount gap, of which the skilled tier gets what it is short of.
        let gap = (target - n_now).max(0.0);
        let skilled_vacancies = (target_skilled - have_skilled).max(0.0).min(gap);
        let general_vacancies = gap - skilled_vacancies;

        // Hire: the cheapest unemployed people of the right tier, up to the monthly cap.
        let mut unfilled = false;
        let mut hires_left = lp.max_hire_per_month;
        let mut hired_now = 0.0;
        for (want_skilled, vac) in [(true, skilled_vacancies), (false, general_vacancies)] {
            let mut vacancies = vac as u32;
            let mut pool: Vec<usize> = s
                .persons
                .iter()
                .filter(|x| x.in_labour_force && job[x.id.idx()].is_none())
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
        }

        // Fire after a sustained surplus: the least skilled go first.
        let mut over = f.over_target_months;
        if n_now - x.max(lp.min_target_workers) >= lp.fire_slack {
            over += 1;
        } else {
            over = 0;
        }
        let mut fired_now = 0.0;
        if over >= lp.fire_after_months {
            let surplus = (n_now - x.max(lp.min_target_workers)).max(0.0).floor() as u32;
            let mut to_fire = surplus.min(lp.max_fire_per_month);
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
            over = 0;
        }

        // Wage adjustment follows employment: w *= 1 + trend + k (u_ref - u), capped each month.
        // The trend is the sector's productivity growth, so that wages and prices do not drift apart.
        let trend = lp.inflation_anchor
            + lp.productivity_passthrough
                * match f.sector {
                    Sector::Goods => p.production.tfp_growth_goods,
                    Sector::Food => p.production.tfp_growth_food,
                    Sector::Services => p.production.tfp_growth_services,
                };
        let step = (trend + lp.wage_k * (lp.u_ref - unemployment)).clamp(-lp.wage_cap, lp.wage_cap);
        let wage = Cents::from_f64(f.wage.as_f64() * (1.0 + step)).max(min_wage);
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
        .filter(|x| x.in_labour_force && job[x.id.idx()].is_none())
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

    // Person updates: jobs, guarantee, skill drift (employment builds skill, idleness erodes it).
    for x in &s.persons {
        let i = x.id.idx();
        let working = job[i].is_some() || in_jg[i];
        let mut patch = PersonPatch::default();
        if job[i] != x.job {
            patch.job = Some(job[i]);
        }
        if in_jg[i] != x.jg {
            patch.jg = Some(in_jg[i]);
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
