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
use sim_model::decisions::{DECISIONS, MECHANISMS};
use sim_params::Params;
use sim_types::{DecisionId, MechanismId, OwnerKind, QuantityId};
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
struct MechanismDef {
    id: String,
    title: String,
    /// One line: what the plain code does (the decision's `decides`).
    explains: String,
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
    /// Contract v2.11: the words of the edge sentences (`docs/features/causality-graph.md` stage 2).
    effects: EffectWords,
}

/// Templates of the plain-English sentence of an edge, filled from the declared sign and lag so the
/// sentence cannot disagree with the sign: `{effect} ({lag}), {because}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EffectWords {
    up: String,
    down: String,
    varies: String,
    none: String,
    sentence: String,
    sentence_plain: String,
    lag_now: String,
    lag_next: String,
    lag_months: String,
    because: String,
    by_rule: String,
    basis: BasisWords,
    /// A hand-written reason for one effect, keyed by spec and quantity ids; the rest say which rule the spec follows.
    because_overrides: Vec<BecauseDef>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BasisWords {
    perturbed: String,
    system: String,
    declared_unverified: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BecauseDef {
    spec: String,
    from: String,
    to: String,
    because: String,
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
    /// Contract v2.10: the words of the registered mechanisms (plain physics and accounting code).
    mechanisms: Vec<MechanismDef>,
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
/// Contract v2.10: one registered mechanism, the plain code (physics or accounting) that links the
/// quantities decisions read and write. Same shape as a decision, minus "decides": it is not a
/// choice, and declaring it changes no behaviour.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MechanismInfo {
    /// The `MechanismId` wire name.
    pub id: String,
    pub title: String,
    /// The `OwnerKind` wire name.
    pub owner: String,
    /// The Agents explorer kind (`AgentKindInfo.id`) that lists it.
    pub agent_kind: String,
    /// The simulation system it runs in.
    pub system: String,
    /// One line: what the code does.
    pub explains: String,
    pub rule: DecisionRule,
    pub inputs: Vec<QuantityRef>,
    pub outputs: Vec<QuantityRef>,
    pub params: Vec<AssumptionParam>,
}

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
    /// `quantity:<id>`, `decision:<id>` or `mechanism:<id>` (a quantity and a decision can share a bare id).
    pub id: String,
    /// `quantity`, `decision` or `mechanism` (contract v2.10: plain physics and accounting code).
    pub kind: String,
    /// `source` (nothing in the registry writes it), `derived`, `sink` (nothing else reads it),
    /// `decision` or `mechanism`.
    pub role: String,
    /// The bare `QuantityId`, `DecisionId` or `MechanismId` wire name.
    pub ref_id: String,
    pub label: String,
    /// The `OwnerKind` wire name.
    pub owner: String,
    /// A decision or mechanism both reads and writes this quantity (legitimacy is an example).
    pub self_loop: bool,
    /// The weakly connected component, numbered in node order.
    pub component: usize,
    /// Quantity node ids that can change this quantity, nearest first, up to
    /// `causal.upstreamDepth` steps back (decisions or mechanisms crossed). Empty for decisions,
    /// mechanisms and sources.
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
    /// The decision or mechanism node id a `drives` edge goes through.
    pub via: Option<String>,
    pub self_loop: bool,
    /// Contract v2.11: the declared effect of a `drives` edge (sign, lag, how it was checked, and
    /// its plain-English sentence). Absent on `reads` and `writes` edges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<EdgeEffect>,
}

/// Contract v2.11: what one input does to one output of a decision or mechanism.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeEffect {
    /// `up` (more input, more output), `down`, `varies` (depends on the state) or `none` (read, but no direct effect).
    pub sign: String,
    /// Months from a change in the input to the change in the output: the spec's own lag plus the wait
    /// the tick order imposes (0 = the same month).
    pub lag_months: u32,
    /// `perturbed` (measured on pure code or a decision), `system` (measured on a whole system on a real state) or `declared_unverified`.
    pub basis: String,
    /// The reason for `varies`, `none` and `declared_unverified`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The lag in words (`the same month`, `the next month`, `3 months later`), from `content/decisions.json`.
    pub lag_label: String,
    /// The sentence, from the templates in `content/decisions.json` and the declared sign.
    pub sentence: String,
    /// How it was checked, in words.
    pub checked: String,
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
    pub mechanisms: Vec<MechanismInfo>,
    pub quantities: Vec<QuantityInfo>,
    pub causal: CausalGraph,
    /// Documented notes by Agents kind id.
    pub notes: Vec<(String, Vec<String>)>,
}

