//! The causal graph of the registry (`docs/features/decision-points.md`, "Future: causality
//! graph", and `docs/features/causality-graph.md`): pure functions over `DECISIONS` and
//! `MECHANISMS`, no state, no content. Nodes are quantities, decisions and mechanisms (plain
//! physics and accounting code, stage 1). An edge says "this decision or mechanism reads that
//! quantity" or "writes that one", and a derived edge links a quantity one reads to each quantity
//! it writes ("b -> a via X"). The wire (`sim-api::decisions`) adds labels from
//! `content/decisions.json`. Order is stable: quantities in `QuantityId::ALL` order, then decisions
//! in `DECISIONS` order, then mechanisms in `MECHANISMS` order; no hash maps.

use crate::decisions::{DECISIONS, MECHANISMS};
use sim_types::{DecisionId, MechanismId, QuantityId};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Node {
    Quantity(QuantityId),
    Decision(DecisionId),
    Mechanism(MechanismId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// quantity -> decision or mechanism: it reads it.
    Reads,
    /// decision or mechanism -> quantity: it writes it.
    Writes,
    /// quantity -> quantity, `via` the decision or mechanism that reads the first and writes the second.
    Drives,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    pub from: Node,
    pub to: Node,
    pub kind: EdgeKind,
    /// The decision or mechanism a `Drives` edge goes through.
    pub via: Option<Node>,
    /// The same decision or mechanism both reads and writes this quantity (for example legitimacy).
    pub self_loop: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Nothing in the registry writes it: a state variable, a lever or a setting that starts a chain.
    Source,
    /// Written by a decision or mechanism and read by one that does not write it.
    Derived,
    /// Written, but read by nothing except those that write it: an outcome.
    Sink,
}

/// A registered decision or mechanism, as the graph sees it: what it reads and writes.
#[derive(Clone, Copy)]
struct Actor {
    node: Node,
    inputs: &'static [QuantityId],
    outputs: &'static [QuantityId],
}

/// Decisions in registry order, then mechanisms in registry order.
fn actors() -> impl Iterator<Item = Actor> {
    DECISIONS
        .iter()
        .map(|d| Actor {
            node: Node::Decision(d.id),
            inputs: d.inputs,
            outputs: d.outputs,
        })
        .chain(MECHANISMS.iter().map(|m| Actor {
            node: Node::Mechanism(m.id),
            inputs: m.inputs,
            outputs: m.outputs,
        }))
}

/// Every quantity, then every decision, then every mechanism, in stable order.
pub fn nodes() -> Vec<Node> {
    QuantityId::ALL
        .iter()
        .map(|q| Node::Quantity(*q))
        .chain(DECISIONS.iter().map(|d| Node::Decision(d.id)))
        .chain(MECHANISMS.iter().map(|m| Node::Mechanism(m.id)))
        .collect()
}

/// The decisions and mechanisms that write a quantity, in registry order.
pub fn writers(q: QuantityId) -> Vec<Node> {
    actors()
        .filter(|a| a.outputs.contains(&q))
        .map(|a| a.node)
        .collect()
}

/// The decisions and mechanisms that read a quantity, in registry order.
pub fn readers(q: QuantityId) -> Vec<Node> {
    actors()
        .filter(|a| a.inputs.contains(&q))
        .map(|a| a.node)
        .collect()
}

pub fn role(q: QuantityId) -> Role {
    if !actors().any(|a| a.outputs.contains(&q)) {
        Role::Source
    } else if actors()
        .filter(|a| a.inputs.contains(&q))
        .all(|a| a.outputs.contains(&q))
    {
        Role::Sink
    } else {
        Role::Derived
    }
}

/// Some decision or mechanism reads and writes this quantity.
pub fn has_self_loop(q: QuantityId) -> bool {
    actors().any(|a| a.inputs.contains(&q) && a.outputs.contains(&q))
}

/// All edges: reads and writes per decision then per mechanism in registry order, then the
/// derived links in the same order.
pub fn edges() -> Vec<Edge> {
    let mut out = Vec::new();
    for a in actors() {
        for q in a.inputs {
            out.push(Edge {
                from: Node::Quantity(*q),
                to: a.node,
                kind: EdgeKind::Reads,
                via: None,
                self_loop: a.outputs.contains(q),
            });
        }
        for q in a.outputs {
            out.push(Edge {
                from: a.node,
                to: Node::Quantity(*q),
                kind: EdgeKind::Writes,
                via: None,
                self_loop: a.inputs.contains(q),
            });
        }
    }
    for a in actors() {
        for i in a.inputs {
            for o in a.outputs {
                out.push(Edge {
                    from: Node::Quantity(*i),
                    to: Node::Quantity(*o),
                    kind: EdgeKind::Drives,
                    via: Some(a.node),
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

/// What lies upstream of a quantity: the quantities, decisions and mechanisms that can change it,
/// each with the number of steps (decisions or mechanisms crossed) to reach it (1 = writes the
/// quantity directly, or is read by one that does). Nearest first, then registry order.
/// `max_depth` limits the walk.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Causes {
    pub quantities: Vec<(QuantityId, usize)>,
    pub decisions: Vec<(DecisionId, usize)>,
    pub mechanisms: Vec<(MechanismId, usize)>,
}

pub fn causes_of(q: QuantityId, max_depth: usize) -> Causes {
    let mut seen_q: BTreeSet<QuantityId> = BTreeSet::from([q]);
    let mut seen_n: BTreeSet<Node> = BTreeSet::new();
    let mut frontier = vec![q];
    let mut out = Causes::default();
    for depth in 1..=max_depth {
        let mut next: Vec<QuantityId> = Vec::new();
        for f in &frontier {
            for a in actors().filter(|a| a.outputs.contains(f)) {
                if seen_n.insert(a.node) {
                    match a.node {
                        Node::Decision(d) => out.decisions.push((d, depth)),
                        Node::Mechanism(m) => out.mechanisms.push((m, depth)),
                        Node::Quantity(_) => {}
                    }
                }
                for i in a.inputs {
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

/// Source and sink counts over all quantities: (sources, sinks, derived).
pub fn role_counts() -> (usize, usize, usize) {
    let mut c = (0, 0, 0);
    for q in QuantityId::ALL {
        match role(*q) {
            Role::Source => c.0 += 1,
            Role::Sink => c.1 += 1,
            Role::Derived => c.2 += 1,
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decisions::{mechanism, spec};
    use sim_types::QuantityId as Q;

    fn io(n: Node) -> (&'static [QuantityId], &'static [QuantityId]) {
        match n {
            Node::Decision(d) => {
                let s = spec(d).unwrap();
                (s.inputs, s.outputs)
            }
            Node::Mechanism(m) => {
                let s = mechanism(m).unwrap();
                (s.inputs, s.outputs)
            }
            Node::Quantity(_) => panic!("a quantity is not a spec"),
        }
    }

    #[test]
    fn every_edge_comes_from_a_spec_and_every_spec_io_has_an_edge() {
        let es = edges();
        for e in &es {
            match (e.kind, e.from, e.to, e.via) {
                (EdgeKind::Reads, Node::Quantity(q), n, None) => {
                    assert!(io(n).0.contains(&q));
                }
                (EdgeKind::Writes, n, Node::Quantity(q), None) => {
                    assert!(io(n).1.contains(&q));
                }
                (EdgeKind::Drives, Node::Quantity(i), Node::Quantity(o), Some(n)) => {
                    assert!(io(n).0.contains(&i) && io(n).1.contains(&o));
                    assert_eq!(e.self_loop, i == o);
                }
                other => panic!("malformed edge {other:?}"),
            }
        }
        for a in actors() {
            for q in a.inputs {
                assert!(es.iter().any(|e| e.kind == EdgeKind::Reads
                    && e.from == Node::Quantity(*q)
                    && e.to == a.node));
            }
            for q in a.outputs {
                assert!(es.iter().any(|e| e.kind == EdgeKind::Writes
                    && e.from == a.node
                    && e.to == Node::Quantity(*q)));
            }
            for i in a.inputs {
                for o in a.outputs {
                    assert_eq!(
                        es.iter()
                            .filter(|e| e.kind == EdgeKind::Drives
                                && e.via == Some(a.node)
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
        assert_eq!(
            ns.len(),
            QuantityId::ALL.len() + DECISIONS.len() + MECHANISMS.len()
        );
        let set: BTreeSet<_> = ns.iter().collect();
        assert_eq!(set.len(), ns.len(), "node ids are unique");
        for e in edges() {
            assert!(ns.contains(&e.from) && ns.contains(&e.to));
        }
        assert_eq!(edges(), edges());
        assert_eq!(components(), components());
        // Every quantity is used by some decision or mechanism.
        for q in QuantityId::ALL {
            assert!(
                actors().any(|a| a.inputs.contains(q) || a.outputs.contains(q)),
                "{} is in no spec",
                q.as_str()
            );
        }
    }

    #[test]
    fn roles_and_self_loops() {
        // Nothing writes a lever or a firm's markup, nor the treasury's cash: they start chains.
        assert_eq!(role(Q::CarbonPrice), Role::Source);
        assert_eq!(role(Q::Markup), Role::Source);
        assert_eq!(role(Q::TreasuryCash), Role::Source);
        // Stage 1: the mechanisms write what used to be a starting point.
        assert_eq!(role(Q::Savings), Role::Derived);
        assert_ne!(role(Q::Unemployment), Role::Source);
        assert_ne!(role(Q::WeatherFactor), Role::Source);
        // Legitimacy is read and written by one decision only: a sink with a self loop.
        assert!(has_self_loop(Q::Legitimacy));
        assert_eq!(role(Q::Legitimacy), Role::Sink);
        // Wanted workers is written by hire and read by fire.
        assert_eq!(role(Q::WantedWorkers), Role::Derived);
        // A mechanism's self loop (the technology that grows on itself).
        assert!(has_self_loop(Q::Technology));
        let loops: Vec<_> = edges()
            .into_iter()
            .filter(|e| e.self_loop && e.kind == EdgeKind::Drives)
            .collect();
        assert!(loops
            .iter()
            .any(|e| e.from == Node::Quantity(Q::Legitimacy)));
        assert!(loops
            .iter()
            .any(|e| e.from == Node::Quantity(Q::Technology)));
    }

    #[test]
    fn causes_walk_upstream_with_depth_and_survive_cycles() {
        let c = causes_of(Q::Vacancies, 1);
        assert!(c.decisions.iter().all(|(_, d)| *d == 1));
        assert!(c.mechanisms.iter().all(|(_, d)| *d == 1));
        assert!(!c.quantities.is_empty());
        let deep = causes_of(Q::LoanApproved, 8);
        assert!(deep.quantities.len() >= causes_of(Q::LoanApproved, 1).quantities.len());
        // No duplicates, and the start is never its own cause.
        let set: BTreeSet<_> = deep.quantities.iter().map(|x| x.0).collect();
        assert_eq!(set.len(), deep.quantities.len());
        assert!(!set.contains(&Q::LoanApproved));
        // A pure self loop: legitimacy's causes do not include itself.
        let l = causes_of(Q::Legitimacy, 8);
        assert!(l.quantities.iter().all(|(q, _)| *q != Q::Legitimacy));
        assert_eq!(causes_of(Q::TreasuryCash, 8), Causes::default());
        assert_eq!(causes_of(Q::Wage, 0), Causes::default());
    }

    #[test]
    fn components_cover_every_node() {
        let cs = components();
        assert_eq!(cs.len(), nodes().len());
        assert_eq!(cs[0].1, 0);
    }

    // ---- stage 1: mechanisms ----

    #[test]
    fn mechanism_ids_are_unique_and_each_has_one_spec_and_one_owner() {
        for id in MechanismId::ALL {
            assert_eq!(
                MECHANISMS.iter().filter(|m| m.id == *id).count(),
                1,
                "{}",
                id.as_str()
            );
        }
        assert_eq!(MECHANISMS.len(), MechanismId::ALL.len());
        let names: BTreeSet<_> = MechanismId::ALL.iter().map(|i| i.as_str()).collect();
        assert_eq!(names.len(), MechanismId::ALL.len(), "wire names are unique");
        // A mechanism's id never collides with a decision's (they are separate node kinds, but a
        // shared name would make the Agents text ambiguous).
        for id in MechanismId::ALL {
            assert!(
                !DecisionId::ALL.iter().any(|d| d.as_str() == id.as_str()),
                "{} is also a decision",
                id.as_str()
            );
        }
        // Exactly one owner kind per mechanism: it is derived from the id.
        for m in MECHANISMS {
            assert_eq!(m.owner(), m.id.owner());
            assert!(!m.inputs.is_empty() && !m.outputs.is_empty(), "{:?}", m.id);
            assert!(!m.rule.is_empty());
            let i: BTreeSet<_> = m.inputs.iter().collect();
            assert_eq!(i.len(), m.inputs.len(), "{:?} lists an input twice", m.id);
            let o: BTreeSet<_> = m.outputs.iter().collect();
            assert_eq!(o.len(), m.outputs.len(), "{:?} lists an output twice", m.id);
            let p: BTreeSet<_> = m.params.iter().collect();
            assert_eq!(p.len(), m.params.len(), "{:?} lists a param twice", m.id);
        }
    }

    /// The named example chains of DP-13: each one is connected end to end through the registry.
    #[test]
    fn the_dp13_chains_connect() {
        let upstream = |q: QuantityId| -> BTreeSet<QuantityId> {
            causes_of(q, 40)
                .quantities
                .into_iter()
                .map(|x| x.0)
                .collect()
        };
        // Firm emissions -> gross emissions -> carbon budget -> stress -> drought -> weather
        // factor -> output -> hiring: hiring's causes reach back to what firms emit.
        let hire = upstream(Q::WantedWorkers);
        for q in [
            Q::WeatherFactor,
            Q::Drought,
            Q::ClimateStress,
            Q::CumulativeEmissions,
            Q::GrossEmissions,
            Q::DirectEmissions,
            Q::Output,
        ] {
            assert!(
                hire.contains(&q),
                "{} is not upstream of hiring",
                q.as_str()
            );
        }
        // The same chain step by step: each link is a derived edge of the graph.
        let step = |from: QuantityId, to: QuantityId| {
            edges().iter().any(|e| {
                e.kind == EdgeKind::Drives
                    && e.from == Node::Quantity(from)
                    && e.to == Node::Quantity(to)
            })
        };
        assert!(step(Q::GrossEmissions, Q::CumulativeEmissions));
        assert!(step(Q::CumulativeEmissions, Q::ClimateStress));
        assert!(step(Q::ClimateStress, Q::Drought));
        assert!(step(Q::Drought, Q::WeatherFactor));
        assert!(step(Q::WeatherFactor, Q::Output));
        assert!(step(Q::Output, Q::Inventory));
        assert!(step(Q::Inventory, Q::WantedWorkers));
        // Wages -> unit cost -> price (the cost-push chain) and wages -> savings -> spending.
        assert!(upstream(Q::Price).contains(&Q::Wage));
        assert!(upstream(Q::ConsumptionBudget).contains(&Q::Wage));
        // Nature -> wellbeing -> legitimacy, and rent -> disposable income -> spending.
        assert!(upstream(Q::Legitimacy).contains(&Q::ForestStock));
        assert!(upstream(Q::Legitimacy).contains(&Q::GrossEmissions));
        assert!(upstream(Q::ConsumptionBudget).contains(&Q::Rent));
        // Revenue -> profit -> taxes -> the treasury's funding decision.
        assert!(upstream(Q::Borrowing).contains(&Q::TreasuryTaxes));
        assert!(upstream(Q::TreasuryTaxes).contains(&Q::Revenue));
    }

    #[test]
    fn mechanisms_cut_the_starting_points() {
        let (sources, sinks, derived) = role_counts();
        assert_eq!(sources + sinks + derived, QuantityId::ALL.len());
        // Stage 0 had 65 sources among 99 quantities; the mechanisms must cut that clearly.
        assert!(sources < 65, "{sources} starting points");
        if std::env::var("LIST_SOURCES").is_ok() {
            for q in QuantityId::ALL {
                if role(*q) == Role::Source {
                    println!("source {}", q.as_str());
                }
            }
        }
        println!(
            "quantities {} sources {sources} sinks {sinks} derived {derived} mechanisms {} edges {}",
            QuantityId::ALL.len(),
            MECHANISMS.len(),
            edges().len()
        );
        // The remaining starting points are real inputs: levers, the player's settings, private
        // working variables nothing computes. Every one is either read by something or a lever.
        for q in QuantityId::ALL {
            if role(*q) == Role::Source {
                assert!(
                    !readers(*q).is_empty(),
                    "{} is an unread source",
                    q.as_str()
                );
            }
        }
    }

    #[test]
    fn writers_and_readers_list_mechanisms() {
        assert!(writers(Q::WeatherFactor)
            .iter()
            .all(|n| matches!(n, Node::Mechanism(_))));
        assert_eq!(writers(Q::WeatherFactor).len(), 3);
        assert!(readers(Q::WeatherFactor).contains(&Node::Decision(DecisionId::FirmHire)));
        assert_eq!(
            writers(Q::GrossEmissions),
            vec![Node::Mechanism(MechanismId::EmissionsAccounting)]
        );
    }
}
