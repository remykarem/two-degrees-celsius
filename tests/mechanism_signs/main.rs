//! The perturbation sign test on whole systems (causality graph stage 2). Mechanisms that live
//! inline in a system's `run` cannot be called as a pure function, so each effect marked
//! `Basis::PerturbedSystem` is measured by nudging one field of a real mid-run `State`, running
//! the real system of `TICK_ORDER` and applying its deltas (`common::after`), then reading one field
//! of the result. The judging rules are `sim_model::sign_probe` (shared with the in-crate probes in
//! `crates/sim-model/src/sign_tests`): flat responses are clamped samples and ignored; a signed
//! effect must move, all unclamped samples in the declared direction.
//!
//! Each probe says in its comment which concrete field it nudges (a town aggregate such as
//! `Wage` or `Savings` is a sum or mean over firms or households) and which it reads.

mod common;
mod group_a;
mod group_b;
mod group_c;

use sim_model::causal::Node;
use sim_model::decisions::MECHANISMS;
use sim_model::sign_probe::Report;
use sim_types::Basis;

#[test]
fn declared_system_signs_agree_with_the_real_systems() {
    let mut r = Report::new(Basis::PerturbedSystem);
    group_a::probe_all(&mut r);
    group_b::probe_all(&mut r);
    group_c::probe_all(&mut r);
    let mut missing = Vec::new();
    for m in MECHANISMS {
        for e in m
            .effects
            .iter()
            .filter(|e| e.basis == Basis::PerturbedSystem)
        {
            let n = r
                .probed
                .iter()
                .filter(|(nn, f, t, _)| *nn == Node::Mechanism(m.id) && *f == e.from && *t == e.to)
                .count();
            if n != 1 {
                missing.push(format!(
                    "{}: {} -> {} probed {n} times",
                    m.id.as_str(),
                    e.from.as_str(),
                    e.to.as_str()
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "system effects without exactly one probe:\n{}",
        missing.join("\n")
    );
    assert!(
        r.failures.is_empty(),
        "system sign test failures:\n{}",
        r.failures.join("\n")
    );
    println!("system effects verified: {}", r.probed.len());
}
