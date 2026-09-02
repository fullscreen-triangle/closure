//! Identity: the conserved invariant and the monotone record.
//!
//! An individual is the pair `(chi, m)` of a conserved, relabelling-invariant
//! cut structure and an unrepealable monotone record (Definition 9.1).
//! Neither half alone suffices: a deterministic component is invariant-only
//! and cannot distinguish an individual from a faithful copy (Theorem 9.4),
//! while a generative component holds no structure and so carries neither
//! (Theorem 9.5).

use crate::graph::ContactGraph;
use crate::invariants::InvariantViolation;
use serde::{Deserialize, Serialize};

/// A counter that is incremented on every committed act and decremented by
/// nothing.
///
/// **There is no `decrement`, no `reset`, and no `set`.** Invariant 2 is
/// enforced by the absence of those methods rather than by a check that could
/// be forgotten. An "undo" is a compensating commit that raises the count
/// further (Theorem 5.9(ii)), which is what [`Record::undo`] does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Record(u64);

impl Record {
    /// A fresh record, at zero. An agent here is not yet an individual: it
    /// has an invariant but no history, so nothing distinguishes it from a
    /// faithful copy (Theorem 8.7).
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// The current count.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Deposit a committed act. Every act costs at least the act floor, so
    /// this always advances (Axiom 4).
    pub const fn commit(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    /// Undo is itself a committed act, so the record rises rather than falls.
    pub const fn undo(&mut self) {
        self.commit();
    }
}

/// Which phase an agent occupies. Construction and commitment cannot share an
/// instant (Axiom 5), which is a second, independent source of intermittent
/// responsiveness (Theorem 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Reshaping distinctions; emits no act, in any scene.
    Construction,
    /// Emitting acts; performs no structural update.
    #[default]
    Commitment,
}

/// An agent: a contact graph paired with a renderer.
///
/// The renderer is not stored here — it is supplied by the runtime, because
/// no theorem depends on any property of it beyond the fact that its
/// propagations deposit (Theorem 9.7). What this type owns is the half that
/// carries the identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Stable handle.
    pub id: String,
    /// The graph where propagations deposit; this is the agent's identity.
    pub graph: ContactGraph,
    /// The monotone record.
    record: Record,
    /// The current phase.
    phase: Phase,
    /// Cached character invariant, computed at construction.
    chi: Option<f64>,
}

impl Agent {
    /// Build an agent over `graph`.
    #[must_use]
    pub fn new(id: impl Into<String>, graph: ContactGraph) -> Self {
        let chi = graph.separation_cost();
        Self {
            id: id.into(),
            graph,
            record: Record::new(),
            phase: Phase::Commitment,
            chi,
        }
    }

    /// The conserved character invariant (Theorem 9.3). Fixed under every
    /// relabelling of parts and independent of which scene the agent is
    /// engaged from.
    #[must_use]
    pub fn chi(&self) -> Option<f64> {
        self.chi
    }

    /// The current record.
    #[must_use]
    pub fn record(&self) -> Record {
        self.record
    }

    /// The current phase.
    #[must_use]
    pub fn phase(&self) -> Phase {
        self.phase
    }

    /// Enter a phase.
    pub const fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }

    /// Perform a determination, depositing before it returns.
    ///
    /// This is where Invariants 3, 4 and 5 meet. The closure `f` computes a
    /// fresh answer against the *current* graph — there is no cache to
    /// consult, because a stored answer would be the determination of a prior
    /// state the agent cannot re-occupy (Proposition 9.9).
    ///
    /// # Errors
    /// [`InvariantViolation::PhaseExclusion`] if called outside a commitment
    /// phase; [`InvariantViolation::MissingDeposit`] if the record did not
    /// advance, which would place the agent in the position Theorem 9.7
    /// forbids.
    pub fn determine<T>(
        &mut self,
        f: impl FnOnce(&ContactGraph) -> T,
    ) -> Result<T, InvariantViolation> {
        if self.phase != Phase::Commitment {
            return Err(InvariantViolation::PhaseExclusion);
        }
        let before = self.record;
        let value = f(&self.graph);
        self.record.commit();
        if self.record <= before {
            return Err(InvariantViolation::MissingDeposit);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent() -> Agent {
        let mut g = ContactGraph::new(3);
        g.add_edge(0, 1, 2.0).unwrap();
        g.add_edge(1, 2, 3.0).unwrap();
        g.add_edge(0, 2, 2.0).unwrap();
        Agent::new("t", g)
    }

    #[test]
    fn undo_raises_the_record() {
        let mut a = agent();
        let before = a.record().get();
        a.record.undo();
        assert!(a.record().get() > before, "Theorem 5.9(ii)");
    }

    #[test]
    fn determination_deposits_before_returning() {
        let mut a = agent();
        let before = a.record();
        let _ = a.determine(|g| g.floor()).unwrap();
        assert!(a.record() > before, "Invariant 4");
    }

    #[test]
    fn acts_are_refused_during_construction() {
        let mut a = agent();
        a.set_phase(Phase::Construction);
        assert!(
            matches!(a.determine(|_| ()), Err(InvariantViolation::PhaseExclusion)),
            "Invariant 5"
        );
    }

    #[test]
    fn a_fresh_copy_is_not_the_original() {
        let mut a = agent();
        a.determine(|_| ()).unwrap();
        let copy = Agent::new("t", a.graph.clone());
        assert_eq!(a.chi(), copy.chi(), "same invariant");
        assert_ne!(
            a.record(),
            copy.record(),
            "different individual (Thm 5.9(iii))"
        );
    }
}
