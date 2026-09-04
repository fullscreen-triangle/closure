//! Contact graphs and the floor (Definitions 2.1-2.2, Theorem 3.1).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A position in a contact graph.
pub type Position = u32;

/// The cost of telling two positions apart. Always strictly positive
/// (Axiom 3, costly individuation).
pub type EdgeWeight = f64;

/// A finite, connected, positively weighted simple graph.
///
/// Vertices are *positions*; an edge `uv` records that `u` and `v` are in
/// contact, and its weight is the cost of separating them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactGraph {
    order: u32,
    weights: BTreeMap<(Position, Position), EdgeWeight>,
}

impl ContactGraph {
    /// An empty graph on `order` positions.
    #[must_use]
    pub fn new(order: u32) -> Self {
        Self {
            order,
            weights: BTreeMap::new(),
        }
    }

    /// Number of positions.
    #[must_use]
    pub fn order(&self) -> u32 {
        self.order
    }

    /// Add a contact. Rejects self-loops and non-positive weights: a
    /// zero-cost distinction distinguishes nothing (Remark 2.5).
    ///
    /// # Errors
    /// If `u == v`, or if `w` is not strictly positive and finite.
    pub fn add_edge(&mut self, u: Position, v: Position, w: EdgeWeight) -> crate::Result<()> {
        if u == v {
            return Err(crate::Error::Graph("contact graphs are simple".into()));
        }
        if w <= 0.0 || !w.is_finite() {
            return Err(crate::Error::Graph(
                "Axiom 3: edge weights must be strictly positive and finite".into(),
            ));
        }
        let key = if u < v { (u, v) } else { (v, u) };
        self.weights.insert(key, w);
        Ok(())
    }

    /// Weight of the contact `{u, v}`, if it exists.
    ///
    /// The companion's separation algorithm needs adjacency, not just cut
    /// weights; this is the accessor it reads through.
    #[must_use]
    pub fn weight(&self, u: Position, v: Position) -> Option<EdgeWeight> {
        let key = if u < v { (u, v) } else { (v, u) };
        self.weights.get(&key).copied()
    }

