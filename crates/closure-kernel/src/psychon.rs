//! The psychon: a walk together with where it ended (Psychon §4–§5).
//!
//! A psychon is the pair `Ψ = (γ, v)` — the route taken through the graph, and
//! the terminus it rests at — carrying the monotone record `m` it accumulated
//! along the way. Only one of those three crosses the agent boundary:
//!
//! ```text
//! π(Ψ) = (v, m)
//! ```
//!
//! ## The walk is private, and that is the whole point
//!
//! Theorem 5.4 (path opacity) exhibits two distinct walks
//!
//! ```text
//! γ₁ = (⊥, a, ⊥, b, ⊥)      γ₂ = (⊥, b, ⊥, a, ⊥)
//! ```
//!
//! which traverse the same edge multiset in a different order. For *any* cost
//! function on edges they accumulate the same record and rest at the same
//! terminus, so `π(γ₁) = π(γ₂)` while `γ₁ ≠ γ₂`. The emission map is not
//! injective, and no receiver — however attentive, however well resourced —
//! can recover the route from what it is given.
//!
//! This module therefore encodes that as an access property rather than a
//! convention:
//!
//! * [`Psychon::walk`] is private and has **no public accessor**;
//! * [`Psychon`] does **not** implement `Serialize`, so no route can leak
//!   through an API boundary by an author forgetting;
//! * [`Psychon`] does **not** implement `PartialEq`, because two psychons that
//!   an observer must treat as identical would compare unequal, which invites
//!   exactly the reasoning Theorem 5.4 forbids;
//! * [`Emitted`] is the serialisable type, and it is what routes return.
//!
//! Compare [`crate::identity::Record`], where invariant 2 is likewise
//! enforced by a missing method rather than a check.
//!
//! ## What may be compared
//!
//! [`Psychon::emit`] gives `(terminus, record)`, and [`Emitted`] is `PartialEq`.
//! Two psychons agree exactly when their emissions do — that is the only
//! equality the theory licenses, and it is deliberately the only one available.

use crate::graph::Position;
use crate::identity::Record;
use serde::{Deserialize, Serialize};

/// What crosses the boundary: `π(Ψ) = (v, m)`.
///
/// This is the *only* representation of a psychon that may be serialised,
/// logged, compared, or sent to a receiver. It is what §6 means by an emitted
/// state, and every result from Theorem 6.2 onward is stated about these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Emitted {
    /// The terminus `v` the walk rests at.
    pub terminus: Position,
    /// The monotone record `m` accumulated along the walk.
    pub record: Record,
}

/// A psychon `Ψ = (γ, v)` with its record.
///
/// Deliberately not `Serialize`, not `PartialEq`, and holding no accessor for
/// its walk. See the module documentation: these three absences are Theorem
/// 5.4 expressed as a type rather than as a comment.
#[derive(Debug, Clone)]
pub struct Psychon {
    /// The route `γ`. Private, and it stays private.
    walk: Vec<Position>,
    record: Record,
}

impl Psychon {
    /// Begin a psychon resting at `origin`, with a fresh record.
    #[must_use]
    pub fn new(origin: Position) -> Self {
        Self {
            walk: vec![origin],
            record: Record::new(),
        }
    }

    /// Begin from an existing record, for a psychon resumed mid-history.
    #[must_use]
    pub fn resumed(origin: Position, record: Record) -> Self {
        Self {
            walk: vec![origin],
            record,
        }
    }

    /// Step to `next`, depositing on the record.
    ///
    /// Every propagation deposits (Axiom `actfloor`), so the record advances
    /// on each step and never on nothing.
    pub fn step(&mut self, next: Position) {
        self.walk.push(next);
        self.record.commit();
    }

    /// The terminus `v`: where the walk currently rests.
    #[must_use]
    pub fn terminus(&self) -> Position {
        *self
            .walk
            .last()
            .expect("a psychon always has at least its origin")
    }

    /// The record `m`.
    #[must_use]
    pub fn record(&self) -> Record {
        self.record
    }

    /// Number of steps taken. This is a count, not a route: it is already
    /// recoverable from the record, so exposing it discloses nothing the
    /// emission does not.
    #[must_use]
    pub fn steps(&self) -> usize {
        self.walk.len() - 1
    }

    /// `π(Ψ) = (v, m)` — everything a receiver may have.
    #[must_use]
    pub fn emit(&self) -> Emitted {
        Emitted {
            terminus: self.terminus(),
            record: self.record,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the two walks of Theorem 5.4 over the medium `⊥ = 0`, with
    /// `a = 1` and `b = 2`.
    fn opacity_pair() -> (Psychon, Psychon) {
        let (bot, a, b) = (0, 1, 2);
        let mut g1 = Psychon::new(bot);
        for v in [a, bot, b, bot] {
            g1.step(v);
        }
        let mut g2 = Psychon::new(bot);
        for v in [b, bot, a, bot] {
            g2.step(v);
        }
        (g1, g2)
    }

    #[test]
    fn thm_5_4_distinct_walks_emit_identically() {
        let (g1, g2) = opacity_pair();
        assert_ne!(g1.walk, g2.walk, "the walks differ, by construction");
        assert_eq!(
            g1.emit(),
            g2.emit(),
            "Theorem 5.4: pi is not injective — same terminus, same record"
        );
    }

    #[test]
    fn thm_5_4_holds_for_any_cost_function() {
        // The theorem is stated for *any* cost on edges, so the equality must
        // not depend on the particular deposit. Both walks traverse the same
        // edge multiset {⊥a, a⊥, ⊥b, b⊥}, so any per-edge cost sums equally.
        let (g1, g2) = opacity_pair();
        let cost = |u: Position, v: Position| -> f64 {
            let (x, y) = if u < v { (u, v) } else { (v, u) };
            f64::from(x) * 7.0 + f64::from(y) * 13.0 + 1.5
        };
        let total = |p: &Psychon| -> f64 { p.walk.windows(2).map(|w| cost(w[0], w[1])).sum() };
        assert!(
            (total(&g1) - total(&g2)).abs() < 1e-12,
            "same edge multiset, so equal under any cost function"
        );
    }

    #[test]
    fn the_record_advances_on_every_step() {
        let mut p = Psychon::new(0);
        assert_eq!(p.record().get(), 0);
        p.step(1);
        p.step(0);
        assert_eq!(p.record().get(), 2, "Axiom actfloor: every step deposits");
        assert_eq!(p.steps(), 2);
    }

    #[test]
    fn a_returning_walk_does_not_return_the_record() {
        // Coming back to where you started is not coming back to when you
        // started (Invariant 2 / binv:record).
        let mut p = Psychon::new(4);
        p.step(9);
        p.step(4);
        assert_eq!(p.terminus(), 4, "same place");
        assert_eq!(p.record().get(), 2, "later time");
    }

    #[test]
    fn emission_carries_the_terminus_and_the_record_only() {
        let mut p = Psychon::new(3);
        p.step(8);
        let e = p.emit();
        assert_eq!(e.terminus, 8);
        assert_eq!(e.record, p.record());
        // Serialising the emission must not smuggle the route.
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains('3'), "the origin must not appear: {json}");
    }
}