fn node_id(n: Node) -> String {
    match n {
        Node::Quantity(q) => format!("quantity:{}", q.as_str()),
        Node::Decision(d) => format!("decision:{}", d.as_str()),
        Node::Mechanism(m) => format!("mechanism:{}", m.as_str()),
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
    mechanisms: &[MechanismInfo],
) -> ApiResult<CausalGraph> {
    let role_text = |id: &str| -> ApiResult<()> {
        c.roles
            .iter()
            .any(|r| r.id == id)
            .then_some(())
            .ok_or_else(|| bad(format!("causal.roles has no {id}")))
    };
    for id in ["source", "derived", "sink", "decision", "mechanism"] {
        role_text(id)?;
    }
    if c.roles.len() != 5 {
        return Err(bad(
            "causal.roles must be source, derived, sink, decision, mechanism".into(),
        ));
    }
    // Hand-written reasons name real, signed effects, once each.
    let mut seen_b: Vec<(&str, &str, &str)> = Vec::new();
    for b in &c.effects.because_overrides {
        let real = causal::edges().into_iter().any(|e| {
            matches!((e.effect, e.via, e.from, e.to),
            (Some(fx), Some(v), Node::Quantity(i), Node::Quantity(o))
            if !matches!(fx.sign, sim_types::Sign::None(_))
                && i.as_str() == b.from
                && o.as_str() == b.to
                && match v {
                    Node::Decision(d) => d.as_str() == b.spec,
                    Node::Mechanism(m) => m.as_str() == b.spec,
                    Node::Quantity(_) => false,
                })
        });
        if !real {
            return Err(bad(format!(
                "causal.effects.because_overrides: {} {} -> {} is not a signed effect of the registry",
                b.spec, b.from, b.to
            )));
        }
        if seen_b.contains(&(&b.spec[..], &b.from[..], &b.to[..])) {
            return Err(bad(format!(
                "causal.effects.because_overrides: {} {} -> {} appears twice",
                b.spec, b.from, b.to
            )));
        }
        seen_b.push((&b.spec, &b.from, &b.to));
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
            Node::Mechanism(m) => {
                let info = mechanisms
                    .iter()
                    .find(|x| x.id == m.as_str())
                    .ok_or_else(|| bad(format!("mechanism {} has no entry", m.as_str())))?;
                CausalNode {
                    id: node_id(n),
                    kind: "mechanism".into(),
                    role: "mechanism".into(),
                    ref_id: m.as_str().into(),
                    label: info.title.clone(),
                    owner: info.owner.clone(),
                    self_loop: false,
                    component: comp[&n],
                    upstream: Vec::new(),
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
        .map(|e| {
            let effect = match (e.effect, e.via, e.from, e.to) {
                (Some(fx), Some(via), Node::Quantity(i), Node::Quantity(o)) => Some(edge_effect(
                    &c.effects,
                    quantities,
                    decisions,
                    mechanisms,
                    via,
                    i,
                    o,
                    fx,
                    causal::edge_lag_months(&e).unwrap_or(0),
                )?),
                _ => None,
            };
            Ok(CausalEdge {
                from: node_id(e.from),
                to: node_id(e.to),
                kind: match e.kind {
                    EdgeKind::Reads => "reads",
                    EdgeKind::Writes => "writes",
                    EdgeKind::Drives => "drives",
                }
                .into(),
                via: e.via.map(node_id),
                self_loop: e.self_loop,
                effect,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
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

/// Lower-case the first letter of a label for the middle of a sentence ("Unit cost" -> "unit cost"),
/// keeping acronyms ("GDP").
fn mid_sentence(label: &str) -> String {
    let mut cs = label.chars();
    match (cs.next(), cs.next()) {
        (Some(a), Some(b)) if a.is_uppercase() && !b.is_uppercase() => {
            a.to_lowercase().collect::<String>() + &label[a.len_utf8()..]
        }
        _ => label.to_string(),
    }
}

/// The sign, lag, basis and sentence of one `drives` edge, from the declared `Effect` and the words.
#[allow(clippy::too_many_arguments)]
fn edge_effect(
    w: &EffectWords,
    quantities: &[QuantityInfo],
    decisions: &[DecisionInfo],
    mechanisms: &[MechanismInfo],
    via: Node,
    from: QuantityId,
    to: QuantityId,
    fx: sim_types::Effect,
    lag: u32,
) -> ApiResult<EdgeEffect> {
    let label = |q: QuantityId| {
        quantities
            .iter()
            .find(|x| x.id == q.as_str())
            .map(|x| mid_sentence(&x.label))
            .ok_or_else(|| bad(format!("quantity {} has no entry", q.as_str())))
    };
    let (spec_id, rule_label) = match via {
        Node::Decision(d) => (
            d.as_str(),
            decisions
                .iter()
                .find(|x| x.id == d.as_str())
                .map(|x| x.rule.label.clone()),
        ),
        Node::Mechanism(m) => (
            m.as_str(),
            mechanisms
                .iter()
                .find(|x| x.id == m.as_str())
                .map(|x| x.rule.label.clone()),
        ),
        Node::Quantity(_) => {
            return Err(bad("an edge goes through a decision or mechanism".into()))
        }
    };
    let rule_label = rule_label.ok_or_else(|| bad(format!("{spec_id} has no entry")))?;
    let (sign, reason) = match fx.sign {
        sim_types::Sign::Up => ("up", None),
        sim_types::Sign::Down => ("down", None),
        sim_types::Sign::Varies(r) => ("varies", Some(r.to_string())),
        sim_types::Sign::None(r) => ("none", Some(r.to_string())),
    };
    let template = match sign {
        "up" => &w.up,
        "down" => &w.down,
        "varies" => &w.varies,
        _ => &w.none,
    };
    let effect = template
        .replace("{input}", &label(from)?)
        .replace("{output}", &label(to)?)
        .replace("{reason}", reason.as_deref().unwrap_or(""));
    let lag_words = match lag {
        0 => w.lag_now.clone(),
        1 => w.lag_next.clone(),
        n => w.lag_months.replace("{n}", &n.to_string()),
    };
    let because = match w
        .because_overrides
        .iter()
        .find(|b| b.spec == spec_id && b.from == from.as_str() && b.to == to.as_str())
    {
        Some(b) => w.because.replace("{why}", &b.because),
        None => w.by_rule.replace("{rule}", &rule_label),
    };
    // "none" has no lag to speak of and "varies" gives its reason in the sentence: neither has a "because".
    let sentence = if sign == "none" {
        capitalise(&effect)
    } else {
        let t = if sign == "varies" {
            &w.sentence_plain
        } else {
            &w.sentence
        };
        capitalise(
            &t.replace("{effect}", &effect)
                .replace("{lag}", &lag_words)
                .replace("{because}", &because),
        )
    };
    let (basis, checked) = match fx.basis {
        sim_types::Basis::Perturbed => ("perturbed", w.basis.perturbed.clone()),
        sim_types::Basis::PerturbedSystem => ("system", w.basis.system.clone()),
        sim_types::Basis::DeclaredUnverified(r) => (
            "declared_unverified",
            w.basis.declared_unverified.replace("{reason}", r),
        ),
    };
    let reason = match fx.basis {
        sim_types::Basis::DeclaredUnverified(r) if reason.is_none() => Some(r.to_string()),
        _ => reason,
    };
    Ok(EdgeEffect {
        sign: sign.into(),
        lag_months: lag,
        basis: basis.into(),
        reason,
        lag_label: lag_words,
        sentence,
        checked,
    })
}

fn capitalise(s: &str) -> String {
    let mut cs = s.chars();
    match cs.next() {
        Some(a) => a.to_uppercase().collect::<String>() + cs.as_str(),
        None => String::new(),
    }
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
    let mut mechanisms = Vec::new();
    for spec in MECHANISMS {
        let id = spec.id.as_str();
        let def = f
            .mechanisms
            .iter()
            .find(|d| d.id == id)
            .ok_or_else(|| bad(format!("mechanism {id} has no entry")))?;
        if def.rule.id != spec.rule {
            return Err(bad(format!(
                "mechanism {id}: rule id {} is not the code's {}",
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
                .ok_or_else(|| bad(format!("mechanism {id}: no label for parameter {path}")))?;
            let value = param_value(&pj, path)
                .ok_or_else(|| bad(format!("mechanism {id}: no parameter {path}")))?;
            ps.push(AssumptionParam {
                path: (*path).to_string(),
                label: d.label.clone(),
                value,
                unit: d.unit.clone(),
            });
        }
        mechanisms.push(MechanismInfo {
            id: id.to_string(),
            title: def.title.clone(),
            owner: owner.to_string(),
            agent_kind,
            system: spec.system.name().to_string(),
            explains: def.explains.clone(),
            rule: def.rule.clone(),
            inputs: spec.inputs.iter().map(qref).collect(),
            outputs: spec.outputs.iter().map(qref).collect(),
            params: ps,
        });
    }
    if let Some(d) = f
        .mechanisms
        .iter()
        .find(|d| !MechanismId::ALL.iter().any(|i| i.as_str() == d.id))
    {
        return Err(bad(format!("mechanism {} is not a MechanismId", d.id)));
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
    let causal = build_causal(&f.causal, &quantities, &decisions, &mechanisms)?;
    let notes = f
        .kind_notes
        .into_iter()
        .map(|n| (n.agent_kind, n.notes))
        .collect();
    Ok(Resolved {
        decisions,
        mechanisms,
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
