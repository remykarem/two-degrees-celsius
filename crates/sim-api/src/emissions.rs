//! The emissions tally (contract v1.6): by-source lines that add up to the headline by
//! construction. Every figure comes from the model's own per-tick trace (recorded in the
//! wrapper log), never from a mix of windows or sign conventions.
//!
//! The mismatch the player found had these causes (see `docs/sim-api-notes.md`): the headline
//! showed the latest month's GROSS emissions while the by-source bars summed the whole turn's
//! trace (three months), with the sink as a negative bar (so the bars summed to a three-month NET
//! total), and the informational `fire_sink_loss` line was drawn as a positive source. Lines
//! here are per month or per turn on the same basis, the sink is a negative line, and the fire
//! line is flagged informational and left out of every sum.
use crate::log::TickRecord;
use crate::snapshot::firm_names;
use crate::sources::{EmissionCategory, EmissionSourceTotal, Sources};
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types::PlantKind;

/// One period (the latest month, or the months of the latest turn). Contract v2.0: the sources
/// are a list (`content/ui.json` `emission_sources` names, labels and categorises them), not
/// fixed fields. `sources` sum to `gross_total`; a source that is zero may be left out.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissionsLine {
    pub sources: Vec<EmissionSourceTotal>,
    /// The sum of `sources`.
    pub gross_total: f64,
    /// Forest sink, negative.
    pub sink: f64,
    /// `grossTotal + sink`.
    pub net: f64,
    /// The model's own net figure for the same period (the state's `net_emissions`, summed over
    /// the ticks of a turn). `net` must equal it; `EmissionsBreakdown.reconciles` says so.
    pub headline: f64,
}

/// Totals of a period before they become a wire line.
#[derive(Clone, Copy, Debug, Default)]
struct Acc {
    firms: f64,
    households: f64,
    plants: f64,
    aviation: f64,
    sink: f64,
    headline: f64,
}

impl Acc {
    fn tonnes(&self, id: &str) -> f64 {
        match id {
            "firms" => self.firms,
            "households" => self.households,
            "plants" => self.plants,
            "aviation" => self.aviation,
            _ => 0.0, // ids are validated against EMISSION_IDS at load
        }
    }

    fn gross(&self) -> f64 {
        self.firms + self.households + self.plants + self.aviation
    }

