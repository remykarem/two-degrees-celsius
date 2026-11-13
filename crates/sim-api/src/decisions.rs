//! Decision points on the wire (contract v2.7, `docs/features/decision-points.md`). The registry
//! (`sim_model::decisions::DECISIONS`) says what each decision reads, writes and uses; the words,
//! labels and units are authored in `content/decisions.json`. This module joins the two, checks they
//! agree, and serves `catalog.decisions` and `catalog.quantities`. It also generates the "decides",
//! "observes", parameter and rule lines of the Agents explorer for every kind that owns registered
//! decisions, so those lines cannot drift from the code, and the causal graph (contract v2.8) from
//! `sim_model::causal`.
use crate::assumptions::{param_value, AgentKindInfo, AssumptionParam};
use crate::error::{code, ApiError, ApiResult};
use crate::live;
use serde::{Deserialize, Serialize};
use sim_model::causal::{self, EdgeKind, Node, Role};
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
struct NoteDef {
    agent_kind: String,
    notes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoleDef {
    id: String,
    label: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CausalDef {
    /// How many decisions back the "upstream causes" of a quantity reach.
    upstream_depth: usize,
    roles: Vec<RoleDef>,
    live: LiveDef,
}

/// Words for the live-value binding: aggregates and reasons a quantity has no value.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LiveDef {
    aggregates: Vec<RoleDef>,
    unbound: Vec<RoleDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    owners: Vec<OwnerDef>,
    /// Documented exceptions: what an Agents kind does that no registered decision covers.
    kind_notes: Vec<NoteDef>,
    causal: CausalDef,
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
    /// Contract v2.9: where the quantity's live number comes from, or why it has none.
    pub live: LiveBinding,
}

/// Contract v2.9: the live-value binding of a quantity (`docs/features/causality-graph.md` C8). A
/// bound quantity has a `path` into the snapshot and an `aggregate` id; its current value is on
/// `snapshot.quantities`. An unbound one names a reason id (`catalog().causal.live.unbound`).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBinding {
    pub bound: bool,
    /// The snapshot path the number is read from (`firms[].price`, `markets[food].price`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub path: Option<String>,
    /// One of `catalog().causal.live.aggregates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub aggregate: Option<String>,
    /// The `LeverKey` this quantity is the setting of, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lever: Option<String>,
    /// The snapshot value carries a `previous` and a `trend` (the history keeps its series).
    pub has_trend: bool,
    /// One of `catalog().causal.live.unbound` when `bound` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unbound: Option<String>,
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

/// A word for a node role (`source`, `derived`, `sink`, `decision`), from content.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CausalRole {
    pub id: String,
    pub label: String,
    pub text: String,
}

/// Contract v2.9: the words of the live-value binding (an id, a label and a sentence each).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CausalLive {
    /// How a town-level number is made (`town`, `sum`, `mean`, `lever`, `flag`).
    pub aggregates: Vec<CausalRole>,
    /// Why a quantity has no live value (`private`, `computed`, `not_published`, `fixed`, `random`).
    pub unbound: Vec<CausalRole>,
}

/// A node of the causal graph (contract v2.8): a quantity or a decision.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CausalNode {
    /// `quantity:<id>` or `decision:<id>` (a quantity and a decision can share a bare id).
    pub id: String,
    /// `quantity` or `decision`.
    pub kind: String,
    /// `source` (nothing writes it), `derived`, `sink` (nothing else reads it) or `decision`.
    pub role: String,
    /// The bare `QuantityId` or `DecisionId` wire name.
    pub ref_id: String,
    pub label: String,
    /// The `OwnerKind` wire name.
    pub owner: String,
    /// A decision both reads and writes this quantity (legitimacy is an example).
    pub self_loop: bool,
    /// The weakly connected component, numbered in node order.
    pub component: usize,
    /// Quantity node ids that can change this quantity, nearest first, up to
    /// `causal.upstreamDepth` decisions back. Empty for decisions and sources.
    pub upstream: Vec<String>,
}

/// An edge: `reads` (quantity -> decision), `writes` (decision -> quantity) or `drives`
/// (quantity -> quantity through `via`).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CausalEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
    /// The decision node id a `drives` edge goes through.
    pub via: Option<String>,
    pub self_loop: bool,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CausalGraph {
    pub upstream_depth: usize,
    pub roles: Vec<CausalRole>,
    /// Contract v2.9: words for the live-value binding.
    pub live: CausalLive,
    pub nodes: Vec<CausalNode>,
    pub edges: Vec<CausalEdge>,
}