    /// Every contact, as `(u, v, w)` with `u < v`.
    pub fn edges(&self) -> impl Iterator<Item = (Position, Position, EdgeWeight)> + '_ {
        self.weights.iter().map(|((u, v), w)| (*u, *v, *w))
    }

    /// The floor: the least edge weight (Axiom 3).
    #[must_use]
    pub fn floor(&self) -> Option<EdgeWeight> {
        self.weights.values().copied().reduce(f64::min)
    }

    /// Cost of the cut separating `part` from its complement (Def. 2.2).
    #[must_use]
    pub fn cut_weight(&self, part: &BTreeSet<Position>) -> EdgeWeight {
        self.weights
            .iter()
            .filter(|((u, v), _)| part.contains(u) != part.contains(v))
            .map(|(_, w)| *w)
            .sum()
    }

    /// Separation cost `Res(G)`: the minimum over nonempty proper parts.
    ///
    /// Exhaustive by design. The manuscript's first validation convention is
    /// that structures are built by brute force from the definitions, so that
    /// agreement between theorem and computation is not an artefact of shared
    /// cleverness.
    ///
    /// **Exponential in the order**, so it is the oracle and not the working
    /// routine. Anything on a hot path wants [`Self::separation_cost_fast`],
    /// which computes the same number in cubic time; the two are asserted
    /// equal in this module's tests, which is what keeps the convention
    /// honest rather than merely stated.
    #[must_use]
    pub fn separation_cost_exhaustive(&self) -> Option<EdgeWeight> {
        if self.order < 2 || self.weights.is_empty() {
            return None;
        }
        let n = self.order;
        let mut best = f64::INFINITY;
        // Iterate proper nonempty subsets via bitmask; fix position 0 on one
        // side to halve the work, since cut(A) == cut(complement of A).
        for mask in 1u64..(1u64 << (n - 1)) {
            let part: BTreeSet<Position> = (0..n).filter(|i| mask >> i & 1 == 1).collect();
            if part.is_empty() || part.len() as u32 == n {
                continue;
            }
            best = best.min(self.cut_weight(&part));
        }
        (best.is_finite()).then_some(best)
    }

    /// Separation cost, in cubic time.
    ///
    /// The same quantity as [`Self::separation_cost_exhaustive`] — the global
    /// minimum cut — computed by Stoer–Wagner rather than by enumerating
    /// subsets. Theorem 3.2(iii) is what licenses the substitution: the
    /// separation of a vertex is a max flow, hence the global minimum over
    /// vertices is a minimum cut, and a minimum cut is strongly polynomial.
    /// Nothing about the *definition* changes; only the route to the number.
    ///
    /// This matters because the character invariant is computed on every
    /// pruning, and an agent cut from a city of thirty positions would
    /// otherwise cost a billion subset evaluations to construct.
    #[must_use]
    pub fn separation_cost_fast(&self) -> Option<EdgeWeight> {
        let n = self.order as usize;
        if n < 2 || self.weights.is_empty() {
            return None;
        }
        // Dense adjacency over merged vertices. Stoer–Wagner contracts the
        // last two vertices of each maximum-adjacency ordering and keeps the
        // best cut-of-the-phase seen.
        let mut w = vec![vec![0.0f64; n]; n];
        for ((u, v), x) in &self.weights {
            let (a, b) = (*u as usize, *v as usize);
            w[a][b] += *x;
            w[b][a] += *x;
        }
        let mut alive: Vec<usize> = (0..n).collect();
        let mut best = f64::INFINITY;

        while alive.len() > 1 {
            let mut added = vec![false; alive.len()];
            let mut weight = vec![0.0f64; alive.len()];
            // The maximum-adjacency order, built one vertex at a time. Only
            // its final two entries are needed: the last is the cut of the
            // phase, and it contracts into the one before it.
            let mut order: Vec<usize> = Vec::with_capacity(alive.len());
            for i in 0..alive.len() {
                // Take the unadded vertex most tightly attached so far.
                let mut sel = usize::MAX;
                for j in 0..alive.len() {
                    if !added[j] && (sel == usize::MAX || weight[j] > weight[sel]) {
                        sel = j;
                    }
                }
                added[sel] = true;
                order.push(sel);
                if i + 1 == alive.len() {
                    // The cut-of-the-phase: `last` alone against the rest.
                    best = best.min(weight[sel]);
                    let last = sel;
                    let Some(prev) = order.len().checked_sub(2).map(|k| order[k]) else {
                        break;
                    };
                    // Contract `last` into `prev`. Skip both endpoints: a
                    // merged vertex has no edge to itself, and folding one in
                    // would invent contact where the graph has none.
                    let (p, l) = (alive[prev], alive[last]);
                    for (j, &b) in alive.iter().enumerate() {
                        if j == prev || j == last {
                            continue;
                        }
                        w[p][b] += w[l][b];
                        w[b][p] = w[p][b];
                    }
                    alive.remove(last);
                    break;
                }
                for j in 0..alive.len() {
                    if !added[j] {
                        weight[j] += w[alive[sel]][alive[j]];
                    }
                }
            }
        }
        best.is_finite().then_some(best)
    }

    /// Separation cost `Res(G)` (Def. 2.3).
    ///
    /// Delegates to [`Self::separation_cost_fast`]. The exhaustive routine
    /// remains available as [`Self::separation_cost_exhaustive`] and is the
    /// oracle the fast one is checked against.
    #[must_use]
    pub fn separation_cost(&self) -> Option<EdgeWeight> {
        self.separation_cost_fast()
    }

    /// Whether every position is reachable from position 0 (Axiom 2).
    #[must_use]
    pub fn is_connected(&self) -> bool {
        if self.order == 0 {
            return false;
        }
        let mut seen = BTreeSet::from([0u32]);
        let mut stack = vec![0u32];
        while let Some(x) = stack.pop() {
            for (u, v) in self.weights.keys() {
                let nbr = if *u == x {
                    Some(*v)
                } else if *v == x {
                    Some(*u)
                } else {
                    None
                };
                if let Some(n) = nbr {
                    if seen.insert(n) {
                        stack.push(n);
                    }
                }
            }
        }
        seen.len() as u32 == self.order
    }

    /// Apply a relabelling of positions. The character invariant must be
    /// unchanged by this (Invariant 1); `tests` checks that it is.
    #[must_use]
    pub fn relabel(&self, perm: &[Position]) -> Self {
        let mut g = Self::new(self.order);
        for ((u, v), w) in &self.weights {
            let (a, b) = (perm[*u as usize], perm[*v as usize]);
            let key = if a < b { (a, b) } else { (b, a) };
            g.weights.insert(key, *w);
        }
        g
    }
}

