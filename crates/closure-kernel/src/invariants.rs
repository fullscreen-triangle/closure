//! The six implementation invariants (Section 12).
//!
//! Each is a design commitment, honourable by construction. The table below
//! is the checklist a reviewer reads; the enforcement lives in the types.

use serde::{Deserialize, Serialize};

/// A violation of an implementation invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InvariantViolation {
    /// Invariant 1: the character invariant changed under a relabelling.
    #[error("Invariant 1: character invariant is not relabelling-conserved")]
    InvariantNotConserved,

    /// Invariant 2: something tried to lower the record.
    #[error("Invariant 2: the committed record may not be decremented")]
    RecordDecrement,

    /// Invariant 3: an answer was returned from a cache.
    #[error("Invariant 3: determinations are computed, never looked up")]
    StaleLookup,

    /// Invariant 4: a determination returned without depositing.
    #[error("Invariant 4: determination returned without advancing the record")]
    MissingDeposit,

    /// Invariant 5: an act was emitted during a construction phase.
    #[error("Invariant 5: no act may be emitted during a construction phase")]
    PhaseExclusion,

    /// Invariant 6: a verdict or attribution was requested.
    #[error("Invariant 6: the system reports determinations and declines, never verdicts")]
    VerdictRequested,
}

/// A single invariant, for documentation and for the `closure doctor` report.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Invariant {
    /// 1-based index, matching Section 12 of the manuscript.
    pub index: u8,
    /// Short name.
    pub name: &'static str,
    /// The predicate a runtime must satisfy at all times.
    pub predicate: &'static str,
    /// The theorem that makes the predicate the right one.
    pub certified_by: &'static str,
}

/// The six invariants, in the order they appear in Section 12.
pub const INVARIANTS: [Invariant; 6] = [
    Invariant {
        index: 1,
        name: "conserved invariant",
        predicate: "under any relabelling of positions, chi is unchanged",
        certified_by: "Theorem 9.3",
    },
    Invariant {
        index: 2,
        name: "never-resetting record",
        predicate: "no code path decreases the counter; undo is a compensating commit",
        certified_by: "Theorem 5.9",
    },
    Invariant {
        index: 3,
        name: "determination by search, not lookup",
        predicate: "no answer is returned from a cache of prior determinations",
        certified_by: "Proposition 9.9, Corollary 9.10",
    },
    Invariant {
        index: 4,
        name: "deposit on every propagation",
        predicate: "the record after a determination strictly exceeds the record before",
        certified_by: "Theorem 9.7",
    },
    Invariant {
        index: 5,
        name: "exclusive phases",
        predicate: "structural update and act emission occupy disjoint instants",
        certified_by: "Theorem 5.6",
    },
    Invariant {
        index: 6,
        name: "no system verdict",
        predicate: "no comparison of an achieved determination to an expected one, \
                    and no attribution of an outcome to a consideration",
        certified_by: "Theorem 11.5, Corollary 12.11",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invariants_are_indexed_in_order() {
        for (i, inv) in INVARIANTS.iter().enumerate() {
            assert_eq!(inv.index as usize, i + 1);
        }
    }
}