/// The decisions and the quantity vocabulary, joined from code and content and validated.
pub struct Resolved {
    pub decisions: Vec<DecisionInfo>,
    pub quantities: Vec<QuantityInfo>,
    pub causal: CausalGraph,
    /// Documented notes by Agents kind id.
    pub notes: Vec<(String, Vec<String>)>,
}

fn node_id(n: Node) -> String {
    match n {
        Node::Quantity(q) => format!("quantity:{}", q.as_str()),
        Node::Decision(d) => format!("decision:{}", d.as_str()),
    }
}

fn live_words(what: &str, defs: &[RoleDef], ids: Vec<&'static str>) -> ApiResult<Vec<CausalRole>> {
    for id in &ids {
        if !defs.iter().any(|d| d.id == *id) {
            return Err(bad(format!("causal.live.{what} has no {id}")));
        }
    }
    if let Some(d) = defs.iter().find(|d| !ids.contains(&d.id.as_str())) {
        return Err(bad(format!("causal.live.{what}: unknown id {}", d.id)));
    }
    Ok(defs
        .iter()
        .map(|d| CausalRole {
            id: d.id.clone(),
            label: d.label.clone(),
            text: d.text.clone(),
        })
        .collect())
}

fn build_live(l: &LiveDef) -> ApiResult<CausalLive> {
    Ok(CausalLive {
        aggregates: live_words(
            "aggregates",
            &l.aggregates,
            live::Aggregate::ALL.iter().map(|a| a.as_str()).collect(),
        )?,
        unbound: live_words(
            "unbound",
            &l.unbound,
            live::Unbound::ALL.iter().map(|a| a.as_str()).collect(),
        )?,
    })
}

/// The binding of one quantity as the catalog serves it.
fn live_binding(q: QuantityId) -> LiveBinding {
    match live::binding(q) {
        live::Binding::Bound(b) => LiveBinding {
            bound: true,
            path: Some(b.path.to_string()),
            aggregate: Some(b.aggregate.as_str().to_string()),
            lever: b.lever.map(|l| l.as_str().to_string()),
            has_trend: b.previous.is_some(),
            unbound: None,
        },
        live::Binding::Unbound(u) => LiveBinding {
            bound: false,
            path: None,
            aggregate: None,
            lever: None,
            has_trend: false,
            unbound: Some(u.as_str().to_string()),
        },
    }
}

fn build_causal(
    c: &CausalDef,
    quantities: &[QuantityInfo],
    decisions: &[DecisionInfo],
) -> ApiResult<CausalGraph> {
    let role_text = |id: &str| -> ApiResult<()> {
        c.roles
            .iter()
            .any(|r| r.id == id)
            .then_some(())
            .ok_or_else(|| bad(format!("causal.roles has no {id}")))
    };
    for id in ["source", "derived", "sink", "decision"] {
        role_text(id)?;
    }
    if c.roles.len() != 4 {
        return Err(bad(
            "causal.roles must be source, derived, sink, decision".into()
        ));
    }
    let comp: BTreeMap<Node, usize> = causal::components().into_iter().collect();
    let mut nodes = Vec::new();
    for n in causal::nodes() {
        nodes.push(match n {
            Node::Quantity(q) => {
                let info = quantities
                    .iter()
                    .find(|x| x.id == q.as_str())
                    .ok_or_else(|| bad(format!("quantity {} has no entry", q.as_str())))?;
                let upstream = causal::causes_of(q, c.upstream_depth)
                    .quantities
                    .into_iter()
                    .map(|(u, _)| node_id(Node::Quantity(u)))
                    .collect();
                CausalNode {
                    id: node_id(n),
                    kind: "quantity".into(),
                    role: match causal::role(q) {
                        Role::Source => "source",
                        Role::Derived => "derived",
                        Role::Sink => "sink",
                    }
                    .into(),
                    ref_id: q.as_str().into(),
                    label: info.label.clone(),
                    owner: info.owner.clone(),
                    self_loop: causal::has_self_loop(q),
                    component: comp[&n],
                    upstream,
                }
            }
            Node::Decision(d) => {
                let info = decisions
                    .iter()
                    .find(|x| x.id == d.as_str())
                    .ok_or_else(|| bad(format!("decision {} has no entry", d.as_str())))?;
                CausalNode {
                    id: node_id(n),
                    kind: "decision".into(),
                    role: "decision".into(),
                    ref_id: d.as_str().into(),
                    label: info.title.clone(),
                    owner: info.owner.clone(),
                    self_loop: false,
                    component: comp[&n],
                    upstream: Vec::new(),
                }
            }
        });
    }
    let edges = causal::edges()
        .into_iter()
        .map(|e| CausalEdge {
            from: node_id(e.from),
            to: node_id(e.to),
            kind: match e.kind {
                EdgeKind::Reads => "reads",
                EdgeKind::Writes => "writes",
                EdgeKind::Drives => "drives",
            }
            .into(),
            via: e.via.map(|d| node_id(Node::Decision(d))),
            self_loop: e.self_loop,
        })
        .collect();
    Ok(CausalGraph {
        upstream_depth: c.upstream_depth,
        roles: c
            .roles
            .iter()
            .map(|r| CausalRole {
                id: r.id.clone(),
                label: r.label.clone(),
                text: r.text.clone(),
            })
            .collect(),
        live: build_live(&c.live)?,
        nodes,
        edges,
    })
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
    let quantities: Vec<QuantityInfo> = f
        .quantities
        .iter()
        .map(|q| QuantityInfo {
            id: q.id.clone(),
            label: q.label.clone(),
            unit: q.unit.clone(),
            owner: q.owner.clone(),
            summary: q.summary.clone(),
            live: QuantityId::ALL
                .iter()
                .find(|i| i.as_str() == q.id)
                .map(|i| live_binding(*i))
                .unwrap_or(LiveBinding {
                    bound: false,
                    path: None,
                    aggregate: None,
                    lever: None,
                    has_trend: false,
                    unbound: None,
                }),
        })
        .collect();
    let causal = build_causal(&f.causal, &quantities, &decisions)?;
    let notes = f
        .kind_notes
        .into_iter()
        .map(|n| (n.agent_kind, n.notes))
        .collect();
    Ok(Resolved {
        decisions,
        quantities,
        causal,
        notes,
    })
}

