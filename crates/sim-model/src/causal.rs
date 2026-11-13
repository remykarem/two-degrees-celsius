//! The causal graph of the decision registry (`docs/features/decision-points.md`, "Future:
//! causality graph"): pure functions over `DECISIONS`, no state, no content. Nodes are quantities
//! and decisions; an edge says "this decision reads that quantity" or "writes that one", and a
//! derived edge links a quantity a decision reads to each quantity it writes ("b -> a via X").
//! The wire (`sim-api::decisions`) adds labels from `content/decisions.json`. Order is stable:
//! quantities in `QuantityId::ALL` order, then decisions in `DECISIONS` order; no hash maps.

use crate::decisions::{readers_of, writers_of, DECISIONS};
use sim_types::{DecisionId, QuantityId};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    Quantity(QuantityId),
    Decision(DecisionId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// quantity -> decision: the decision reads it.
    Reads,
    /// decision -> quantity: the decision writes it.
    Writes,
    /// quantity -> quantity, `via` the decision that reads the first and writes the second.
    Drives,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: Node,
    pub to: Node,
    pub kind: EdgeKind,
    /// The decision a `Drives` edge goes through.
    pub via: Option<DecisionId>,
    /// The decision both reads and writes this quantity (for example legitimacy).
    pub self_loop: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// No decision writes it: a state variable, a lever or a setting that starts a chain.
    Source,
    /// Written by a decision and read by one that does not write it.
    Derived,
    /// Written, but read by nothing except the decisions that write it: an outcome.
    Sink,
}

/// Every quantity then every decision, in stable order.
pub fn nodes() -> Vec<Node> {
    QuantityId::ALL
        .iter()
        .map(|q| Node::Quantity(*q))
        .chain(DECISIONS.iter().map(|d| Node::Decision(d.id)))
        .collect()
}

pub fn role(q: QuantityId) -> Role {
    if writers_of(q).next().is_none() {
        Role::Source
    } else if readers_of(q).all(|r| r.outputs.contains(&q)) {
        Role::Sink
    } else {
        Role::Derived
    }
}

/// Some decision reads and writes this quantity.
pub fn has_self_loop(q: QuantityId) -> bool {
    DECISIONS
        .iter()
        .any(|d| d.inputs.contains(&q) && d.outputs.contains(&q))
}

/// All edges: reads and writes per decision in registry order, then the derived links.
pub fn edges() -> Vec<Edge> {
    let mut out = Vec::new();
    for d in DECISIONS {
        for q in d.inputs {
            out.push(Edge {
                from: Node::Quantity(*q),
                to: Node::Decision(d.id),
                kind: EdgeKind::Reads,
                via: None,
                self_loop: d.outputs.contains(q),
            });
        }
        for q in d.outputs {
            out.push(Edge {
                from: Node::Decision(d.id),
                to: Node::Quantity(*q),
                kind: EdgeKind::Writes,
                via: None,
                self_loop: d.inputs.contains(q),
            });
        }
    }
    for d in DECISIONS {
        for i in d.inputs {
            for o in d.outputs {
                out.push(Edge {
                    from: Node::Quantity(*i),
                    to: Node::Quantity(*o),
                    kind: EdgeKind::Drives,
                    via: Some(d.id),
                    self_loop: i == o,
                });
            }
        }
    }
    out
}

/// The weakly connected component of every node (ignoring direction), numbered in node order, so
/// the first node is in component 0.
pub fn components() -> Vec<(Node, usize)> {
    let ns = nodes();
    let idx = |n: &Node| ns.iter().position(|x| x == n).unwrap_or(0);
    let mut parent: Vec<usize> = (0..ns.len()).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for e in edges().iter().filter(|e| e.kind != EdgeKind::Drives) {
        let (a, b) = (
            find(&mut parent, idx(&e.from)),
            find(&mut parent, idx(&e.to)),
        );
        if a != b {
            parent[a.max(b)] = a.min(b);
        }
    }
    let mut roots: Vec<usize> = Vec::new();
    let mut out = Vec::new();
    for (i, n) in ns.iter().enumerate() {
        let r = find(&mut parent, i);
        let c = roots.iter().position(|x| *x == r).unwrap_or_else(|| {
            roots.push(r);
            roots.len() - 1
        });
        out.push((*n, c));
    }
    out
}

/// What lies upstream of a quantity: the quantities and decisions that can change it, each with
/// the number of decisions crossed to reach it (1 = written directly by the decision, or read by
/// that decision). Nearest first, then registry order. `max_depth` limits the walk.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Causes {
    pub quantities: Vec<(QuantityId, usize)>,
    pub decisions: Vec<(DecisionId, usize)>,
}

