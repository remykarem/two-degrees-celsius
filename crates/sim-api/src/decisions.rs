//! Decision points on the wire (contract v2.7, `docs/features/decision-points.md`). The registry
//! (`sim_model::decisions::DECISIONS`) says what each decision reads, writes and uses; the words,
//! labels and units are authored in `content/decisions.json`. This module joins the two, checks they
//! agree, and serves `catalog.decisions` and `catalog.quantities`. It also generates the "decides",
//! "observes" and parameter lines of the Agents explorer for the kinds that own registered
//! decisions, so those lines cannot drift from the code.
use crate::assumptions::{param_value, AgentKindInfo, AssumptionParam};
use crate::error::{code, ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use sim_model::decisions::DECISIONS;
use sim_params::Params;
use sim_types::{DecisionId, OwnerKind, QuantityId};
use std::collections::BTreeMap;

pub const DECISIONS_JSON: &str = include_str!("../../../content/decisions.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerDef {
    kind: String,
    /// The id of the Agents explorer kind that shows this owner's decisions.
    agent_kind: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuantityDef {
    id: String,
    label: String,
    unit: String,
    owner: String,
    summary: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionDef {
    id: String,
    title: String,
    decides: String,
    rule: DecisionRule,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ParamDef {
    path: String,
    label: String,
    unit: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    owners: Vec<OwnerDef>,
    quantities: Vec<QuantityDef>,
    decisions: Vec<DecisionDef>,
    params: Vec<ParamDef>,
}

/// A quantity of the vocabulary: what decisions read and write (the nodes of the causal graph).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantityInfo {
    /// The `QuantityId` wire name.
    pub id: String,
    pub label: String,
    pub unit: String,
    /// The `OwnerKind` wire name of whoever holds the quantity.
    pub owner: String,
    pub summary: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantityRef {
    pub id: String,
    pub label: String,
    pub unit: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionRule {
    pub id: String,
    pub label: String,
    pub text: String,
}

/// One registered decision: owner, where it runs, what it reads and writes, and the live
/// parameters it uses.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionInfo {
    /// The `DecisionId` wire name.
    pub id: String,
    pub title: String,
    /// The `OwnerKind` wire name.
    pub owner: String,
    /// The Agents explorer kind (`AgentKindInfo.id`) this decision belongs to.
    pub agent_kind: String,
    /// The simulation system it runs in.
    pub system: String,
    /// One line: what it decides.
    pub decides: String,
    pub rule: DecisionRule,
    pub inputs: Vec<QuantityRef>,
    pub outputs: Vec<QuantityRef>,
    pub params: Vec<AssumptionParam>,
}

/// The decisions and the quantity vocabulary, joined from code and content and validated.
pub struct Resolved {
    pub decisions: Vec<DecisionInfo>,
    pub quantities: Vec<QuantityInfo>,
}

fn bad(m: String) -> ApiError {
    ApiError::new(code::BAD_METADATA, format!("content/decisions.json: {m}"))
}

pub fn resolve(params: &Params) -> ApiResult<Resolved> {
    let f: File = serde_json::from_str(DECISIONS_JSON).map_err(|e| bad(e.to_string()))?;
    let pj = serde_json::to_value(params).map_err(|e| bad(format!("params: {e}")))?;
    let owner_ok = |o: &str| OwnerKind::ALL.iter().any(|k| k.as_str() == o);

    // Quantities: exactly the vocabulary.
    let mut by_q: BTreeMap<&str, &QuantityDef> = BTreeMap::new();
    for q in &f.quantities {
        if !owner_ok(&q.owner) {
            return Err(bad(format!("quantity {}: unknown owner {}", q.id, q.owner)));
        }
        if by_q.insert(&q.id, q).is_some() {
            return Err(bad(format!("quantity {} appears twice", q.id)));
        }
    }
    for id in QuantityId::ALL {
        if !by_q.contains_key(id.as_str()) {
            return Err(bad(format!("quantity {} has no entry", id.as_str())));
        }
    }
    if let Some(q) = f
        .quantities
        .iter()
        .find(|q| !QuantityId::ALL.iter().any(|i| i.as_str() == q.id))
    {
        return Err(bad(format!("quantity {} is not a QuantityId", q.id)));
    }
    let qref = |id: &QuantityId| {
        let q = by_q[id.as_str()];
        QuantityRef {
            id: q.id.clone(),
            label: q.label.clone(),
            unit: q.unit.clone(),
        }
    };

    let mut by_p: BTreeMap<&str, &ParamDef> = BTreeMap::new();
    for p in &f.params {
        by_p.insert(&p.path, p);
    }

    let mut decisions = Vec::new();
    for spec in DECISIONS {
        let id = spec.id.as_str();
        let def = f
            .decisions
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| bad(format!("decision {id} has no entry")))?;
        if def.rule.id != spec.rule {
            return Err(bad(format!(
                "decision {id}: rule id {} is not the code's {}",
                def.rule.id, spec.rule
            )));
        }
        let owner = spec.owner().as_str();
        let agent_kind = f
            .owners
            .iter()
            .find(|o| o.kind == owner)
            .ok_or_else(|| bad(format!("no owner entry for {owner}")))?
            .agent_kind
            .clone();
        let mut ps = Vec::new();
        for path in spec.params {
            let d = by_p
                .get(path)
                .ok_or_else(|| bad(format!("decision {id}: no label for parameter {path}")))?;
            let value = param_value(&pj, path)
                .ok_or_else(|| bad(format!("decision {id}: no parameter {path}")))?;
            ps.push(AssumptionParam {
                path: (*path).to_string(),
                label: d.label.clone(),
                value,
                unit: d.unit.clone(),
            });
        }
        decisions.push(DecisionInfo {
            id: id.to_string(),
            title: def.title.clone(),
            owner: owner.to_string(),
            agent_kind,
            system: spec.system.name().to_string(),
            decides: def.decides.clone(),
            rule: def.rule.clone(),
            inputs: spec.inputs.iter().map(qref).collect(),
            outputs: spec.outputs.iter().map(qref).collect(),
            params: ps,
        });
    }
    if let Some(d) = f
        .decisions
        .iter()
        .find(|d| !DecisionId::ALL.iter().any(|i| i.as_str() == d.id))
    {
        return Err(bad(format!("decision {} is not a DecisionId", d.id)));
    }
    let quantities = f
        .quantities
        .iter()
        .map(|q| QuantityInfo {
            id: q.id.clone(),
            label: q.label.clone(),
            unit: q.unit.clone(),
            owner: q.owner.clone(),
            summary: q.summary.clone(),
        })
        .collect();
    Ok(Resolved {
        decisions,
        quantities,
    })
}

/// Put the generated lines in front of the hand-written ones of the Agents kind that owns the
/// decisions: `decides` from each decision, `observes` from the labels and summaries of the
/// inputs, `params` from the specs (live values). What stays in `content/assumptions.json` is only
/// what no registered decision covers yet.
pub fn merge_into_kind(kind: &mut AgentKindInfo, r: &Resolved, quantities: &[QuantityInfo]) {
    let mine: Vec<&DecisionInfo> = r
        .decisions
        .iter()
        .filter(|d| d.agent_kind == kind.id)
        .collect();
    if mine.is_empty() {
        return;
    }
    let mut decides: Vec<String> = mine.iter().map(|d| d.decides.clone()).collect();
    let mut observes: Vec<String> = Vec::new();
    let mut params: Vec<AssumptionParam> = Vec::new();
    for d in &mine {
        for q in &d.inputs {
            let summary = quantities
                .iter()
                .find(|x| x.id == q.id)
                .map_or("", |x| x.summary.as_str());
            let line = format!("{}: {summary}", q.label);
            if !observes.contains(&line) {
                observes.push(line);
            }
        }
        for p in &d.params {
            if !params.iter().any(|x| x.path == p.path) {
                params.push(p.clone());
            }
        }
    }
    decides.append(&mut kind.decides);
    observes.append(&mut kind.observes);
    params.append(&mut kind.params);
    kind.decides = decides;
    kind.observes = observes;
    kind.params = params;
}