    fn line(&self, sources: &Sources) -> EmissionsLine {
        let gross = self.gross();
        EmissionsLine {
            sources: sources.emission_line(|id| self.tonnes(id)),
            gross_total: gross,
            sink: self.sink,
            net: gross + self.sink,
            headline: self.headline,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cumulative {
    pub tonnes: f64,
    pub budget: f64,
    pub remaining: f64,
    /// Share of the budget used, 0..=1 (`tonnes / budget`).
    pub share: f64,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Firm,
    Households,
    Plant,
    /// Contract v1.9.
    Aviation,
    Sink,
    FireSinkLoss,
}

/// One by-source line of the latest month.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLine {
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub id: Option<u32>,
    pub label: String,
    /// The roll-up category of the source (`content/ui.json`); none for the sink and the fire line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub category: Option<EmissionCategory>,
    /// Tonnes this month. The sink is negative; `fire_sink_loss` is positive but informational.
    pub tonnes: f64,
    /// True for `fire_sink_loss`: shown for information, never part of a sum.
    pub informational: bool,
}

/// Contract v1.8: where the budget goes at the current net rate. `per_turn` is the latest month's
/// net emissions (never below zero) times `run.ticks_per_turn`; `run_out_turn` is the turn on
/// which the remaining budget would be used up at that rate (`None` when nothing is emitted).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Runway {
    pub per_turn: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub run_out_turn: Option<f64>,
    /// True when the run-out turn falls on or before the turn after the last one.
    pub runs_out_before_end: bool,
}

fn runway(sim: &Sim) -> Runway {
    let s = &sim.state;
    let per_turn = s.nature.net_emissions.0.max(0.0) * f64::from(sim.params.run.ticks_per_turn);
    let last = sim.params.run.turns;
    let now = s.turn.min(last + 1);
    let remaining = s.indicators.remaining_budget.0;
    let run_out_turn = (per_turn > 0.0).then(|| f64::from(now) + remaining / per_turn);
    Runway {
        per_turn,
        run_out_turn,
        runs_out_before_end: run_out_turn.is_some_and(|t| t <= f64::from(last + 1)),
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissionsBreakdown {
    /// Additive (contract v1.8).
    pub runway: Runway,
    pub month: EmissionsLine,
    pub turn: EmissionsLine,
    pub cumulative: Cumulative,
    pub reconciles: bool,
    pub note: String,
    /// Additive: the latest month by source (sums to `month.grossTotal` and `month.sink`).
    pub month_lines: Vec<SourceLine>,
    /// Additive: the turn the `turn` line covers and how many of its months have been played.
    pub turn_number: u32,
    pub turn_months: u32,
}

/// Tolerance for the tally: absolute 1e-6 tonnes plus 1e-9 of the gross total.
pub fn tolerance(gross: f64) -> f64 {
    1e-6 + 1e-9 * gross.abs()
}

fn acc(ticks: &[&TickRecord]) -> Acc {
    let mut a = Acc::default();
    for t in ticks {
        a.firms += t.firms_total();
        a.households += t.households;
        a.plants += t.plants_total();
        a.aviation += t.aviation;
        a.sink += t.sink;
        a.headline += t.net_state;
    }
    a
}

fn line_reconciles(l: &EmissionsLine) -> bool {
    let listed: f64 = l.sources.iter().map(|s| s.tonnes).sum();
    (l.net - l.headline).abs() <= tolerance(l.gross_total)
        && (listed - l.gross_total).abs() <= tolerance(l.gross_total)
}

pub fn build(sim: &Sim, ticks: &[TickRecord], sources: &Sources) -> EmissionsBreakdown {
    let s = &sim.state;
    let b0 = sim.params.nature.budget_b0;
    let cumulative = Cumulative {
        tonnes: s.nature.cumulative.0,
        budget: b0,
        remaining: s.indicators.remaining_budget.0,
        share: if b0 > 0.0 {
            (s.nature.cumulative.0 / b0).clamp(0.0, 1.0)
        } else {
            0.0
        },
    };
    let Some(last) = ticks.last() else {
        return EmissionsBreakdown {
            runway: runway(sim),
            month: Acc::default().line(sources),
            turn: Acc::default().line(sources),
            cumulative,
            reconciles: true,
            note: NOTE.to_string(),
            month_lines: Vec::new(),
            turn_number: s.turn,
            turn_months: 0,
        };
    };
    let in_turn: Vec<&TickRecord> = ticks.iter().filter(|t| t.turn == last.turn).collect();
    let month = acc(&[last]).line(sources);
    let turn = acc(&in_turn).line(sources);
    let names = firm_names(&s.firms);
    let cat = |id: &str| {
        sources
            .emission
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.category)
    };
    let mut lines: Vec<SourceLine> = last
        .firms
        .iter()
        .map(|(id, t)| SourceLine {
            kind: SourceKind::Firm,
            id: Some(*id),
            category: cat("firms"),
            label: names
                .get(*id as usize)
                .cloned()
                .unwrap_or_else(|| format!("Firm {id}")),
            tonnes: *t,
            informational: false,
        })
        .collect();
    lines.push(SourceLine {
        kind: SourceKind::Households,
        id: None,
        category: cat("households"),
        label: "Homes (heating, transport)".to_string(),
        tonnes: last.households,
        informational: false,
    });
    for (id, t) in &last.plants {
        let plant = s.plants.iter().find(|p| p.id.0 == *id);
        let label = match plant {
            Some(p) => format!(
                "{} plant at ({},{})",
                if p.kind == PlantKind::Fossil {
                    "Fossil"
                } else {
                    "Renewable"
                },
                p.tile.x,
                p.tile.y
            ),
            None => format!("Plant {id}"),
        };
        lines.push(SourceLine {
            kind: SourceKind::Plant,
            id: Some(*id),
            category: cat("plants"),
            label,
            tonnes: *t,
            informational: false,
        });
    }
    if last.aviation != 0.0 {
        lines.push(SourceLine {
            kind: SourceKind::Aviation,
            id: None,
            category: cat("aviation"),
            label: "Private jet flight".to_string(),
            tonnes: last.aviation,
            informational: false,
        });
    }
    lines.push(SourceLine {
        kind: SourceKind::Sink,
        id: None,
        category: None,
        label: "Forest sink (absorbed)".to_string(),
        tonnes: last.sink,
        informational: false,
    });
    if last.fire_sink_loss != 0.0 {
        lines.push(SourceLine {
            kind: SourceKind::FireSinkLoss,
            id: None,
            category: None,
            label: "Sink lost to fire (information only, not in any sum)".to_string(),
            tonnes: last.fire_sink_loss,
            informational: true,
        });
    }
    let gross_state_ok =
        (last.gross_state - month.gross_total).abs() <= tolerance(month.gross_total);
    let reconciles = line_reconciles(&month) && line_reconciles(&turn) && gross_state_ok;
    EmissionsBreakdown {
        runway: runway(sim),
        month,
        turn,
        cumulative,
        reconciles,
        note: NOTE.to_string(),
        month_lines: lines,
        turn_number: last.turn,
        turn_months: in_turn.len() as u32,
    }
}

const NOTE: &str = "Gross emissions = firms (direct) + households (direct) + private jet flights + power plants. Electricity used by firms and households is counted once, at the plants that burn the fuel, so it is not added again to firms or households. Net = gross minus the forest sink. The month line is the latest month; the turn line adds the months of the latest turn. The fire line shows sink lost to fire and is not a term of the sum.";
