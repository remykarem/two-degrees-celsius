//! The perturbation sign test on pure functions and decisions (causality graph stage 2,
//! `docs/features/causality-graph.md` section 5). The rules and the report are in
//! `crate::sign_probe` (shared with `tests/mechanism_signs`, which measures whole systems on a real
//! state); the probes here (`decisions`, `mechanisms`) hold the states and the two closures per
//! effect (how to raise the input, how to read the output) for effects marked `Basis::Perturbed`.

use crate::causal::Node;
use crate::decisions::{DECISIONS, MECHANISMS};
use crate::sign_probe::{name, Report};
use sim_types::Basis;

mod decisions;
mod mechanisms;

/// Run every probe once.
pub(crate) fn run_all() -> Report {
    let mut r = Report::new(Basis::Perturbed);
    decisions::probe_all(&mut r);
    mechanisms::probe_all(&mut r);
    r
}

/// (declared effects, perturbed, declared_unverified) over the whole registry.
pub(crate) fn counts() -> (usize, usize, usize, usize) {
    let mut c = (0, 0, 0, 0);
    let all = DECISIONS
        .iter()
        .map(|d| d.effects)
        .chain(MECHANISMS.iter().map(|m| m.effects));
    for es in all {
        for e in es {
            c.0 += 1;
            match e.basis {
                Basis::Perturbed => c.1 += 1,
                Basis::DeclaredUnverified(_) => c.2 += 1,
                Basis::PerturbedSystem => c.3 += 1,
            }
        }
    }
    c
}

#[test]
fn declared_signs_agree_with_the_real_code() {
    let r = run_all();
    let (all, perturbed, unverified, system) = counts();
    // Every perturbed effect was probed exactly once.
    let mut missing = Vec::new();
    for (node, effects) in DECISIONS
        .iter()
        .map(|d| (Node::Decision(d.id), d.effects))
        .chain(
            MECHANISMS
                .iter()
                .map(|m| (Node::Mechanism(m.id), m.effects)),
        )
    {
        for e in effects.iter().filter(|e| e.basis == Basis::Perturbed) {
            let n = r
                .probed
                .iter()
                .filter(|(nn, f, t, _)| *nn == node && *f == e.from && *t == e.to)
                .count();
            if n != 1 {
                missing.push(format!(
                    "{}: {} -> {} probed {n} times",
                    name(node),
                    e.from.as_str(),
                    e.to.as_str()
                ));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "effects without exactly one probe:\n{}",
        missing.join("\n")
    );
    assert!(
        r.failures.is_empty(),
        "sign test failures:\n{}",
        r.failures.join("\n")
    );
    assert_eq!(r.probed.len(), perturbed);
    println!(
        "effects {all}, verified on pure code {perturbed}, on whole systems {system}, declared_unverified {unverified}"
    );
}
