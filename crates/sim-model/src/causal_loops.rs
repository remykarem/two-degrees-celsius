//! Feedback loops of the causal graph (`docs/features/causality-graph.md` stage 3): strongly
//! connected components and bounded simple cycles of the quantity graph, classified reinforcing or
//! balancing by the signs of their edges. Pure functions over `causal::edges()`; no state, no
//! content, no `HashMap`, no `petgraph` (the graph has about 130 nodes and 330 edges, so Tarjan is
//! about 40 lines and every order stays under our control).
//!
//! The graph: one node per quantity; one edge per `(via, from, to)` `Drives` edge whose effect sign
//! is not `None` (a `none` effect has no direct effect, so it cannot carry feedback). A self-loop
//! (a decision or mechanism that reads and writes the same quantity) is a cycle of length 1.
//!
//! Enumeration is deterministic: a cycle is found only from its smallest quantity (`QuantityId`
//! order), visiting only larger ones, so each cycle appears exactly once in canonical rotation;
//! edges are tried in registry order. The search is bounded by `MAX_LOOP_LEN` quantities per loop
//! and `MAX_LOOPS` loops in all, and reports when either bound cut it short.

use crate::causal::{self, EdgeKind, NetSign, Node};
use sim_types::{QuantityId, Sign};
use std::collections::{BTreeMap, BTreeSet};

/// Longest loop searched, in quantities (edges). Chosen from the measured counts (see the test
/// `loop_counts_are_stable` and `docs/features/causality-graph.md` section 14).
pub const MAX_LOOP_LEN: usize = 8;
/// Most loops kept. The search stops here and sets `Loops::truncated`.
pub const MAX_LOOPS: usize = 20_000;

/// One edge of the quantity graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QEdge {
    pub from: QuantityId,
    pub to: QuantityId,
    pub via: Node,
    pub sign: Sign,
    /// The lag in months the spec itself declares (without the tick-order wait).
    pub spec_lag: u32,
    /// `causal::edge_lag_months`: the spec's lag plus the wait for the earliest registry writer.
    pub lag_months: u32,
    /// The effect is not measured (`declared_unverified`).
    pub unverified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoopKind {
    /// Reinforcing: an even number of `-` edges.
    R,
    /// Balancing: an odd number of `-` edges.
    B,
    /// Some edge can go either way (`varies`), so the loop's kind depends on the state.
    Depends,
}

/// A feedback loop: the quantities in cycle order starting at the smallest, and the edge from each
/// to the next (the last edge returns to the first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loop {
    pub quantities: Vec<QuantityId>,
    pub edges: Vec<QEdge>,
    pub kind: LoopKind,
    /// Months around the loop. Each edge adds its spec's own lag plus the tick-order wait between
    /// the system that writes its input *in this loop* (the previous edge's decision or mechanism)
    /// and the system that reads it. At least 1 for any cycle: a loop cannot close inside one tick,
    /// because systems run in a fixed order and the cycle must return to where it started.
    pub delay_months: u32,
    /// The months each edge adds in this loop (they sum to `delay_months`).
    pub edge_lags: Vec<u32>,
    /// The plain sum of `causal::edge_lag_months` around the loop, which takes the *earliest* of
    /// the registry writers of each input. It can be 0 or smaller than `delay_months` when an input
    /// has several writers (an artefact, not a real same-month loop); kept for the findings.
    pub summed_edge_lags: u32,
    /// Index into `Loops::sccs`.
    pub scc: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loops {
    pub loops: Vec<Loop>,
    /// Non-trivial strongly connected components (more than one quantity, or a self-loop), each
    /// sorted, ordered by their smallest member.
    pub sccs: Vec<Vec<QuantityId>>,
    /// Quantities in no component with a cycle.
    pub acyclic: Vec<QuantityId>,
    /// The search stopped at `MAX_LOOPS` (the length bound `MAX_LOOP_LEN` always applies).
    pub truncated: bool,
}

/// The quantity graph: every signed `Drives` edge, in registry order.
pub fn quantity_edges() -> Vec<QEdge> {
    causal::edges()
        .into_iter()
        .filter(|e| e.kind == EdgeKind::Drives)
        .filter_map(|e| {
            let fx = e.effect?;
            if matches!(fx.sign, Sign::None(_)) {
                return None;
            }
            let (Node::Quantity(from), Node::Quantity(to), Some(via)) = (e.from, e.to, e.via)
            else {
                return None;
            };
            Some(QEdge {
                from,
                to,
                via,
                sign: fx.sign,
                spec_lag: u32::from(fx.lag_months),
                lag_months: causal::edge_lag_months(&e)?,
                unverified: matches!(fx.basis, sim_types::Basis::DeclaredUnverified(_)),
            })
        })
        .collect()
}

