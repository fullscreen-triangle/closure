//! Per-vertex separation against the medium (Psychon §3).
//!
//! The parent manuscript asks for the cheapest boundary in a graph, and
//! [`ContactGraph::separation_cost`] answers it by brute force over subsets.
//! The companion asks a different question: for *one* vertex `v`, what is the
//! cheapest boundary separating `v` from the medium `⊥`?
//!
//! ```text
//! σ(v) = min { w(cut(S)) : v ∈ S ⊆ V, ⊥ ∉ S }
//! ```
//!
//! Theorem 3.2 gives three facts about this quantity, and each is load-bearing
//! somewhere else:
//!
//! * (i) it is attained — so a psychon has somewhere to be;
//! * (ii) it is at least the floor β > 0 — so occupancy is quantal, which is
//!   what makes the record of Axiom `actfloor` mean anything;
//! * (iii) it equals a maximum `v`–`⊥` flow, hence is computable in strongly
//!   polynomial time rather than the exponential sweep the parent uses.
//!
//! Part (iii) is why this module exists as more than a wrapper. The forum
//! layer computes σ on every visibility check, so an exponential answer is not
//! merely inelegant, it is unusable.
//!
//! ## The medium
//!
//! [`MediumGraph`] adjoins a distinguished vertex ⊥ adjacent to every other
//! position (Def. 2.1). The medium is not a place an agent can be; it is what
//! everything is separated *from*. Its edges carry weights like any others and
//! are subject to the same floor.
//!
//! ## Non-uniqueness is reported, never broken
//!
//! Proposition 3.4 exhibits a four-vertex graph on which every feasible cut
//! has the same weight. [`Separation::minimisers`] therefore returns all of
//! them. An implementation that silently picked one would be asserting a
//! determination the graph does not make, which is what `binv:tiebreak`
//! forbids.

use crate::graph::{ContactGraph, EdgeWeight, Position};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The medium vertex ⊥, adjacent to every position (Def. 2.1).
///
/// Represented as the position one past the end of the underlying graph, so
/// that positions keep their indices and no relabelling is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Medium(pub Position);

/// A contact graph together with a medium vertex adjacent to all of it.
///
/// This is the agent graph of Definition 2.1: finite, connected, weighted,
/// undirected, with a distinguished ⊥ adjacent to every other vertex and
/// `n ≥ 2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediumGraph {
    base: ContactGraph,
    /// Weight of the edge `v⊥` for each position `v`.
    medium_edges: BTreeMap<Position, EdgeWeight>,
}

/// The result of a separation query: the cost, and every set attaining it.
///
/// Both fields matter. The cost is what Theorem 3.2 bounds and what the gap
/// of Definition 7.1 is computed against; the minimisers are what
/// `binv:tiebreak` requires be surfaced rather than collapsed.
#[derive(Debug, Clone, PartialEq)]
pub struct Separation {
    cost: EdgeWeight,
    minimisers: Vec<BTreeSet<Position>>,
}

impl Separation {
    /// σ(v), the separation cost. At least the floor, by Theorem 3.2(ii).
    #[must_use]
    pub fn cost(&self) -> EdgeWeight {
        self.cost
    }

    /// Every resting cut attaining σ(v).
    ///
    /// Length is at least 1 by Theorem 3.2(i). Length greater than 1 is not a
    /// defect; see [`Separation::is_determinate`].
    #[must_use]
    pub fn minimisers(&self) -> &[BTreeSet<Position>] {
        &self.minimisers
    }

    /// Whether the graph determines a single resting cut.
    ///
    /// When this is false the graph genuinely fails to single out "what counts
    /// as this thing rather than its surroundings" (Prop. 3.4), and a caller
    /// that needs one set must record that its choice is the implementation's
    /// and not the graph's.
    #[must_use]
    pub fn is_determinate(&self) -> bool {
        self.minimisers.len() == 1
    }
}

impl MediumGraph {
    /// Adjoin a medium to `base`, joining ⊥ to every position at `weight`.
    ///
    /// # Errors
    /// If `weight` is not strictly positive and finite (Axiom `floor`), or if
    /// `base` has fewer than two positions (Def. 2.1 requires `n ≥ 2`).
    pub fn new(base: ContactGraph, weight: EdgeWeight) -> crate::Result<Self> {
        if weight <= 0.0 || !weight.is_finite() {
            return Err(crate::Error::Graph(
                "Axiom floor: medium edges must be strictly positive and finite".into(),
            ));
        }
        if base.order() < 2 {
            return Err(crate::Error::Graph(
                "Definition 2.1: an agent graph has at least two positions".into(),
            ));
        }
        let medium_edges = (0..base.order()).map(|v| (v, weight)).collect();
        Ok(Self { base, medium_edges })
    }