/// Fill the Agents kind from the registry: `decides` from each decision, `observes` from the
/// labels and summaries of the inputs, `params` from the specs (live values) and `rule` from the
/// decisions' rule texts. Nothing about decisions may be hand-written: a kind with registered
/// decisions must have empty `decides`, `observes`, `params` and `rule` in
/// `content/assumptions.json`; a kind without one must have empty `decides` and a documented note
/// in `content/decisions.json` `kind_notes` ("nothing decided here", and why). The notes of any
/// kind are served as `notes`. Its own mechanism lines (`observes`, `params`, `rule` of
/// lobbyists and weather, which have no decision) stay as authored.
pub fn merge_into_kind(
    kind: &mut AgentKindInfo,
    r: &Resolved,
    quantities: &[QuantityInfo],
) -> ApiResult<()> {
    let mine: Vec<&DecisionInfo> = r
        .decisions
        .iter()
        .filter(|d| d.agent_kind == kind.id)
        .collect();
    let noted: Vec<String> = r
        .notes
        .iter()
        .filter(|(k, _)| *k == kind.id)
        .flat_map(|(_, n)| n.clone())
        .collect();
    if !kind.decides.is_empty() {
        return Err(bad(format!(
            "agents kind {}: hand-written `decides` (generate it, or add a kind_notes entry)",
            kind.id
        )));
    }
    if mine.is_empty() {
        if noted.is_empty() {
            return Err(bad(format!(
                "agents kind {} has no registered decision and no kind_notes entry",
                kind.id
            )));
        }
        kind.notes = noted;
        return Ok(());
    }
    if !kind.observes.is_empty() || !kind.params.is_empty() || !kind.rule.is_empty() {
        return Err(bad(format!(
            "agents kind {}: hand-written observes, params or rule next to registered decisions",
            kind.id
        )));
    }
    for d in &mine {
        kind.decides.push(d.decides.clone());
        for q in &d.inputs {
            let summary = quantities
                .iter()
                .find(|x| x.id == q.id)
                .map_or("", |x| x.summary.as_str());
            let line = format!("{}: {summary}", q.label);
            if !kind.observes.contains(&line) {
                kind.observes.push(line);
            }
        }
        for p in &d.params {
            if !kind.params.iter().any(|x| x.path == p.path) {
                kind.params.push(p.clone());
            }
        }
    }
    kind.rule = mine
        .iter()
        .map(|d| format!("{}: {}", d.title, d.rule.text))
        .collect::<Vec<_>>()
        .join("\n");
    kind.notes = noted;
    Ok(())
}
