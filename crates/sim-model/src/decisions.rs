//! The decision registry: every explicit decision point, declared in one place
//! (`docs/features/decision-points.md`). Each `DecisionSpec` sits next to its `decide_*` function
//! in the system's module; this list gathers them so the wire, the Agents text and a future
//! causality view are generated from code. Add a decision: write the fn and its spec, add the spec
//! here, add its `DecisionId` and any new `QuantityId` in `sim-types`, and its words in
//! `content/decisions.json` (`docs/how-to.md`, "Add a decision").

use sim_types::{DecisionId, DecisionSpec, QuantityId};

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
];

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