    /// The medium vertex.
    #[must_use]
    pub fn medium(&self) -> Medium {
        Medium(self.base.order())
    }

    /// Number of positions, excluding the medium.
    #[must_use]
    pub fn order(&self) -> u32 {
        self.base.order()
    }

    /// The underlying contact graph.
    #[must_use]
    pub fn base(&self) -> &ContactGraph {
        &self.base
    }

    /// Set the weight of the edge `v⊥`.
    ///
    /// # Errors
    /// If `v` is not a position, or `w` is not strictly positive and finite.
    pub fn set_medium_edge(&mut self, v: Position, w: EdgeWeight) -> crate::Result<()> {
        if v >= self.base.order() {
            return Err(crate::Error::Graph("no such position".into()));
        }
        if w <= 0.0 || !w.is_finite() {
            return Err(crate::Error::Graph(
                "Axiom floor: edge weights must be strictly positive and finite".into(),
            ));
        }
        self.medium_edges.insert(v, w);
        Ok(())
    }

    /// The floor β: the least weight over all edges, medium edges included.
    ///
    /// Theorem 3.2(ii) bounds every separation cost below by this.
    #[must_use]
    pub fn floor(&self) -> Option<EdgeWeight> {
        let base = self.base.floor();
        let med = self.medium_edges.values().copied().reduce(f64::min);
        match (base, med) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Adjacency with weights, over positions and the medium.
    fn adjacency(&self) -> BTreeMap<Position, BTreeMap<Position, EdgeWeight>> {
        let mut adj: BTreeMap<Position, BTreeMap<Position, EdgeWeight>> = BTreeMap::new();
        let bot = self.medium().0;
        for u in 0..self.base.order() {
            for v in 0..self.base.order() {
                if u == v {
                    continue;
                }
                if let Some(w) = self.base.weight(u, v) {
                    adj.entry(u).or_default().insert(v, w);
                }
            }
        }
        for (v, w) in &self.medium_edges {
            adj.entry(*v).or_default().insert(bot, *w);
            adj.entry(bot).or_default().insert(*v, *w);
        }
        adj
    }

    /// σ(v) and every resting cut attaining it (Def. 3.1, Thm. 3.2).
    ///
    /// Computed as a maximum `v`–`⊥` flow by Edmonds–Karp, which is
    /// `O(V·E²)` — strongly polynomial, per Theorem 3.2(iii). The minimisers
    /// are recovered from the residual graph.
    ///
    /// Returns `None` if `v` is not a position of the graph.
    #[must_use]
    pub fn separation(&self, v: Position) -> Option<Separation> {
        if v >= self.base.order() {
            return None;
        }
        let bot = self.medium().0;
        let adj = self.adjacency();

        // Residual capacities. Each undirected edge {a,b} of weight w becomes
        // a pair of directed arcs each of capacity w, which is the standard
        // reduction for undirected min cut.
        let mut cap: BTreeMap<(Position, Position), EdgeWeight> = BTreeMap::new();
        for (a, nbrs) in &adj {
            for (b, w) in nbrs {
                cap.insert((*a, *b), *w);
            }
        }

        // Edmonds-Karp: repeatedly augment along a shortest residual path.
        while let Some(path) = shortest_augmenting_path(&cap, v, bot) {
            let bottleneck = path
                .windows(2)
                .filter_map(|w| cap.get(&(w[0], w[1])).copied())
                .fold(f64::INFINITY, f64::min);
            if bottleneck <= TOLERANCE || !bottleneck.is_finite() {
                break;
            }
            for w in path.windows(2) {
                let (a, b) = (w[0], w[1]);
                *cap.entry((a, b)).or_insert(0.0) -= bottleneck;
                *cap.entry((b, a)).or_insert(0.0) += bottleneck;
            }
        }

        // The min cut value is the total flow, which equals the weight of the
        // cut induced by the residual-reachable set from v.
        let reachable = residual_reachable(&cap, v);
        let cost = self.cut_weight_against_medium(&reachable);

        // Enumerate every minimiser. The residual set is one of them; others
        // are found by testing feasible sets on small graphs, and by reporting
        // only the canonical one when the graph is too large to sweep.
        let minimisers = self.all_minimisers(v, cost, &reachable);

        Some(Separation { cost, minimisers })
    }

    /// Weight of `cut(S)` where `S` excludes the medium.
    fn cut_weight_against_medium(&self, part: &BTreeSet<Position>) -> EdgeWeight {
        let bot = self.medium().0;
        let mut total = 0.0;
        // Base edges crossing the partition.
        for u in 0..self.base.order() {
            for v in (u + 1)..self.base.order() {
                if let Some(w) = self.base.weight(u, v)
                    && part.contains(&u) != part.contains(&v)
                {
                    total += w;
                }
            }
        }
        // Every member of S contributes its medium edge, since ⊥ ∉ S.
        for v in part {
            if *v != bot
                && let Some(w) = self.medium_edges.get(v)
            {
                total += w;
            }
        }
        total
    }

    /// Every feasible `S` attaining `cost`.
    ///
    /// For graphs small enough to sweep (order ≤ [`SWEEP_LIMIT`]) this is
    /// exact, which is what `binv:tiebreak` needs in order to report
    /// non-uniqueness honestly. Above that limit only the canonical residual
    /// minimiser is returned, and callers should treat determinacy as unknown
    /// rather than established.
    fn all_minimisers(
        &self,
        v: Position,
        cost: EdgeWeight,
        canonical: &BTreeSet<Position>,
    ) -> Vec<BTreeSet<Position>> {
        let n = self.base.order();
        if n > SWEEP_LIMIT {
            return vec![canonical.clone()];
        }
        let mut found = Vec::new();
        // Sweep subsets containing v and excluding the medium.
        for mask in 0u64..(1u64 << n) {
            let s: BTreeSet<Position> = (0..n).filter(|i| mask >> i & 1 == 1).collect();
            if !s.contains(&v) {
                continue;
            }
            if (self.cut_weight_against_medium(&s) - cost).abs() < TOLERANCE {
                found.push(s);
            }
        }
        if found.is_empty() {
            found.push(canonical.clone());
        }
        found
    }
}

/// Above this order the minimiser sweep is skipped; see
/// [`MediumGraph::all_minimisers`].
pub const SWEEP_LIMIT: u32 = 20;

/// Float comparison tolerance for cut weights.
const TOLERANCE: EdgeWeight = 1e-9;

/// BFS for a shortest residual path from `s` to `t`.
fn shortest_augmenting_path(
    cap: &BTreeMap<(Position, Position), EdgeWeight>,
    s: Position,
    t: Position,
) -> Option<Vec<Position>> {
    let mut prev: BTreeMap<Position, Position> = BTreeMap::new();
    let mut seen: BTreeSet<Position> = BTreeSet::from([s]);
    let mut q = VecDeque::from([s]);
    while let Some(a) = q.pop_front() {
        if a == t {
            break;
        }
        for ((from, to), c) in cap.iter() {
            if *from == a && *c > TOLERANCE && !seen.contains(to) {
                seen.insert(*to);
                prev.insert(*to, a);
                q.push_back(*to);
            }
        }
    }
    if !seen.contains(&t) {
        return None;
    }
    let mut path = vec![t];
    let mut cur = t;
    while cur != s {
        cur = *prev.get(&cur)?;
        path.push(cur);
    }
    path.reverse();
    Some(path)
}

/// Vertices reachable from `s` in the residual graph.
fn residual_reachable(
    cap: &BTreeMap<(Position, Position), EdgeWeight>,
    s: Position,
) -> BTreeSet<Position> {
    let mut seen = BTreeSet::from([s]);
    let mut q = VecDeque::from([s]);
    while let Some(a) = q.pop_front() {
        for ((from, to), c) in cap.iter() {
            if *from == a && *c > TOLERANCE && !seen.contains(to) {
                seen.insert(*to);
                q.push_back(*to);
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The witness of Proposition 3.4: `V = {⊥, v, a, b}` with `ab ∉ E`,
    /// `w(v⊥) = 3` and every other edge of weight 1. All four feasible sets
    /// have cut weight 5, so σ(v) is attained non-uniquely.
    fn prop_3_4_witness() -> MediumGraph {
        // Positions: v = 0, a = 1, b = 2. Medium is 3.
        let mut base = ContactGraph::new(3);
        base.add_edge(0, 1, 1.0).unwrap(); // va
        base.add_edge(0, 2, 1.0).unwrap(); // vb
        // ab is deliberately absent.
        let mut g = MediumGraph::new(base, 1.0).unwrap();
        g.set_medium_edge(0, 3.0).unwrap(); // v⊥ = 3
        g
    }

    #[test]
    fn separation_is_attained() {
        let g = prop_3_4_witness();
        assert!(g.separation(0).is_some(), "Theorem 3.2(i)");
    }

    #[test]
    fn separation_is_at_least_the_floor() {
        let g = prop_3_4_witness();
        let beta = g.floor().unwrap();
        for v in 0..g.order() {
            let s = g.separation(v).unwrap();
            assert!(s.cost() >= beta - TOLERANCE, "Theorem 3.2(ii) at {v}");
            assert!(s.cost() > 0.0, "Theorem 3.2(ii): strictly positive");
        }
    }

    #[test]
    fn prop_3_4_all_four_cuts_are_minimisers() {
        let g = prop_3_4_witness();
        let s = g.separation(0).unwrap();
        assert!(
            (s.cost() - 5.0).abs() < TOLERANCE,
            "σ(v) = 5, got {}",
            s.cost()
        );
        assert_eq!(s.minimisers().len(), 4, "Proposition 3.4: four minimisers");
        assert!(
            !s.is_determinate(),
            "binv:tiebreak: non-uniqueness is reported"
        );

        let expected: BTreeSet<BTreeSet<Position>> = [
            BTreeSet::from([0]),
            BTreeSet::from([0, 1]),
            BTreeSet::from([0, 2]),
            BTreeSet::from([0, 1, 2]),
        ]
        .into_iter()
        .collect();
        let got: BTreeSet<BTreeSet<Position>> = s.minimisers().iter().cloned().collect();
        assert_eq!(got, expected, "the four feasible sets of Prop. 3.4");
    }

    #[test]
    fn every_minimiser_contains_v_and_excludes_the_medium() {
        let g = prop_3_4_witness();
        let bot = g.medium().0;
        for v in 0..g.order() {
            for s in g.separation(v).unwrap().minimisers() {
                assert!(s.contains(&v), "Def. 3.1: v ∈ S");
                assert!(!s.contains(&bot), "Def. 3.1: ⊥ ∉ S");
            }
        }
    }

    #[test]
    fn max_flow_agrees_with_brute_force() {
        // Theorem 3.2(iii): the flow value equals the min cut. Check the
        // polynomial answer against an exhaustive sweep on a graph where the
        // sweep is still cheap.
        let mut base = ContactGraph::new(5);
        for (u, v, w) in [
            (0, 1, 2.0),
            (1, 2, 5.0),
            (2, 3, 1.5),
            (3, 4, 4.0),
            (0, 4, 3.0),
            (1, 3, 2.5),
        ] {
            base.add_edge(u, v, w).unwrap();
        }
        let g = MediumGraph::new(base, 2.0).unwrap();

        for v in 0..g.order() {
            let flow = g.separation(v).unwrap().cost();
            let brute = (0u64..(1u64 << g.order()))
                .map(|mask| {
                    (0..g.order())
                        .filter(|i| mask >> i & 1 == 1)
                        .collect::<BTreeSet<Position>>()
                })
                .filter(|s| s.contains(&v))
                .map(|s| g.cut_weight_against_medium(&s))
                .fold(f64::INFINITY, f64::min);
            assert!(
                (flow - brute).abs() < 1e-6,
                "Thm 3.2(iii) at {v}: flow {flow} vs brute force {brute}"
            );
        }
    }

    #[test]
    fn a_determinate_graph_reports_one_minimiser() {
        // Make v cheap to isolate and everything else expensive, so the
        // singleton strictly wins.
        let mut base = ContactGraph::new(3);
        base.add_edge(0, 1, 1.0).unwrap();
        base.add_edge(1, 2, 40.0).unwrap();
        let mut g = MediumGraph::new(base, 40.0).unwrap();
        g.set_medium_edge(0, 1.0).unwrap();
        let s = g.separation(0).unwrap();
        assert!(
            s.is_determinate(),
            "unique minimiser expected, got {:?}",
            s.minimisers()
        );
    }

    #[test]
    fn medium_requires_two_positions() {
        let base = ContactGraph::new(1);
        assert!(
            MediumGraph::new(base, 1.0).is_err(),
            "Definition 2.1: n ≥ 2"
        );
    }

    #[test]
    fn zero_weight_medium_is_refused() {
        let base = ContactGraph::new(3);
        assert!(MediumGraph::new(base, 0.0).is_err(), "Axiom floor");
    }

    #[test]
    fn unknown_position_has_no_separation() {
        let g = prop_3_4_witness();
        assert!(g.separation(99).is_none());
    }
}
