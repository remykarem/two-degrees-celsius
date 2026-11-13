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
mod group_d;

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
    group_d::probe_all(&mut r);
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

/// `content/strengths.json` is generated from these probes (`UPDATE_STRENGTHS=1`); without the
/// variable the file must equal what the probes measure now (a staleness test).
fn reconcile_strengths(basis: &str, fresh: Vec<sim_params::strengths::StrengthEntry>) {
    use sim_params::strengths as st;
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/strengths.json");
    let current = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| st::parse(&t).ok())
        .unwrap_or(st::StrengthFile {
            note: st::NOTE.into(),
            entries: Vec::new(),
        });
    if std::env::var_os("UPDATE_STRENGTHS").is_some() {
        let merged = st::merge(current, basis, fresh);
        std::fs::write(path, st::render(&merged)).unwrap();
        return;
    }
    let mut mine: Vec<_> = current
        .entries
        .into_iter()
        .filter(|e| e.basis == basis)
        .collect();
    let mut fresh = fresh;
    let key = |e: &st::StrengthEntry| (e.via.clone(), e.from.clone(), e.to.clone());
    mine.sort_by_key(key);
    fresh.sort_by_key(key);
    assert!(
        mine == fresh,
        "content/strengths.json is stale for basis {basis}: regenerate with UPDATE_STRENGTHS=1 (docs/how-to.md section 31)"
    );
}

#[test]
fn system_strengths_file_is_current() {
    let mut r = Report::new(Basis::PerturbedSystem);
    group_a::probe_all(&mut r);
    group_b::probe_all(&mut r);
    group_c::probe_all(&mut r);
    group_d::probe_all(&mut r);
    reconcile_strengths("system", r.strengths);
}
