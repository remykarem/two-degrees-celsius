//! The decision registry: every explicit decision point, declared in one place
//! (`docs/features/decision-points.md`). Each `DecisionSpec` sits next to its `decide_*` function
//! in the system's module; this list gathers them so the wire, the Agents text and a future
//! causality view are generated from code. Add a decision: write the fn and its spec, add the spec
//! here, add its `DecisionId` and any new `QuantityId` in `sim-types`, and its words in
//! `content/decisions.json` (`docs/how-to.md`, "Add a decision").

use sim_types::{DecisionId, DecisionSpec, MechanismId, MechanismSpec, QuantityId};

pub const DECISIONS: &[DecisionSpec] = &[
    crate::labour::FIRM_HIRE,
    crate::labour::FIRM_FIRE,
    crate::labour::FIRM_WAGE,
    crate::goods::FIRM_PRICE,
    crate::investment::FIRM_GREEN,
    crate::investment::FIRM_CAPEX,
    crate::income::FIRM_PAYOUT,
    crate::credit::FIRM_EXIT,
    crate::class::HOUSEHOLD_FOUND_FIRM,
    crate::goods::HOUSEHOLD_SPEND,
    crate::goods::HOUSEHOLD_SHOP,
    crate::goods::FIRM_EXPECT,
    crate::lobbying::HOUSEHOLD_LOBBY,
    crate::credit::BANK_HEADROOM,
    crate::credit::BANK_LEND,
    crate::credit::BANK_SPREAD,
    crate::credit::BANK_DIVIDEND,
    crate::credit::BANK_RESCUE,
    crate::government::GOVERNMENT_FUND,
    crate::government::GOVERNMENT_CUT,
    crate::government::GOVERNMENT_LEGITIMACY,
    crate::estate::ESTATE_PILOT,
    crate::estate::ESTATE_FLIGHT,
    crate::nature::NATURE_ENCLOSURE,
];

/// Every mechanism: plain physics and accounting code that links the decisions' quantities (causality
/// graph stage 1, `docs/features/causality-graph.md` C5 and C7). A mechanism is declared next to
/// the code it describes and changes no behaviour; add one with `docs/how-to.md` "Add a mechanism".
pub const MECHANISMS: &[MechanismSpec] = &[
    // Production and the factors of capacity (DP-2).
    crate::production::PRODUCTION_TECHNOLOGY,
    crate::production::TECHNOLOGY_GROWTH,
    crate::investment::CAPITAL_ACCUMULATION,
    crate::production::EFFECTIVE_LABOUR_SUPPLY,
    crate::production::FARM_HEALTH_FACTOR,
    crate::production::CLIMATE_DAMAGE,
    crate::production::DROUGHT_LOSS,
    crate::production::FIRE_SMOKE,
    crate::production::TRANSITION_LOSS,
    crate::production::PRODUCTION_OUTPUT,
    crate::production::FOOD_CAP,
    crate::production::INVENTORY_STOCK,
    // Electricity and emissions.
    crate::production::ELECTRICITY_DEMAND,
    crate::production::ELECTRICITY_RATIONING,
    crate::production::GRID_INTENSITY_MIX,
    crate::production::FIRM_EMISSIONS,
    crate::production::PLANT_EMISSIONS_OUTPUT,
    crate::nature::EMISSIONS_ACCOUNTING,
    crate::nature::CARBON_BUDGET,
    // Nature's stocks.
    crate::nature::FOREST_SINK_UPTAKE,
    crate::nature::FOREST_STOCK_CHANGE,
    crate::nature::COMMONS_STOCK_CHANGE,
    // Weather.
    crate::weather::FIRE_BURNS_FOREST,
    crate::weather::CLIMATE_STRESS_CURVE,
    crate::weather::DROUGHT_ONSET,
    crate::weather::FIRE_IGNITION,
    // Wellbeing and the household record.
    crate::wellbeing::WELLBEING_SCORE,
    crate::wellbeing::INEQUALITY_GINI,
    crate::wellbeing::LAST_DISPOSABLE_RECORD,
    // Income, taxes and goods settlement.
    crate::income::WAGE_PAYMENT,
    crate::income::HOUSEHOLD_INCOME_FLOWS,
    crate::income::FIRM_PROFIT,
    crate::government::TAX_COLLECTION,
    crate::goods::GOODS_SETTLEMENT,
    crate::goods::UNIT_COST_BUILDUP,
    // Housing and labour matching.
    crate::goods::HOUSING_RENT,
    crate::goods::HOUSING_MOVES,
    crate::labour::LABOUR_MATCHING,
    crate::labour::UNEMPLOYMENT_RATE,
];