/// Tarjan's algorithm over `BTreeSet` adjacency, iterative so the depth cannot overflow. Returns
/// the components of every node (singletons included), each sorted, ordered by smallest member.
pub fn strongly_connected(
    nodes: &[QuantityId],
    adj: &BTreeMap<QuantityId, BTreeSet<QuantityId>>,
) -> Vec<Vec<QuantityId>> {
    let mut index: BTreeMap<QuantityId, usize> = BTreeMap::new();
    let mut low: BTreeMap<QuantityId, usize> = BTreeMap::new();
    let mut on_stack: BTreeSet<QuantityId> = BTreeSet::new();
    let mut stack: Vec<QuantityId> = Vec::new();
    let mut next = 0usize;
    let mut out: Vec<Vec<QuantityId>> = Vec::new();
    for root in nodes {
        if index.contains_key(root) {
            continue;
        }
        // Work stack of (node, its successors, position).
        let mut work: Vec<(QuantityId, Vec<QuantityId>, usize)> = Vec::new();
        let succ = |n: &QuantityId| {
            adj.get(n)
                .map(|s| s.iter().copied().collect())
                .unwrap_or_default()
        };
        index.insert(*root, next);
        low.insert(*root, next);
        next += 1;
        stack.push(*root);
        on_stack.insert(*root);
        work.push((*root, succ(root), 0));
        while let Some((v, ss, pos)) = work.last_mut() {
            let v = *v;
            if *pos < ss.len() {
                let w = ss[*pos];
                *pos += 1;
                #[allow(clippy::map_entry)] // the else branch reads `index` for a node already seen
                if !index.contains_key(&w) {
                    index.insert(w, next);
                    low.insert(w, next);
                    next += 1;
                    stack.push(w);
                    on_stack.insert(w);
                    work.push((w, succ(&w), 0));
                } else if on_stack.contains(&w) {
                    let lw = index[&w];
                    let lv = low[&v];
                    low.insert(v, lv.min(lw));
                }
            } else {
                work.pop();
                if let Some((p, _, _)) = work.last() {
                    let lp = low[p].min(low[&v]);
                    low.insert(*p, lp);
                }
                if low[&v] == index[&v] {
                    let mut comp = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack.remove(&w);
                        comp.push(w);
                        if w == v {
                            break;
                        }
                    }
                    comp.sort();
                    out.push(comp);
                }
            }
        }
    }
    out.sort_by_key(|c| c[0]);
    out
}

/// Months each edge of a cycle adds (see `Loop::delay_months`).
fn cycle_lags(es: &[QEdge]) -> Vec<u32> {
    let n = es.len();
    (0..n)
        .map(|i| {
            let writer = causal::system_of(es[(i + n - 1) % n].via);
            let reader = causal::system_of(es[i].via);
            let wait = match (writer, reader) {
                (Some(w), Some(r)) => u32::from(causal::tick_wait(w, r)),
                _ => 1,
            };
            // The spec's own lag is the edge lag minus the wait the edge was given on its own.
            es[i].spec_lag + wait
        })
        .collect()
}

/// The kind of a cycle from the signs of its edges.
pub fn loop_kind(signs: &[Sign]) -> LoopKind {
    match causal::path_sign(signs) {
        NetSign::Up => LoopKind::R,
        NetSign::Down => LoopKind::B,
        _ => LoopKind::Depends,
    }
}

/// Stable ids of the loops, in order: `loop:` and the quantity wire names sorted and joined with
/// `+` (a loop is named by what is in it, not by where the search found it). Two loops with the
/// same quantities in a different order, or through different decisions, get `~2`, `~3` ... in
/// the loops' own (deterministic) order.
pub fn loop_ids(l: &Loops) -> Vec<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    l.loops
        .iter()
        .map(|x| {
            let mut names: Vec<&str> = x.quantities.iter().map(|q| q.as_str()).collect();
            names.sort_unstable();
            let base = format!("loop:{}", names.join("+"));
            let n = seen.entry(base.clone()).or_insert(0);
            *n += 1;
            if *n == 1 {
                base
            } else {
                format!("{base}~{n}")
            }
        })
        .collect()
}