pub fn causes_of(q: QuantityId, max_depth: usize) -> Causes {
    let mut seen_q: BTreeSet<QuantityId> = BTreeSet::from([q]);
    let mut seen_d: BTreeSet<DecisionId> = BTreeSet::new();
    let mut frontier = vec![q];
    let mut out = Causes::default();
    for depth in 1..=max_depth {
        let mut next: Vec<QuantityId> = Vec::new();
        for f in &frontier {
            for d in writers_of(*f) {
                if seen_d.insert(d.id) {
                    out.decisions.push((d.id, depth));
                }
                for i in d.inputs {
                    if seen_q.insert(*i) {
                        out.quantities.push((*i, depth));
                        next.push(*i);
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_edge_comes_from_a_spec_and_every_spec_io_has_an_edge() {
        let es = edges();
        for e in &es {
            match (e.kind, e.from, e.to, e.via) {
                (EdgeKind::Reads, Node::Quantity(q), Node::Decision(d), None) => {
                    assert!(DECISIONS.iter().any(|s| s.id == d && s.inputs.contains(&q)));
                }
                (EdgeKind::Writes, Node::Decision(d), Node::Quantity(q), None) => {
                    assert!(DECISIONS
                        .iter()
                        .any(|s| s.id == d && s.outputs.contains(&q)));
                }
                (EdgeKind::Drives, Node::Quantity(i), Node::Quantity(o), Some(d)) => {
                    assert!(DECISIONS
                        .iter()
                        .any(|s| s.id == d && s.inputs.contains(&i) && s.outputs.contains(&o)));
                    assert_eq!(e.self_loop, i == o);
                }
                other => panic!("malformed edge {other:?}"),
            }
        }
        for s in DECISIONS {
            for q in s.inputs {
                assert!(es.iter().any(|e| e.kind == EdgeKind::Reads
                    && e.from == Node::Quantity(*q)
                    && e.to == Node::Decision(s.id)));
            }
            for q in s.outputs {
                assert!(es.iter().any(|e| e.kind == EdgeKind::Writes
                    && e.from == Node::Decision(s.id)
                    && e.to == Node::Quantity(*q)));
            }
            for i in s.inputs {
                for o in s.outputs {
                    assert_eq!(
                        es.iter()
                            .filter(|e| e.kind == EdgeKind::Drives
                                && e.via == Some(s.id)
                                && e.from == Node::Quantity(*i)
                                && e.to == Node::Quantity(*o))
                            .count(),
                        1
                    );
                }
            }
        }
    }

    #[test]
    fn no_dangling_nodes_and_order_is_deterministic() {
        let ns = nodes();
        assert_eq!(ns.len(), QuantityId::ALL.len() + DECISIONS.len());
        for e in edges() {
            assert!(ns.contains(&e.from) && ns.contains(&e.to));
        }
        assert_eq!(edges(), edges());
        assert_eq!(components(), components());
        // Every quantity is used by some decision.
        for q in QuantityId::ALL {
            assert!(
                readers_of(*q).next().is_some() || writers_of(*q).next().is_some(),
                "{} is in no spec",
                q.as_str()
            );
        }
    }

    #[test]
    fn roles_and_self_loops() {
        // Nothing writes unemployment or the treasury's cash: they start chains.
        assert_eq!(role(QuantityId::Unemployment), Role::Source);
        assert_eq!(role(QuantityId::Savings), Role::Source);
        assert_eq!(role(QuantityId::TreasuryCash), Role::Source);
        // Legitimacy is read and written by one decision only: a sink with a self loop.
        assert!(has_self_loop(QuantityId::Legitimacy));
        assert_eq!(role(QuantityId::Legitimacy), Role::Sink);
        // Wanted workers is written by hire and read by fire.
        assert_eq!(role(QuantityId::WantedWorkers), Role::Derived);
        let loops: Vec<_> = edges()
            .into_iter()
            .filter(|e| e.self_loop && e.kind == EdgeKind::Drives)
            .collect();
        assert!(loops
            .iter()
            .any(|e| e.from == Node::Quantity(QuantityId::Legitimacy)));
    }

    #[test]
    fn causes_walk_upstream_with_depth_and_survive_cycles() {
        let c = causes_of(QuantityId::Vacancies, 1);
        assert!(c.decisions.iter().all(|(_, d)| *d == 1));
        assert!(!c.quantities.is_empty());
        let deep = causes_of(QuantityId::LoanApproved, 8);
        assert!(deep.quantities.len() >= causes_of(QuantityId::LoanApproved, 1).quantities.len());
        // No duplicates, and the start is never its own cause.
        let set: BTreeSet<_> = deep.quantities.iter().map(|x| x.0).collect();
        assert_eq!(set.len(), deep.quantities.len());
        assert!(!set.contains(&QuantityId::LoanApproved));
        // A pure self loop: legitimacy's causes do not include itself.
        let l = causes_of(QuantityId::Legitimacy, 8);
        assert!(l
            .quantities
            .iter()
            .all(|(q, _)| *q != QuantityId::Legitimacy));
        assert_eq!(causes_of(QuantityId::Savings, 8), Causes::default());
        assert_eq!(causes_of(QuantityId::Wage, 0), Causes::default());
    }

    #[test]
    fn components_cover_every_node() {
        let cs = components();
        assert_eq!(cs.len(), nodes().len());
        assert_eq!(cs[0].1, 0);
    }
}
