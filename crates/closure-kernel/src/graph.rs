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
    /// cleverness. Callers needing scale should quotient first (Thm 3.6).
    #[must_use]
    pub fn separation_cost(&self) -> Option<EdgeWeight> {
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
