//! The perturbation sign test (causality graph stage 2, `docs/features/causality-graph.md` section 5).
//! For every declared `Effect` with `Basis::Perturbed`, nudge the input of the real code up by a
//! finite step over a sample of states and read the output; the declared sign must agree with the
//! measured one. The probes (`decisions`, `mechanisms`) hold the states and the two closures per
//! effect (how to raise the input, how to read the output); this module holds the rules.
//!
//! Rules (one place, so every probe is judged the same way):
//! - A sample is *clamped* when the output does not move at all (exactly equal before and after the
//!   nudge, which is how a `min`, `max`, `clamp`, floor, cap or threshold on the flat side shows up);
//!   clamped samples are ignored. Choose bases so that at least one sample is on the live side.
//! - `Up` / `Down`: at least one unclamped sample, and every unclamped sample moves that way.
//! - `Varies`: both signs are seen (so the reason is real, not decoration).
//! - `None`: no sample moves at all (the input is read but has no direct effect on this output).
//! - `DeclaredUnverified` effects must have no probe (a stale marker would hide a verified effect),
//!   and every `Perturbed` effect must have one.

use crate::causal::{effects_of, Node};
use crate::decisions::{DECISIONS, MECHANISMS};
use sim_types::{Basis, Effect, QuantityId, Sign};

mod decisions;
mod mechanisms;

/// An input and how to raise it in a state.
pub(crate) type Nudge<V> = (QuantityId, fn(&mut V));
/// An output and how to read it from a state.
pub(crate) type Read<V> = (QuantityId, fn(&V) -> f64);

/// What one probe needs: the states, how to raise each input, how to read each output.
pub(crate) struct Probe<V> {
    pub node: Node,
    pub bases: Vec<V>,
    /// Each raises the named input in place (a relative or absolute step the probe author chose).
    pub nudges: Vec<Nudge<V>>,
    /// Each reads the named output from the real code's result for a state.
    pub reads: Vec<Read<V>>,
}

/// The count of unclamped samples by direction, for one effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Measured {
    pub up: u32,
    pub down: u32,
    pub flat: u32,
}

#[derive(Default)]
pub(crate) struct Report {
    /// (node, from, to, measured) of every probed effect.
    pub probed: Vec<(Node, QuantityId, QuantityId, Measured)>,
    pub failures: Vec<String>,
}

fn name(n: Node) -> String {
    match n {
        Node::Decision(d) => d.as_str().to_string(),
        Node::Mechanism(m) => m.as_str().to_string(),
        Node::Quantity(q) => q.as_str().to_string(),
    }
}

/// Measure one input -> output pair over the bases.
pub(crate) fn measure<V: Clone>(p: &Probe<V>, nudge: fn(&mut V), read: fn(&V) -> f64) -> Measured {
    let mut m = Measured::default();
    for b in &p.bases {
        let before = read(b);
        let mut v = b.clone();
        nudge(&mut v);
        let after = read(&v);
        assert!(
            before.is_finite() && after.is_finite(),
            "{}: a probe read a non-finite value",
            name(p.node)
        );
        if after > before {
            m.up += 1;
        } else if after < before {
            m.down += 1;
        } else {
            m.flat += 1;
        }
    }
    m
}

impl Report {
    /// Check every effect of the probe's node against the measurement.
    pub(crate) fn check<V: Clone>(&mut self, p: &Probe<V>) {
        let who = name(p.node);
        for fx in effects_of(p.node) {
            let nudge = p.nudges.iter().find(|(q, _)| *q == fx.from).map(|x| x.1);
            let read = p.reads.iter().find(|(q, _)| *q == fx.to).map(|x| x.1);
            let label = format!("{who}: {} -> {}", fx.from.as_str(), fx.to.as_str());
            match (fx.basis, nudge, read) {
                (Basis::DeclaredUnverified(_), None, None)
                | (Basis::DeclaredUnverified(_), _, None)
                | (Basis::DeclaredUnverified(_), None, _) => {}
                (Basis::DeclaredUnverified(_), Some(_), Some(_)) => {
                    self.failures.push(format!("{label}: marked declared_unverified but the probe can measure it; drop the marker"));
                }
                (Basis::Perturbed, Some(n), Some(r)) => {
                    let m = measure(p, n, r);
                    self.judge(&label, fx, m);
                    self.probed.push((p.node, fx.from, fx.to, m));
                }
                (Basis::Perturbed, _, _) => {
                    self.failures.push(format!("{label}: declared as perturbed but the probe has no nudge or no read for it (add the probe or mark it declared_unverified with a reason)"));
                }
            }
        }
    }

    fn judge(&mut self, label: &str, fx: &Effect, m: Measured) {
        let seen = format!(
            "measured: up in {}, down in {}, flat (clamped) in {} samples",
            m.up, m.down, m.flat
        );
        let bad = match fx.sign {
            Sign::Up if m.up == 0 && m.down == 0 => Some("declared + but the output never moved (a dead edge, or every sample is clamped)"),
            Sign::Up if m.down > 0 => Some("declared + but the output fell in some sample"),
            Sign::Down if m.up == 0 && m.down == 0 => Some("declared - but the output never moved (a dead edge, or every sample is clamped)"),
            Sign::Down if m.up > 0 => Some("declared - but the output rose in some sample"),
            Sign::Varies(_) if m.up == 0 || m.down == 0 => Some("declared varies but only one direction was seen (declare the sign, or add a state that shows the other)"),
            Sign::None(_) if m.up > 0 || m.down > 0 => Some("declared no direct effect but the output moved"),
            _ => None,
        };
        if let Some(why) = bad {
            self.failures.push(format!("{label}: {why}; {seen}"));
        }
    }
}

/// Run every probe once.
pub(crate) fn run_all() -> Report {
    let mut r = Report::default();
    decisions::probe_all(&mut r);
    mechanisms::probe_all(&mut r);
    r
}

/// (declared effects, perturbed, declared_unverified) over the whole registry.
pub(crate) fn counts() -> (usize, usize, usize) {
    let mut c = (0, 0, 0);
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
            }
        }
    }
    c
}

#[test]
fn declared_signs_agree_with_the_real_code() {
    let r = run_all();
    let (all, perturbed, unverified) = counts();
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
        "effects {all}, verified by perturbation {perturbed}, declared_unverified {unverified}"
    );
}
