//! The rules of the perturbation sign test (causality graph stage 2, `docs/features/causality-graph.md` section 5).
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
use sim_params::strengths::StrengthEntry;
use sim_types::{Basis, Effect, QuantityId, Sign};
use std::fmt::Debug;

pub type Nudge<V> = (QuantityId, fn(&mut V));
pub type Read<V> = (QuantityId, fn(&V) -> f64);

/// What one probe needs: the states, how to raise each input, how to read each output.
pub struct Probe<V: Debug> {
    pub node: Node,
    pub bases: Vec<V>,
    /// Each raises the named input in place (a relative or absolute step the probe author chose).
    pub nudges: Vec<Nudge<V>>,
    /// Optional: how to read the level of an input where the nudge does not change a number in the
    /// state (a person gets a job: the count of workers). Without one the level is found from the
    /// numbers the nudge changed (`input_levels`).
    pub levels: Vec<Read<V>>,
    /// Each reads the named output from the real code's result for a state.
    pub reads: Vec<Read<V>>,
}

/// The count of unclamped samples by direction, for one effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measured {
    pub up: u32,
    pub down: u32,
    pub flat: u32,
}

pub struct Report {
    /// Which effects this report judges: `Basis::Perturbed` (in-crate probes on pure functions and
    /// decisions) or `Basis::PerturbedSystem` (whole systems on a real state, `tests/mechanism_signs`).
    pub handles: Basis,
    /// (node, from, to, measured) of every probed effect.
    pub probed: Vec<(Node, QuantityId, QuantityId, Measured)>,
    /// The size of every probed effect (stage 5), one per entry of `probed`, in the same order.
    pub strengths: Vec<StrengthEntry>,
    pub failures: Vec<String>,
}

impl Report {
    pub fn new(handles: Basis) -> Self {
        Self {
            handles,
            probed: Vec::new(),
            strengths: Vec::new(),
            failures: Vec::new(),
        }
    }
}

pub fn name(n: Node) -> String {
    match n {
        Node::Decision(d) => d.as_str().to_string(),
        Node::Mechanism(m) => m.as_str().to_string(),
        Node::Quantity(q) => q.as_str().to_string(),
    }
}

/// One sample of one effect: the input and output before and after the nudge.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    /// The input level before and after, when the nudge changed numbers in the probe's state
    /// (the sum of the numbers that changed; relative changes do not depend on the unit).
    pub input: Option<(f64, f64)>,
    pub output: (f64, f64),
}

/// The numbers inside a `Debug` print, in order (a number starts where the previous character is
/// not part of a name).
fn numbers(s: &str) -> Vec<f64> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let starts = (c.is_ascii_digit()
            || (c == b'-' && i + 1 < b.len() && b[i + 1].is_ascii_digit()))
            && (i == 0
                || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_' || b[i - 1] == b'.'));
        if !starts {
            i += 1;
            continue;
        }
        let st = i;
        i += 1;
        while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
            i += 1;
        }
        if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            let mut j = i + 1;
            if j < b.len() && (b[j] == b'-' || b[j] == b'+') {
                j += 1;
            }
            if j < b.len() && b[j].is_ascii_digit() {
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        if let Ok(x) = s[st..i].parse::<f64>() {
            out.push(x);
        }
    }
    out
}

/// The input level before and after a nudge, read from the `Debug` print of the probe's state: the
/// numbers that changed, summed (a town aggregate nudged on every firm is one input). `None` when
/// the print changed shape, no number changed (a category) or one fell (the nudge raises the input,
/// so a falling number means the sum is not that input).
fn input_levels<V: Debug>(before: &V, after: &V) -> Option<(f64, f64)> {
    let (a, b) = (
        numbers(&format!("{before:?}")),
        numbers(&format!("{after:?}")),
    );
    if a.len() != b.len() {
        return None;
    }
    let (mut x, mut y, mut any) = (0.0, 0.0, false);
    for (p, q) in a.iter().zip(&b) {
        if p.to_bits() != q.to_bits() {
            if q < p {
                // A number fell while the input was raised: the print is not one clean input.
                return None;
            }
            x += p;
            y += q;
            any = true;
        }
    }
    (any && x.is_finite() && y.is_finite()).then_some((x, y))
}

/// Measure one input -> output pair over the bases.
pub fn measure<V: Clone + Debug>(
    p: &Probe<V>,
    nudge: fn(&mut V),
    read: fn(&V) -> f64,
    level: Option<fn(&V) -> f64>,
) -> (Measured, Vec<Sample>) {
    let mut m = Measured::default();
    let mut samples = Vec::new();
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
        samples.push(Sample {
            input: match level {
                Some(l) => Some((l(b), l(&v))),
                None => input_levels(b, &v),
            },
            output: (before, after),
        });
    }
    (m, samples)
}

