//! S1 labour market: vacancies, hiring from the unemployed pool, firing, wage adjustment, skill.

use crate::production::capacity;
use crate::util::{cfg, is_skilled};
use sim_params::Params;
use sim_rng::Rng;
use sim_types::*;

/// Unemployed people available in cohort `h`, given the current employment matrix.
fn free_people(s: &State, emp: &[Vec<f64>], h: usize) -> f64 {
    let hh = &s.households[h];
    (hh.labour_force().0 - emp[h].iter().sum::<f64>()).max(0.0)
}

/// Cohorts of one tier, cheapest first (lowest reservation wage), ties by highest skill.
fn pool_order(s: &State, skilled: bool) -> Vec<usize> {
    let mut v: Vec<usize> = s
        .households
        .iter()
        .enumerate()
        .filter(|(_, h)| h.labour_share.0 > 0.0 && is_skilled(h) == skilled)
        .map(|(i, _)| i)
        .collect();
    v.sort_by(|&a, &b| {
        let (ha, hb) = (&s.households[a], &s.households[b]);
        ha.reservation
            .cmp(&hb.reservation)
            .then(
                hb.skill
                    .0
                    .partial_cmp(&ha.skill.0)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.cmp(&b))
    });
    v
}

pub fn run(s: &State, p: &Params, _r: &mut Rng) -> Deltas {
    let mut d = Deltas::new();
    let c = cfg(s, p);
    let lp = &p.labour;
    let n_h = s.households.len();
    let n_f = s.firms.len();
    let mut emp: Vec<Vec<f64>> = s
        .households
        .iter()
        .map(|h| h.employment.iter().map(|x| x.0).collect())
        .collect();
    let unemployment = s.indicators.unemployment.0;
    let skilled_order = pool_order(s, true);
    let general_order = pool_order(s, false);
    let min_wage = s.policy.min_wage.scale(c.min_wage_multiplier);

    for f in s.firms.iter().filter(|f| f.active) {
        let fi = f.id.idx();
        let skilled_now: f64 = (0..n_h)
            .filter(|&h| is_skilled(&s.households[h]))
            .map(|h| emp[h][fi])
            .sum();
        let n_now: f64 = (0..n_h).map(|h| emp[h][fi]).sum();
        let general_now = n_now - skilled_now;

        let avg_sales = f.sales_hist.iter().sum::<f64>() / 3.0;
        let expected = avg_sales * (1.0 + lp.growth_aspiration);
        let inv_target = lp.inventory_cover * expected;
        let needed = (expected + lp.inventory_adjust * (inv_target - f.inventory.0)).max(0.0);
        let n_ref = n_now.max(lp.min_workers);
        let opw = capacity(s, p, f, n_ref) / n_ref;
        let target = if opw > 0.0 {
            (needed / opw).ceil()
        } else {
            0.0
        };
        let target_skilled = (lp.skilled_share * target).ceil();
        let target_general = (target - target_skilled).max(0.0);

        // Hire.
        let mut unfilled = false;
        for (skilled, order, have, want) in [
            (true, &skilled_order, skilled_now, target_skilled),
            (false, &general_order, general_now, target_general),
        ] {
            let mut vac = (want - have).max(0.0);
            for &h in order {
                if vac <= 1e-9 {
                    break;
                }
                let take = free_people(s, &emp, h).min(vac);
                if take > 1e-9 {
                    emp[h][fi] += take;
                    vac -= take;
                    d.events.push(Event::Hired {
                        firm: f.id,
                        people: People(take),
                    });
                    d.events.push(Event::Commuted {
                        household: s.households[h].id,
                        firm: f.id,
                        people: People(take),
                    });
                }
            }
            if vac > 0.5 {
                unfilled = true;
            }
            let _ = skilled;
        }

        // Fire after a sustained surplus.
        let mut over = f.over_target_months;
        let surplus_now = n_now > target_skilled + target_general + 0.5;
        if surplus_now {
            over += 1;
        } else {
            over = 0;
        }
        if over >= lp.fire_after_months {
            for skilled in [true, false] {
                let have: f64 = (0..n_h)
                    .filter(|&h| is_skilled(&s.households[h]) == skilled)
                    .map(|h| emp[h][fi])
                    .sum();
                let want = if skilled {
                    target_skilled
                } else {
                    target_general
                };
                let cut = (have - want).max(0.0).min(lp.max_fire_share * have);
                if cut > 1e-9 && have > 0.0 {
                    let factor = 1.0 - cut / have;
                    for h in 0..n_h {
                        if is_skilled(&s.households[h]) == skilled {
                            emp[h][fi] *= factor;
                        }
                    }
                    d.events.push(Event::Fired {
                        firm: f.id,
                        people: People(cut),
                    });
                }
            }
            over = 0;
        }

        // Wage adjustment: the reserve-army effect.
        let mut w = f.wage.as_f64();
        if unfilled {
            w *= lp.wage_up;
        } else if unemployment > lp.wage_cut_unemployment {
            w *= c.wage_down;
        } else if unemployment < lp.wage_tight_unemployment {
            w *= lp.wage_tight_up;
        }
        let wage = Cents::from_f64(w).max(min_wage);
        let n_new: f64 = (0..n_h).map(|h| emp[h][fi]).sum();
        d.firm(
            f.id,
            FirmPatch {
                wage: Some(wage),
                unfilled: Some(unfilled),
                over_target_months: Some(over),
                workers: Some(People(n_new)),
                ..FirmPatch::default()
            },
        );
    }

    // Employment changes and skill drift.
    for h in 0..n_h {
        for fi in 0..n_f {
            let old = s.households[h].employment[fi].0;
            if (emp[h][fi] - old).abs() > 1e-12 {
                d.changes.push(Delta::Employment {
                    h: HouseholdId(h as u32),
                    f: FirmId(fi as u32),
                    n: People(emp[h][fi].max(0.0)),
                });
            }
        }
        let hh = &s.households[h];
        if hh.labour_share.0 > 0.0 {
            let e = (emp[h].iter().sum::<f64>() / hh.labour_force().0.max(1e-9)).clamp(0.0, 1.0);
            let skill = (hh.skill.0 * (1.0 + lp.skill_gain * e - lp.skill_loss * (1.0 - e)))
                .clamp(0.05, 1.0);
            d.household(
                hh.id,
                HouseholdPatch {
                    skill: Some(Rate(skill)),
                    ..HouseholdPatch::default()
                },
            );
        }
    }
    d
}
