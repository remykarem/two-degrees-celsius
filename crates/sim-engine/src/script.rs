//! Scripted player actions: "set carbon price at tick 60". Pure parsing, no I/O.

use serde::{Deserialize, Serialize};
use sim_types::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlaceSpec {
    pub kind: Option<InfraKind>,
    pub count: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub tick: u32,
    /// Friendly lever settings: name -> value (dollars for money, fractions for rates).
    #[serde(default)]
    pub set: BTreeMap<String, f64>,
    #[serde(default)]
    pub switch: Option<EconomicSystem>,
    #[serde(default)]
    pub place: Option<PlaceSpec>,
    /// A raw command, exactly as the engine's `Command` type serialises.
    #[serde(default)]
    pub command: Option<Command>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub actions: Vec<Action>,
}

fn lever(name: &str, v: f64) -> Result<PolicyChange, String> {
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
        "public_spending" => PolicyChange::PublicSpending(Rate(v)),
        "commons_community_cap" => {
            PolicyChange::CommonsAccess(CommonsAccess::Community { cap: Rate(v) })
        }
        other => return Err(format!("unknown policy lever '{other}'")),
    })
}

impl Script {
    pub fn from_json(text: &str) -> Result<Script, String> {
        let s: Script = serde_json::from_str(text).map_err(|e| format!("script: {e}"))?;
        s.commands_at_all()?;
        Ok(s)
    }

    fn commands_at_all(&self) -> Result<(), String> {
        for a in &self.actions {
            for (k, v) in &a.set {
                lever(k, *v)?;
            }
        }
        Ok(())
    }

    /// Commands to run at the start of the given tick, in script order.
    pub fn commands_at(&self, tick: u32) -> Vec<Command> {
        let mut out = Vec::new();
        for a in self.actions.iter().filter(|a| a.tick == tick) {
            for (k, v) in &a.set {
                if let Ok(c) = lever(k, *v) {
                    out.push(Command::SetPolicy(c));
                }
            }
            if let Some(sys) = a.switch {
                out.push(Command::SwitchSystem(sys));
            }
            if let Some(p) = &a.place {
                if let Some(kind) = p.kind {
                    out.push(Command::Place {
                        kind,
                        count: p.count.unwrap_or(1),
                    });
                }
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
        let s = Script::from_json(
            r#"{"actions":[{"tick":60,"set":{"carbon_price":100}},{"tick":12,"switch":"Regulated"}]}"#,
        )
        .unwrap();
        assert_eq!(
            s.commands_at(60),
            vec![Command::SetPolicy(PolicyChange::CarbonPrice(Cents(10_000)))]
        );
        assert_eq!(
            s.commands_at(12),
            vec![Command::SwitchSystem(EconomicSystem::Regulated)]
        );
        assert!(s.commands_at(5).is_empty());
    }

    #[test]
    fn rejects_unknown_lever() {
        assert!(Script::from_json(r#"{"actions":[{"tick":1,"set":{"magic":1}}]}"#).is_err());
    }
}
