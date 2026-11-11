//! `snapshot.policies` (contract v1.6): every lever with its initial, active, effective and
//! pending values, who weakened it, and the timeline from the wrapper log.
use crate::keys::LeverKey;
use crate::log::{weakened_share, PolicyEvent, PolicyLog};
use crate::meta::Meta;
use crate::snapshot::{PolicyEntry, PolicyScalar};
use serde::{Deserialize, Serialize};
use sim_engine::Sim;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingRef {
    pub id: u32,
    pub value: PolicyScalar,
    pub effective_turn: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRow {
    pub lever: LeverKey,
    pub label: String,
    pub group: String,
    pub initial: PolicyScalar,
    pub active: PolicyScalar,
    /// The value in force after lobbying (lobbied levers only).
    pub effective: Option<f64>,
    pub pending: Option<PendingRef>,
    pub last_change_turn: Option<u32>,
    /// Share of the policy lobbying has cut (lobbied levers only; 0 when none).
    pub weakened_by: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Policies {
    pub rows: Vec<PolicyRow>,
    pub timeline: Vec<PolicyEvent>,
    /// Additive: share of the political capital of a cancelled command that is refunded.
    pub cancel_refund: f64,
}

pub fn build(
    sim: &Sim,
    meta: &Meta,
    policy: &BTreeMap<LeverKey, PolicyEntry>,
    log: &PolicyLog,
) -> Policies {
    let rows = meta
        .levers
        .iter()
        .map(|info| {
            let key = info.key;
            let entry = policy.get(&key);
            let pending = sim
                .state
                .pending
                .iter()
                .filter_map(|p| match crate::command::Command::from_sim(&p.command) {
                    crate::command::Command::SetPolicy { lever, value } if lever == key => {
                        Some(PendingRef {
                            id: p.id,
                            value: crate::log::scalar_of(&value),
                            effective_turn: p.effective_turn,
                        })
                    }
                    _ => None,
                })
                .max_by_key(|p| p.id);
            let weakened_by = match (entry, info.lobbied) {
                (
                    Some(PolicyEntry {
                        nominal: PolicyScalar::Number(n),
                        effective: Some(e),
                    }),
                    true,
                ) => Some(weakened_share(key, *n, *e)),
                _ => None,
            };
            PolicyRow {
                lever: key,
                label: info.label.clone(),
                group: info.group.clone(),
                initial: log
                    .initial
                    .get(&key)
                    .cloned()
                    .unwrap_or(PolicyScalar::Number(0.0)),
                active: entry.map_or(PolicyScalar::Number(0.0), |e| e.nominal.clone()),
                effective: entry.and_then(|e| e.effective),
                pending,
                last_change_turn: log
                    .events
                    .iter()
                    .rev()
                    .find(|e| e.lever == key && e.kind == crate::log::PolicyEventKind::Activated)
                    .map(|e| e.turn),
                weakened_by,
            }
        })
        .collect();
    Policies {
        rows,
        timeline: log.events.clone(),
        cancel_refund: sim.params.turn.cancel_refund,
    }
}