#[cfg(test)]
mod tests {

    /// The oracle and the working routine must agree, on every graph we can
    /// afford to ask both. This is the whole justification for delegating
    /// `separation_cost` to the cubic one: the brute-force convention is
    /// preserved as a *check* rather than abandoned for speed.
    #[test]
    fn the_fast_separation_cost_agrees_with_brute_force() {
        // Deterministic pseudo-random graphs, small enough to enumerate.
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for order in 2u32..9 {
            for _ in 0..12 {
                let mut g = ContactGraph::new(order);
                // A spanning path keeps it connected (Axiom 2), then extra
                // contacts at random.
                for u in 0..order - 1 {
                    let w = 1.0 + f64::from((next() % 5) as u32);
                    g.add_edge(u, u + 1, w).unwrap();
                }
                for _ in 0..order {
                    let u = (next() % u64::from(order)) as u32;
                    let v = (next() % u64::from(order)) as u32;
                    if u != v {
                        let w = 1.0 + f64::from((next() % 5) as u32);
                        g.add_edge(u, v, w).unwrap();
                    }
                }
                let slow = g.separation_cost_exhaustive().unwrap();
                let fast = g.separation_cost_fast().unwrap();
                assert!(
                    (slow - fast).abs() < 1e-9,
                    "order {order}: brute force {slow}, Stoer-Wagner {fast}"
                );
            }
        }
    }

    #[test]
    fn separation_cost_is_computable_on_a_city_sized_graph() {
        // The case that motivated the fast routine: a graph of thirty
        // positions is a billion subsets for the oracle and instant here.
        let mut g = ContactGraph::new(31);
        for u in 0..30 {
            g.add_edge(u, u + 1, if u % 6 == 0 { 1.0 } else { 2.0 })
                .unwrap();
        }
        assert_eq!(g.separation_cost(), Some(1.0), "the thinnest join");
    }
    use super::*;

    /// The two-triangle witness of Theorem 3.3: the minimum cut is attained
    /// by a three-position region and by no singleton.
    fn two_triangles(bridge: f64) -> ContactGraph {
        let mut g = ContactGraph::new(6);
        for (u, v) in [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)] {
            g.add_edge(u, v, 10.0).unwrap();
        }
        g.add_edge(2, 3, bridge).unwrap();
        g
    }

    #[test]
    fn floor_bounds_separation() {
        let g = two_triangles(1.0);
        let beta = g.floor().unwrap();
        assert!(g.separation_cost().unwrap() >= beta, "Theorem 3.1");
    }

    #[test]
    fn individuation_is_regional() {
        let g = two_triangles(1.0);
        let res = g.separation_cost().unwrap();
        let best_singleton = (0..6)
            .map(|v| g.cut_weight(&BTreeSet::from([v])))
            .fold(f64::INFINITY, f64::min);
        assert!(best_singleton > res, "Theorem 3.3: no singleton attains it");
    }

    #[test]
    fn zero_weight_is_refused() {
        let mut g = ContactGraph::new(2);
        assert!(g.add_edge(0, 1, 0.0).is_err(), "Axiom 3");
    }

    #[test]
    fn separation_cost_is_relabelling_invariant() {
        let g = two_triangles(1.0);
        let h = g.relabel(&[3, 4, 5, 0, 1, 2]);
        let (a, b) = (g.separation_cost().unwrap(), h.separation_cost().unwrap());
        assert!((a - b).abs() < 1e-12, "Invariant 1");
    }
}