/// The arc elasticity of one sample: the relative change of the output over the relative change of
/// the input, each against the midpoint (so it is symmetric and defined for a step from zero).
/// `None` when the output did not move, a midpoint is zero, or the input did not change.
fn elasticity(s: &Sample) -> Option<f64> {
    let (i1, i2) = s.input?;
    let (o1, o2) = s.output;
    let (mi, mo) = ((i1 + i2) / 2.0, (o1 + o2) / 2.0);
    if i2 <= i1 || o1 == o2 || mi == 0.0 || mo == 0.0 {
        return None;
    }
    let e = ((o2 - o1) / mo.abs()) / ((i2 - i1) / mi.abs());
    e.is_finite().then_some(e)
}

/// The value at quartile `q` of 4 (0 = min, 2 = median, 4 = max) of a sorted slice, by nearest rank.
fn quartile(sorted: &[f64], q: usize) -> f64 {
    sorted[((sorted.len() - 1) * q + 2) / 4]
}

/// Round to 4 significant digits, so the checked-in file does not depend on the last bits.
fn round4(x: f64) -> f64 {
    format!("{x:.3e}").parse().unwrap_or(x)
}

/// How big the response is, from the samples of one effect. Elasticity is the unit-free measure: the
/// median over the live (not clamped) samples that have a defined one, with the quartiles as the
/// spread. If no live sample has usable levels, the raw slope (output change over input change) is
/// kept with kind `slope`; if the output is only ever 0 or 1 it is a `switch` and has no size.
pub fn summarise(
    node: Node,
    from: QuantityId,
    to: QuantityId,
    basis: &Basis,
    samples: &[Sample],
) -> StrengthEntry {
    let live: Vec<&Sample> = samples
        .iter()
        .filter(|s| s.output.0 != s.output.1)
        .collect();
    let mut e = StrengthEntry {
        via: name(node),
        from: from.as_str().to_string(),
        to: to.as_str().to_string(),
        basis: if *basis == Basis::Perturbed {
            "perturbed"
        } else {
            "system"
        }
        .to_string(),
        kind: "unmeasured".into(),
        median: 0.0,
        low: 0.0,
        high: 0.0,
        samples: 0,
        live: live.len() as u32,
        total: samples.len() as u32,
    };
    let binary = !samples.is_empty()
        && samples.iter().all(|s| {
            [s.output.0, s.output.1]
                .iter()
                .all(|v| *v == 0.0 || *v == 1.0)
        });
    if binary {
        e.kind = "switch".into();
        return e;
    }
    let mut vals: Vec<f64> = live.iter().filter_map(|s| elasticity(s)).collect();
    if vals.is_empty() {
        vals = live
            .iter()
            .filter_map(|s| {
                let (i1, i2) = s.input?;
                let k = (s.output.1 - s.output.0) / (i2 - i1);
                (i2 > i1 && k.is_finite()).then_some(k)
            })
            .collect();
        if vals.is_empty() {
            return e;
        }
        e.kind = "slope".into();
    } else {
        e.kind = "elasticity".into();
    }
    vals.sort_by(|a, b| a.total_cmp(b));
    e.samples = vals.len() as u32;
    e.median = round4(quartile(&vals, 2));
    e.low = round4(quartile(&vals, 1));
    e.high = round4(quartile(&vals, 3));
    e
}

impl Report {
    /// Check every effect of the probe's node that this report handles against the measurement.
    pub fn check<V: Clone + Debug>(&mut self, p: &Probe<V>) {
        let who = name(p.node);
        for fx in effects_of(p.node) {
            let nudge = p.nudges.iter().find(|(q, _)| *q == fx.from).map(|x| x.1);
            let read = p.reads.iter().find(|(q, _)| *q == fx.to).map(|x| x.1);
            let label = format!("{who}: {} -> {}", fx.from.as_str(), fx.to.as_str());
            match (fx.basis, nudge, read) {
                (b, Some(n), Some(r)) if b == self.handles => {
                    let level = p.levels.iter().find(|(q, _)| *q == fx.from).map(|x| x.1);
                    let (m, samples) = measure(p, n, r, level);
                    self.judge(&label, fx, m);
                    self.probed.push((p.node, fx.from, fx.to, m));
                    self.strengths
                        .push(summarise(p.node, fx.from, fx.to, &b, &samples));
                }
                (b, _, _) if b == self.handles => {
                    self.failures.push(format!("{label}: declared {b:?} but the probe has no nudge or no read for it (add the probe or mark it declared_unverified with a reason)"));
                }
                (Basis::DeclaredUnverified(_), Some(_), Some(_)) => {
                    self.failures.push(format!("{label}: marked declared_unverified but the probe can measure it; drop the marker"));
                }
                _ => {}
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