/// All feedback loops of the graph, bounded (see the module docs).
pub fn loops() -> Loops {
    loops_in(&quantity_edges(), MAX_LOOP_LEN, MAX_LOOPS)
}

pub fn loops_in(all: &[QEdge], max_len: usize, max_loops: usize) -> Loops {
    let mut adj: BTreeMap<QuantityId, BTreeSet<QuantityId>> = BTreeMap::new();
    for e in all {
        adj.entry(e.from).or_default().insert(e.to);
    }
    let nodes: Vec<QuantityId> = QuantityId::ALL.to_vec();
    let comps = strongly_connected(&nodes, &adj);
    let mut sccs: Vec<Vec<QuantityId>> = Vec::new();
    let mut acyclic = Vec::new();
    for c in comps {
        let self_loop = c.len() == 1 && all.iter().any(|e| e.from == c[0] && e.to == c[0]);
        if c.len() > 1 || self_loop {
            sccs.push(c);
        } else {
            acyclic.push(c[0]);
        }
    }
    let scc_of = |q: QuantityId| sccs.iter().position(|c| c.contains(&q));

    // Outgoing edges per quantity, in registry order.
    let mut out_edges: BTreeMap<QuantityId, Vec<QEdge>> = BTreeMap::new();
    for e in all {
        out_edges.entry(e.from).or_default().push(*e);
    }

    struct Search<'a> {
        out_edges: &'a BTreeMap<QuantityId, Vec<QEdge>>,
        scc: &'a [QuantityId],
        start: QuantityId,
        max_len: usize,
        max_loops: usize,
        path: Vec<QuantityId>,
        edges: Vec<QEdge>,
        found: Vec<(Vec<QuantityId>, Vec<QEdge>)>,
        truncated: bool,
    }
    impl Search<'_> {
        fn walk(&mut self, at: QuantityId) {
            let Some(es) = self.out_edges.get(&at) else {
                return;
            };
            for e in es {
                if self.found.len() >= self.max_loops {
                    self.truncated = true;
                    return;
                }
                if e.to == self.start {
                    self.edges.push(*e);
                    self.found.push((self.path.clone(), self.edges.clone()));
                    self.edges.pop();
                } else if e.to > self.start
                    && self.scc.contains(&e.to)
                    && !self.path.contains(&e.to)
                {
                    if self.path.len() >= self.max_len {
                        continue;
                    }
                    self.path.push(e.to);
                    self.edges.push(*e);
                    self.walk(e.to);
                    self.edges.pop();
                    self.path.pop();
                }
            }
        }
    }

    let mut loops = Vec::new();
    let mut truncated = false;
    for (si, scc) in sccs.iter().enumerate() {
        for start in scc {
            let mut s = Search {
                out_edges: &out_edges,
                scc,
                start: *start,
                max_len,
                max_loops: max_loops.saturating_sub(loops.len()),
                path: vec![*start],
                edges: Vec::new(),
                found: Vec::new(),
                truncated: false,
            };
            s.walk(*start);
            truncated |= s.truncated;
            for (qs, es) in s.found {
                let signs: Vec<Sign> = es.iter().map(|e| e.sign).collect();
                let lags = cycle_lags(&es);
                loops.push(Loop {
                    kind: loop_kind(&signs),
                    delay_months: lags.iter().sum(),
                    edge_lags: lags,
                    summed_edge_lags: es.iter().map(|e| e.lag_months).sum(),
                    scc: scc_of(qs[0]).unwrap_or(si),
                    quantities: qs,
                    edges: es,
                });
            }
        }
    }
    // Shortest first, then by quantity order, then by the edges' registry position: stable.
    loops.sort_by(|a, b| {
        a.quantities
            .len()
            .cmp(&b.quantities.len())
            .then_with(|| a.quantities.cmp(&b.quantities))
            .then_with(|| {
                let k = |l: &Loop| l.edges.iter().map(|e| e.via).collect::<Vec<_>>();
                k(a).cmp(&k(b))
            })
    });
    Loops {
        loops,
        sccs,
        acyclic,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: &str) -> QuantityId {
        *QuantityId::ALL
            .iter()
            .find(|x| x.as_str() == s)
            .unwrap_or_else(|| panic!("no quantity {s}"))
    }

    fn names(l: &Loop) -> Vec<&'static str> {
        l.quantities.iter().map(|q| q.as_str()).collect()
    }

    fn has(l: &Loops, kind: LoopKind, members: &[&str]) -> bool {
        l.loops.iter().any(|x| {
            let mut a = names(x);
            a.sort_unstable();
            let mut b = members.to_vec();
            b.sort_unstable();
            x.kind == kind && a == b
        })
    }

    #[test]
    fn every_loop_is_a_real_cycle_in_the_edge_list() {
        let all = quantity_edges();
        let l = loops();
        assert!(!l.loops.is_empty());
        for x in &l.loops {
            let n = x.quantities.len();
            assert_eq!(x.edges.len(), n);
            assert!(n <= MAX_LOOP_LEN);
            let mut distinct = x.quantities.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(distinct.len(), n, "a quantity twice in {:?}", names(x));
            assert_eq!(
                x.quantities[0], distinct[0],
                "rotation starts at the smallest"
            );
            for (i, e) in x.edges.iter().enumerate() {
                assert_eq!(e.from, x.quantities[i]);
                assert_eq!(e.to, x.quantities[(i + 1) % n], "the cycle closes");
                assert!(
                    all.contains(e),
                    "edge {:?} -> {:?} is not in the graph",
                    e.from,
                    e.to
                );
            }
        }
    }

    #[test]
    fn classification_follows_the_count_of_minus_edges() {
        for x in &loops().loops {
            let varies = x.edges.iter().any(|e| matches!(e.sign, Sign::Varies(_)));
            let minus = x
                .edges
                .iter()
                .filter(|e| matches!(e.sign, Sign::Down))
                .count();
            let want = if varies {
                LoopKind::Depends
            } else if minus % 2 == 1 {
                LoopKind::B
            } else {
                LoopKind::R
            };
            assert_eq!(x.kind, want, "{:?}", names(x));
            assert!(x.edges.iter().all(|e| !matches!(e.sign, Sign::None(_))));
        }
    }

    #[test]
    fn scc_partition_is_complete_and_correct() {
        let all = quantity_edges();
        let l = loops();
        // Every quantity is in exactly one component or is acyclic.
        let mut seen: Vec<QuantityId> = l.sccs.iter().flatten().copied().collect();
        seen.extend(l.acyclic.iter().copied());
        seen.sort();
        let mut want = QuantityId::ALL.to_vec();
        want.sort();
        assert_eq!(seen, want);
        // Brute force: reachability by repeated relaxation.
        let reach = |a: QuantityId| {
            let mut r: BTreeSet<QuantityId> = BTreeSet::from([a]);
            loop {
                let before = r.len();
                for e in &all {
                    if r.contains(&e.from) {
                        r.insert(e.to);
                    }
                }
                if r.len() == before {
                    return r;
                }
            }
        };
        for c in &l.sccs {
            for a in c {
                let ra = reach(*a);
                for b in c {
                    assert!(
                        ra.contains(b),
                        "{a:?} cannot reach {b:?} in its own component"
                    );
                }
            }
        }
        for (i, c) in l.sccs.iter().enumerate() {
            for d in &l.sccs[i + 1..] {
                assert!(!reach(c[0]).contains(&d[0]) || !reach(d[0]).contains(&c[0]));
            }
        }
        for a in &l.acyclic {
            let on_cycle = all.iter().any(|e| e.from == *a && reach(e.to).contains(a));
            assert!(!on_cycle, "{a:?} is on a cycle but not in a component");
        }
        // Every loop lies in one component, and every component with a cycle has a loop.
        for x in &l.loops {
            assert!(x.quantities.iter().all(|m| l.sccs[x.scc].contains(m)));
        }
        for i in 0..l.sccs.len() {
            assert!(
                l.loops.iter().any(|x| x.scc == i),
                "component {i} has no loop"
            );
        }
    }

    #[test]
    fn output_is_deterministic_and_each_loop_appears_once() {
        let a = loops();
        let b = loops();
        assert_eq!(a, b);
        assert_eq!(loop_ids(&a), loop_ids(&b));
        let mut keys: Vec<(Vec<QuantityId>, Vec<Node>)> = a
            .loops
            .iter()
            .map(|x| {
                (
                    x.quantities.clone(),
                    x.edges.iter().map(|e| e.via).collect(),
                )
            })
            .collect();
        let n = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), n, "a loop found twice");
        let ids = loop_ids(&a);
        let mut u = ids.clone();
        u.sort();
        u.dedup();
        assert_eq!(u.len(), ids.len(), "loop ids are unique");
    }

    #[test]
    fn bounds_cut_the_search_and_say_so() {
        let all = quantity_edges();
        let short = loops_in(&all, 4, 1_000_000);
        let long = loops_in(&all, 8, 1_000_000);
        assert!(short.loops.len() < long.loops.len());
        assert!(short.loops.iter().all(|x| x.quantities.len() <= 4));
        assert!(!long.truncated);
        let capped = loops_in(&all, 8, 10);
        assert_eq!(capped.loops.len(), 10);
        assert!(capped.truncated);
        // The shipped bounds leave room.
        assert!(!loops().truncated);
        assert!(loops().loops.len() * 5 < MAX_LOOPS);
    }

    #[test]
    fn counts_are_pinned() {
        // Measured (docs/features/causality-graph.md section 14); a change here is a change to
        // the registry's effects and should be read, not just updated.
        let l = loops();
        let r = l.loops.iter().filter(|x| x.kind == LoopKind::R).count();
        let b = l.loops.iter().filter(|x| x.kind == LoopKind::B).count();
        let d = l
            .loops
            .iter()
            .filter(|x| x.kind == LoopKind::Depends)
            .count();
        assert_eq!((l.loops.len(), r, b, d), (203, 89, 107, 7));
        let sizes: Vec<usize> = l.sccs.iter().map(Vec::len).collect();
        assert_eq!(sizes, vec![46, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn a_loop_cannot_close_inside_one_tick() {
        let l = loops();
        assert!(l.loops.iter().all(|x| x.delay_months >= 1));
        // The longest delay found, and the loops whose plain edge-lag sum is smaller (the
        // earliest-writer artefact the module docs describe), are reported in section 14.
        let longest = l.loops.iter().map(|x| x.delay_months).max().unwrap_or(0);
        assert!(longest >= 6);
    }

    #[test]
    fn known_feedback_loops_have_the_expected_kind() {
        let l = loops();
        // Emissions -> climate stress -> drought -> weather factor -> output -> emissions: balancing
        // (the damage lowers output, and output makes the emissions).
        assert!(has(
            &l,
            LoopKind::B,
            &[
                "weather_factor",
                "output",
                "direct_emissions",
                "gross_emissions",
                "cumulative_emissions",
                "climate_stress",
                "drought"
            ]
        ));
        // Wages -> unit cost -> price -> demand -> expected sales -> workers wanted -> wage: balancing.
        assert!(has(
            &l,
            LoopKind::B,
            &[
                "expected_sales",
                "wanted_workers",
                "workers",
                "unemployment",
                "wage",
                "unit_cost",
                "price",
                "demand_units"
            ]
        ));
        // The same wage feeds incomes and so demand: reinforcing.
        assert!(has(
            &l,
            LoopKind::R,
            &[
                "expected_sales",
                "wanted_workers",
                "workers",
                "unemployment",
                "wage",
                "last_disposable",
                "consumption_budget",
                "demand_units"
            ]
        ));
        // The shop: a dearer price sells less, so stock piles up: a one-step balancing loop pair.
        assert!(has(&l, LoopKind::B, &["inventory", "units_sold"]));
        // Self-loops: a stock carries over (reinforcing) and rent depends on the state.
        assert!(has(&l, LoopKind::R, &["forest_stock"]));
        assert!(has(&l, LoopKind::Depends, &["rent"]));
        // A loop whose edges include a varies is never called R or B.
        assert!(l
            .loops
            .iter()
            .any(|x| x.kind == LoopKind::Depends && x.quantities.len() > 1));
    }

    /// One of the nine teaching loops of the original hand-written design table (findings:
    /// `docs/features/causality-graph.md` section 14) and what the code supports. `members`
    /// is the quantity set of the generated loop that carries it (`None`: nothing in the registry
    /// supports it), `kind` that loop's kind.
    struct DocRow {
        row: u8,
        story: &'static str,
        doc_kind: LoopKind,
        found: Option<(LoopKind, &'static [&'static str])>,
    }

    fn doc_rows() -> Vec<DocRow> {
        use LoopKind::{B, R};
        vec![
            DocRow {
                row: 1,
                story: "profit -> investment -> output -> profit",
                doc_kind: R,
                found: Some((
                    R,
                    &[
                        "inventory",
                        "units_sold",
                        "revenue",
                        "profit",
                        "investment_plan",
                        "capital",
                        "capital_output",
                        "output",
                    ],
                )),
            },
            DocRow {
                row: 2,
                story: "wealth -> lobbying -> weaker policy -> profit -> wealth",
                doc_kind: R,
                found: None,
            },
            DocRow {
                row: 3,
                story: "debt -> interest -> need for profit -> investment",
                doc_kind: R,
                found: None,
            },
            DocRow {
                row: 4,
                story: "capital income up, wage share down -> demand falls -> layoffs (nearest: the wage bill squeezes payouts, savings and demand)",
                doc_kind: B,
                found: Some((
                    B,
                    &[
                        "expected_sales",
                        "wanted_workers",
                        "workers",
                        "payroll",
                        "dividends",
                        "savings",
                        "consumption_budget",
                        "demand_units",
                    ],
                )),
            },
            DocRow {
                row: 5,
                story: "unemployment -> lower wages -> higher profit -> hiring",
                doc_kind: B,
                found: Some((
                    B,
                    &[
                        "expected_sales",
                        "wanted_workers",
                        "workers",
                        "unemployment",
                        "wage",
                        "unit_cost",
                        "price",
                        "demand_units",
                    ],
                )),
            },
            DocRow {
                row: 6,
                story: "scarcity -> price rises -> demand falls",
                doc_kind: B,
                found: None,
            },
            DocRow {
                row: 7,
                story: "over-harvest -> commons collapses -> farm output falls",
                doc_kind: B,
                found: Some((B, &["commons_stock", "output"])),
            },
            DocRow {
                row: 8,
                story: "emissions -> budget used -> climate damage -> lower output",
                doc_kind: R,
                // The story calls it reinforcing; through output it is balancing (output makes
                // the emissions). The reinforcing version runs through the forest sink.
                found: Some((
                    B,
                    &[
                        "weather_factor",
                        "output",
                        "direct_emissions",
                        "gross_emissions",
                        "cumulative_emissions",
                        "climate_stress",
                        "drought",
                    ],
                )),
            },
            DocRow {
                row: 9,
                story: "carbon price -> green investment -> lower footprint",
                doc_kind: B,
                // The carbon price is a lever (nothing writes it), so it cannot be on a cycle; it
                // acts on this balancing loop from outside.
                found: Some((B, &["footprint", "green_tech"])),
            },
        ]
    }

    #[test]
    fn teaching_loops_are_validated_against_the_generated_loops() {
        let l = loops();
        let mut unsupported = Vec::new();
        let mut kind_disagrees = Vec::new();
        for r in doc_rows() {
            match r.found {
                Some((kind, members)) => {
                    assert!(
                        has(&l, kind, members),
                        "row {} ({}) names a loop the code does not generate",
                        r.row,
                        r.story
                    );
                    if kind != r.doc_kind {
                        kind_disagrees.push(r.row);
                    }
                }
                None => unsupported.push(r.row),
            }
        }
        // The findings of the original design table, pinned so they cannot drift.
        assert_eq!(unsupported, vec![2, 3, 6]);
        assert_eq!(kind_disagrees, vec![8]);
        // Why each unsupported row has no loop:
        let edges = quantity_edges();
        // 2: lobbying spend is a sink: nothing reads it (the lobbying outcome is not a quantity).
        assert!(!edges.iter().any(|e| e.from == q("lobby_spend")));
        // 3: debt is a starting point: nothing writes it.
        assert!(!edges.iter().any(|e| e.to == q("debt")));
        // 6: price is read from unit cost, the markup and its own past only: no scarcity, no demand (MK-1).
        let mut into: Vec<&str> = edges
            .iter()
            .filter(|e| e.to == q("price"))
            .map(|e| e.from.as_str())
            .collect();
        into.sort_unstable();
        assert_eq!(into, vec!["markup", "price", "unit_cost"]);
        // 8: the reinforcing overshoot loop exists through the forest sink.
        assert!(has(
            &l,
            LoopKind::R,
            &[
                "forest_stock",
                "forest_sink",
                "cumulative_emissions",
                "climate_stress",
                "fire"
            ]
        ));
    }

    #[test]
    fn levers_are_never_on_a_loop() {
        // A starting point has no writer, so it has no incoming edge: levers act on loops from outside.
        let l = loops();
        for x in &l.loops {
            for m in &x.quantities {
                assert!(
                    crate::causal::role(*m) != crate::causal::Role::Source,
                    "{m:?} is a starting point on a loop"
                );
            }
        }
    }
}
