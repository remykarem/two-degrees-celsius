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
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use sim_types::PlantKind;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissionsLine {
    pub firms: f64,
    pub households: f64,
    pub plants: f64,
    /// `firms + households + plants`.
    pub gross_total: f64,
    /// Forest sink, negative.
    pub sink: f64,
    /// `grossTotal + sink`.
    pub net: f64,
    /// The model's own net figure for the same period (the state's `net_emissions`, summed over
    /// the ticks of a turn). `net` must equal it; `EmissionsBreakdown.reconciles` says so.
    pub headline: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cumulative {
    pub tonnes: f64,
    pub budget: f64,
    pub remaining: f64,
    /// Share of the budget used, 0..=1 (`tonnes / budget`).
    pub share: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Firm,
    Households,
    Plant,
    Sink,
    FireSinkLoss,
}

/// One by-source line of the latest month.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceLine {
    pub kind: SourceKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    pub label: String,
    /// Tonnes this month. The sink is negative; `fire_sink_loss` is positive but informational.
    pub tonnes: f64,
    /// True for `fire_sink_loss`: shown for information, never part of a sum.
    pub informational: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissionsBreakdown {
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

fn line(ticks: &[&TickRecord]) -> EmissionsLine {
    let mut l = EmissionsLine::default();
    for t in ticks {
        l.firms += t.firms_total();
        l.households += t.households;
        l.plants += t.plants_total();
        l.sink += t.sink;
        l.headline += t.net_state;
    }
    l.gross_total = l.firms + l.households + l.plants;
    l.net = l.gross_total + l.sink;
    l
}

fn line_reconciles(l: &EmissionsLine) -> bool {
    (l.net - l.headline).abs() <= tolerance(l.gross_total)
}

pub fn build(sim: &Sim, ticks: &[TickRecord]) -> EmissionsBreakdown {
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
            month: EmissionsLine::default(),
            turn: EmissionsLine::default(),
            cumulative,
            reconciles: true,
            note: NOTE.to_string(),
            month_lines: Vec::new(),
            turn_number: s.turn,
            turn_months: 0,
        };
    };
    let in_turn: Vec<&TickRecord> = ticks.iter().filter(|t| t.turn == last.turn).collect();
    let month = line(&[last]);
    let turn = line(&in_turn);
    let names = firm_names(&s.firms);
    let mut lines: Vec<SourceLine> = last
        .firms
        .iter()
        .map(|(id, t)| SourceLine {
            kind: SourceKind::Firm,
            id: Some(*id),
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
        label: "Households (direct footprint)".to_string(),
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
            label,
            tonnes: *t,
            informational: false,
        });
    }
    lines.push(SourceLine {
        kind: SourceKind::Sink,
        id: None,
        label: "Forest sink (absorbed)".to_string(),
        tonnes: last.sink,
        informational: false,
    });
    if last.fire_sink_loss != 0.0 {
        lines.push(SourceLine {
            kind: SourceKind::FireSinkLoss,
            id: None,
            label: "Sink lost to fire (information only, not in any sum)".to_string(),
            tonnes: last.fire_sink_loss,
            informational: true,
        });
    }
    let gross_state_ok =
        (last.gross_state - month.gross_total).abs() <= tolerance(month.gross_total);
    let reconciles = line_reconciles(&month) && line_reconciles(&turn) && gross_state_ok;
    EmissionsBreakdown {
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

const NOTE: &str = "Gross emissions = firms (direct) + households (direct) + power plants. Electricity used by firms and households is counted once, at the plants that burn the fuel, so it is not added again to firms or households. Net = gross minus the forest sink. The month line is the latest month; the turn line adds the months of the latest turn. The fire line shows sink lost to fire and is not a term of the sum.";
