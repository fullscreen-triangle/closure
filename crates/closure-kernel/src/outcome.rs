//! Outcomes and the three routes to non-commitment.
//!
//! `Outcome` has exactly two shapes (Theorem 10.11). Note that
//! [`Outcome::Declined`] is a *value*, not an error: an agent that has
//! considered a proposal and holds two incompatible readings of it has
//! terminated normally (Corollary 10.12).

use crate::closure::Region;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// The result of a completed search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// The search closed on a single region.
    Resolved { cell: Region },

    /// The search closed on a plurality of mutually incompatible regions.
    ///
    /// This is a successful termination. Nothing in the system resolves it
    /// further, and no move forces it to (Remark 10.13).
    Declined { cells: BTreeSet<Region> },
}

impl Outcome {
    /// Build an outcome from a reach set. Never fails: a singleton reach
    /// resolves, anything larger declines, and the two are jointly exhaustive
    /// once the reach is nonempty (Theorem 10.11).
    #[must_use]
    pub fn from_reach(reach: BTreeSet<Region>) -> Option<Self> {
        match reach.len() {
            0 => None,
            1 => reach.into_iter().next().map(|cell| Self::Resolved { cell }),
            _ => Some(Self::Declined { cells: reach }),
        }
    }

    /// Whether this outcome declined. Provided for display only — no control
    /// flow in this workspace branches on it as success or failure.
    #[must_use]
    pub fn is_declined(&self) -> bool {
        matches!(self, Self::Declined { .. })
    }
}

/// Why a commitment did not occur (Proposition 4.14).
///
/// The three are structurally distinct and are distinguished by different
/// tests, which is why they present differently from the inside: (i) is
/// ignorance, (ii) is inertia, (iii) is confusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    /// The agent committed.
    Committed,
    /// (i) No invoked consideration reaches the target.
    Ignorance,
    /// (ii) Some available consideration reaches elsewhere, so closure fails.
    Inertia,
    /// (iii) The target is not a sufficient region, so it is not a candidate.
    Confusion,
}

/// Classify an attempt into exactly one route.
#[must_use]
pub fn classify(
    seed: crate::Position,
    target: &Region,
    invoked: &[crate::Consideration],
    available: &[crate::Consideration],
    sufficient: &BTreeSet<Region>,
) -> Route {
    if !sufficient.contains(target) {
        return Route::Confusion;
    }
    if !crate::reach(seed, invoked.iter()).contains(target) {
        return Route::Ignorance;
    }
    if !crate::is_closed(seed, invoked, available) {
        return Route::Inertia;
    }
    Route::Committed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Consideration;

    fn region(xs: &[crate::Position]) -> Region {
        xs.iter().copied().collect()
    }

    #[test]
    fn all_three_routes_are_reachable() {
        let (a, b) = (region(&[0, 1]), region(&[2, 3]));
        let unreachable = region(&[9]);
        let sufficient = BTreeSet::from([a.clone(), b.clone()]);
        let ca = Consideration::new("a", 0, a.clone());
        let cb = Consideration::new("b", 0, b.clone());

        let just_a = [ca.clone()];
        let a_and_b = [ca, cb];

        assert_eq!(
            classify(0, &a, &just_a, &just_a, &sufficient),
            Route::Committed
        );
        assert_eq!(
            classify(0, &b, &just_a, &just_a, &sufficient),
            Route::Ignorance
        );
        assert_eq!(
            classify(0, &a, &just_a, &a_and_b, &sufficient),
            Route::Inertia
        );
        assert_eq!(
            classify(0, &unreachable, &just_a, &just_a, &sufficient),
            Route::Confusion
        );
    }

    #[test]
    fn declination_carries_at_least_two_regions() {
        let r = BTreeSet::from([region(&[0]), region(&[1])]);
        match Outcome::from_reach(r).unwrap() {
            Outcome::Declined { cells } => assert!(cells.len() >= 2),
            Outcome::Resolved { .. } => panic!("expected a declination"),
        }
    }
}
