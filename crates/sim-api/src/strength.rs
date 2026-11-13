//! Contract v2.14: how big each causal link is (causality graph stage 5,
//! `docs/features/causality-graph.md` section 16). The numbers are measured by the sign-test
//! harness and checked in as `content/strengths.json` (generated; a staleness test in the harness);
//! the words and thresholds are in `content/decisions.json` `causal.strength`. This module joins
//! them to the edges and combines them along routes. The web only draws what it is given.
use crate::decisions::{EdgeStrength, NetStrength, StrengthKind, StrengthWord};
use serde::Deserialize;
use sim_model::causal::{NetSign, SignedPath};
use sim_params::strengths::{self as st, StrengthEntry};
use sim_types::QuantityId;

pub const STRENGTHS_JSON: &str = include_str!("../../../content/strengths.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrengthDef {
    /// Elasticity at or below which the thickness weight is 0, and at or above which it is 1
    /// (the weight is on a log scale between them).
    pub floor: f64,
    pub ceiling: f64,
    pub words: Vec<WordDef>,
    pub kinds: Vec<KindDef>,
    pub edge_elasticity: String,
    pub edge_slope: String,
    pub edge_switch: String,
    pub edge_unmeasured: String,
    pub depends: String,
    pub unverified: String,
    pub net: String,
    pub net_cancel: String,
    pub net_partial: String,
    pub sign_plus: String,
    pub sign_minus: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordDef {
    pub id: String,
    pub label: String,
    pub min: f64,
    pub text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindDef {
    pub id: String,
    pub label: String,
    pub text: String,
}

/// The measured sizes, looked up by (via, from, to).
pub struct Strengths {
    entries: Vec<StrengthEntry>,
}

impl Strengths {
    pub fn load() -> Result<Self, String> {
        Ok(Self {
            entries: st::parse(STRENGTHS_JSON)?.entries,
        })
    }

    pub fn entries(&self) -> &[StrengthEntry] {
        &self.entries
    }

    pub fn get(&self, via: &str, from: QuantityId, to: QuantityId) -> Option<&StrengthEntry> {
        self.entries
            .iter()
            .find(|e| e.via == via && e.from == from.as_str() && e.to == to.as_str())
    }
}

/// A number for a sentence: two significant digits below 10, none above ("0.42", "1.4", "12", "<0.01" for less).
pub fn fmt_num(x: f64) -> String {
    let a = x.abs();
    if a == 0.0 {
        "0".into()
    } else if a < 0.01 {
        "<0.01".into()
    } else if a < 1.0 {
        format!("{a:.2}")
    } else if a < 10.0 {
        format!("{a:.1}")
    } else {
        format!("{a:.0}")
    }
}

impl StrengthDef {
    pub fn words(&self) -> Vec<StrengthWord> {
        self.words
            .iter()
            .map(|w| StrengthWord {
                id: w.id.clone(),
                label: w.label.clone(),
                min: w.min,
                weight: self.weight(w.min),
                text: w.text.clone(),
            })
            .collect()
    }

    pub fn kinds(&self) -> Vec<crate::decisions::CausalRole> {
        self.kinds
            .iter()
            .map(|k| crate::decisions::CausalRole {
                id: k.id.clone(),
                label: k.label.clone(),
                text: k.text.clone(),
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(self.floor > 0.0 && self.ceiling > self.floor) {
            return Err("causal.strength: need 0 < floor < ceiling".into());
        }
        let mut prev = -1.0;
        for w in &self.words {
            if w.min <= prev {
                return Err("causal.strength.words: min must be strictly increasing".into());
            }
            prev = w.min;
        }
        if self.words.first().map(|w| w.min) != Some(0.0) {
            return Err("causal.strength.words: the first word must start at 0".into());
        }
        for k in ["elasticity", "slope", "switch", "unmeasured"] {
            if !self.kinds.iter().any(|x| x.id == k) {
                return Err(format!("causal.strength.kinds has no {k}"));
            }
        }
        Ok(())
    }

    /// The word id for an elasticity magnitude: the last word whose `min` it reaches.
    pub fn word_for(&self, magnitude: f64) -> &WordDef {
        self.words
            .iter()
            .rev()
            .find(|w| magnitude >= w.min)
            .unwrap_or(&self.words[0])
    }

    /// 0 to 1 for line thickness: the magnitude on a log scale from `floor` to `ceiling`.
    pub fn weight(&self, magnitude: f64) -> f64 {
        let m = magnitude.max(self.floor);
        let w = libm::log10(m / self.floor) / libm::log10(self.ceiling / self.floor);
        (w.clamp(0.0, 1.0) * 1000.0).round() / 1000.0
    }

    /// The strength of one link with a declared sign of `up` or `down` (the caller leaves
    /// `varies`, `none` and unverified links without one).
    pub fn edge(&self, e: &StrengthEntry) -> EdgeStrength {
        let kind = match e.kind.as_str() {
            "elasticity" => StrengthKind::Elasticity,
            "slope" => StrengthKind::Slope,
            "switch" => StrengthKind::Switch,
            _ => StrengthKind::Unmeasured,
        };
        let sized = matches!(kind, StrengthKind::Elasticity | StrengthKind::Slope);
        let word = (kind == StrengthKind::Elasticity).then(|| self.word_for(e.median.abs()));
        let template = match kind {
            StrengthKind::Elasticity => &self.edge_elasticity,
            StrengthKind::Slope => &self.edge_slope,
            StrengthKind::Switch => &self.edge_switch,
            StrengthKind::Unmeasured => &self.edge_unmeasured,
        };
        let text = template
            .replace(
                "{word}",
                &word.map(|w| w.label.to_lowercase()).unwrap_or_default(),
            )
            .replace("{value}", &fmt_num(e.median))
            .replace("{pct}", &fmt_num(e.median * 10.0))
            .replace("{low}", &fmt_num(e.low))
            .replace("{high}", &fmt_num(e.high))
            .replace("{n}", &e.samples.to_string())
            .replace("{of}", &e.total.to_string());
        EdgeStrength {
            kind,
            value: sized.then_some(e.median),
            low: sized.then_some(e.low),
            high: sized.then_some(e.high),
            samples: e.samples,
            live: e.live,
            total: e.total,
            label: if sized {
                fmt_num(e.median)
            } else {
                String::new()
            },
            word: word.map(|w| w.id.clone()),
            weight: (kind == StrengthKind::Elasticity).then(|| self.weight(e.median.abs())),
            text,
        }
    }

    /// The overall size of a cause on the focus: for each route of at most three steps the product of
    /// its links' elasticities (the chain rule), summed over routes with their signs, so routes that
    /// push opposite ways cancel. A route with a link that has no elasticity (it `varies`, is a
    /// switch, is not sized, or is declared unverified) is left out and `partial` says so. This is
    /// the first-order effect of a small change, through the routes within the depth only: not a
    /// forecast, not through longer routes or feedback loops, and each link was measured at its own
    /// states, not at one common state.
    pub fn net(
        &self,
        paths: &[SignedPath],
        lookup: &dyn Fn(&str, QuantityId, QuantityId) -> Option<f64>,
    ) -> Option<NetStrength> {
        let (mut sum, mut gross, mut used, mut left_out) = (0.0, 0.0, 0usize, false);
        let (mut up, mut down) = (false, false);
        for p in paths {
            let mut prod = 1.0;
            let mut ok = p.sign != NetSign::Varies;
            for (i, via) in p.via.iter().enumerate() {
                let name = crate::decisions::node_ref(*via);
                match lookup(&name, p.quantities[i], p.quantities[i + 1]) {
                    Some(x) if ok => prod *= x,
                    _ => ok = false,
                }
            }
            if ok && prod.is_finite() {
                sum += prod;
                gross += prod.abs();
                used += 1;
                up |= prod > 0.0;
                down |= prod < 0.0;
            } else {
                left_out = true;
            }
        }
        if used == 0 {
            return None;
        }
        let word = self.word_for(sum.abs());
        Some(NetStrength {
            value: sum,
            gross,
            routes: used as u32,
            partial: left_out,
            cancels: up && down,
            label: format!(
                "{}{}",
                if sum < 0.0 {
                    &self.sign_minus
                } else {
                    &self.sign_plus
                },
                fmt_num(sum)
            ),
            gross_label: fmt_num(gross),
            word: word.id.clone(),
            weight: self.weight(sum.abs()),
        })
    }
}
