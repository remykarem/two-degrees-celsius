//! A turn plan: the player's commands per turn number, read from JSON. Pure parsing, no I/O.

use serde::{Deserialize, Serialize};
use sim_types::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpec {
    pub kind: InfraKind,
    /// `[x, y]`.
    pub tile: [u8; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveSpec {
    pub tile: [u8; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanTurn {
    /// Turn number, 1-based.
    pub turn: u32,
    /// Lever settings: name -> value (dollars for money, fractions for rates).
    #[serde(default)]
    pub set: BTreeMap<String, f64>,
    #[serde(default)]
    pub switch: Option<EconomicSystem>,
    #[serde(default)]
    pub place: Vec<PlaceSpec>,
    #[serde(default)]
    pub remove: Vec<RemoveSpec>,
    /// A raw command, exactly as the engine's `Command` type serialises.
    #[serde(default)]
    pub command: Option<Command>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub turns: Vec<PlanTurn>,
}

pub fn lever(name: &str, v: f64) -> Result<PolicyChange, String> {
    let c = Cents::from_dollars;
    Ok(match name {
        "carbon_price" => PolicyChange::CarbonPrice(c(v)),
        "tax_wage" => PolicyChange::TaxWage(Rate(v)),
        "tax_profit" => PolicyChange::TaxProfit(Rate(v)),
        "tax_capital" => PolicyChange::TaxCapital(Rate(v)),
        "wealth_tax" => PolicyChange::WealthTax(Rate(v)),
        "wealth_threshold" => PolicyChange::WealthThreshold(c(v)),
        "base_rate" => PolicyChange::BaseRate(Rate(v)),
        "min_wage" => PolicyChange::MinWage(c(v)),
        "benefit" => PolicyChange::Benefit(c(v)),
        "ubi" => PolicyChange::Ubi(c(v)),
        "green_subsidy" => PolicyChange::GreenSubsidy(Rate(v)),
        "campaign_cap" => PolicyChange::CampaignCap(Rate(v)),
        "emissions_cap" => PolicyChange::EmissionsCap(Tonnes(v)),
        "commons_harvest" => PolicyChange::CommonsHarvest(Rate(v)),
        "bank_capital_ratio" => PolicyChange::BankCapitalRatio(Rate(v)),
        "bailout" => PolicyChange::Bailout(v > 0.5),
        "public_spending" => PolicyChange::PublicSpending(c(v)),
        "job_guarantee" => PolicyChange::JobGuarantee(Rate(v)),
        "commons_community_cap" => {
            PolicyChange::CommonsAccess(CommonsAccess::Community { cap: Rate(v) })
        }
        other => return Err(format!("unknown policy lever '{other}'")),
    })
}

impl Plan {
    pub fn from_json(text: &str) -> Result<Plan, String> {
        let s: Plan = serde_json::from_str(text).map_err(|e| format!("plan: {e}"))?;
        for t in &s.turns {
            if t.turn == 0 {
                return Err("plan: turns are numbered from 1".into());
            }
            for (k, v) in &t.set {
                lever(k, *v)?;
            }
        }
        Ok(s)
    }

    /// Commands to submit in the given turn, in plan order.
    pub fn commands_at(&self, turn: u32) -> Vec<Command> {
        let mut out = Vec::new();
        for a in self.turns.iter().filter(|a| a.turn == turn) {
            for (k, v) in &a.set {
                if let Ok(c) = lever(k, *v) {
                    out.push(Command::SetPolicy(c));
                }
            }
            if let Some(sys) = a.switch {
                out.push(Command::SwitchSystem(sys));
            }
            for pl in &a.place {
                out.push(Command::Place {
                    kind: pl.kind,
                    tile: Tile::new(pl.tile[0], pl.tile[1]),
                });
            }
            for r in &a.remove {
                out.push(Command::Remove {
                    tile: Tile::new(r.tile[0], r.tile[1]),
                });
            }
            if let Some(c) = a.command {
                out.push(c);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_and_switch() {
        let s = Plan::from_json(
            r#"{"turns":[{"turn":20,"set":{"carbon_price":100}},{"turn":5,"switch":"Regulated"}]}"#,
        )
        .unwrap();
        assert_eq!(
            s.commands_at(20),
            vec![Command::SetPolicy(PolicyChange::CarbonPrice(Cents(10_000)))]
        );
        assert_eq!(
            s.commands_at(5),
            vec![Command::SwitchSystem(EconomicSystem::Regulated)]
        );
        assert!(s.commands_at(7).is_empty());
    }

    #[test]
    fn rejects_unknown_lever_and_turn_zero() {
        assert!(Plan::from_json(r#"{"turns":[{"turn":1,"set":{"magic":1}}]}"#).is_err());
        assert!(Plan::from_json(r#"{"turns":[{"turn":0}]}"#).is_err());
    }
}