/// The spec of a mechanism.
pub fn mechanism(id: MechanismId) -> Option<&'static MechanismSpec> {
    MECHANISMS.iter().find(|m| m.id == id)
}

/// The spec of a decision.
pub fn spec(id: DecisionId) -> Option<&'static DecisionSpec> {
    DECISIONS.iter().find(|d| d.id == id)
}

/// The decisions that read or write a quantity: the edges of the causal graph, by quantity.
pub fn readers_of(q: QuantityId) -> impl Iterator<Item = &'static DecisionSpec> {
    DECISIONS.iter().filter(move |d| d.inputs.contains(&q))
}
pub fn writers_of(q: QuantityId) -> impl Iterator<Item = &'static DecisionSpec> {
    DECISIONS.iter().filter(move |d| d.outputs.contains(&q))
}
/// The mechanisms that read or write a quantity.
pub fn mechanism_readers_of(q: QuantityId) -> impl Iterator<Item = &'static MechanismSpec> {
    MECHANISMS.iter().filter(move |m| m.inputs.contains(&q))
}
pub fn mechanism_writers_of(q: QuantityId) -> impl Iterator<Item = &'static MechanismSpec> {
    MECHANISMS.iter().filter(move |m| m.outputs.contains(&q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_decision_id_has_exactly_one_spec_and_ids_are_unique() {
        for id in DecisionId::ALL {
            assert_eq!(
                DECISIONS.iter().filter(|d| d.id == *id).count(),
                1,
                "{}",
                id.as_str()
            );
        }
        assert_eq!(DECISIONS.len(), DecisionId::ALL.len());
        let names: BTreeSet<_> = DecisionId::ALL.iter().map(|i| i.as_str()).collect();
        assert_eq!(names.len(), DecisionId::ALL.len(), "wire names are unique");
        let q: BTreeSet<_> = QuantityId::ALL.iter().map(|i| i.as_str()).collect();
        assert_eq!(q.len(), QuantityId::ALL.len());
    }

    #[test]
    fn specs_are_well_formed() {
        for d in DECISIONS {
            assert!(!d.inputs.is_empty() && !d.outputs.is_empty(), "{:?}", d.id);
            assert!(!d.rule.is_empty());
            let p: BTreeSet<_> = d.params.iter().collect();
            assert_eq!(p.len(), d.params.len(), "{:?} lists a param twice", d.id);
            let i: BTreeSet<_> = d.inputs.iter().collect();
            assert_eq!(i.len(), d.inputs.len(), "{:?} lists an input twice", d.id);
        }
    }

    /// Hidden state is forbidden: a decision may depend only on its view and params. No
    /// `static` item or `thread_local!` anywhere in the model crate.
    #[test]
    fn no_hidden_state_in_the_model_crate() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        for e in std::fs::read_dir(dir).unwrap() {
            let path = e.unwrap().path();
            let src = std::fs::read_to_string(&path).unwrap();
            for (n, line) in src.lines().enumerate() {
                let l = line.trim_start();
                let tl = concat!("thread_", "local!");
                let sm = concat!("static", " mut ");
                let bad = !l.starts_with("//")
                    && (l.starts_with("static ")
                        || l.starts_with("pub static ")
                        || l.contains(tl)
                        || l.contains(sm));
                assert!(!bad, "{}:{}: {line}", path.display(), n + 1);
            }
        }
    }

    #[test]
    fn the_graph_queries_find_the_labour_edges() {
        let writers: Vec<_> = writers_of(QuantityId::WantedWorkers)
            .map(|d| d.id)
            .collect();
        assert_eq!(writers, [DecisionId::FirmHire]);
        let readers: Vec<_> = readers_of(QuantityId::WantedWorkers)
            .map(|d| d.id)
            .collect();
        assert_eq!(readers, [DecisionId::FirmFire]);
    }
}
