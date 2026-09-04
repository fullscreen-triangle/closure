//! Acts: the residual gap and the two directions (Psychon §7).
//!
//! An exchange between two agents does not close (Theorem 7.4): whatever is
//! emitted, some gap remains on at least one side. What *is* decidable is the
//! **direction** the gap moved, and Theorem 7.5 says there are exactly two,
//! they are independent, and both are computable from emitted states alone.
//!
//! ```text
//! report    iff  D-hat  <  D      (the gap fell on the acting side)
//! question  iff  D-hat' >  D'     (the gap rose on the receiving side)
//! ```
//!
//! ## Why there is no content parameter
//!
//! [`classify`] takes four numbers and nothing else. It does not see what was
//! said, who said it, or what it was about. That is not an economy — it is the
//! claim of Theorem 7.5(iii), that the classification is a function of the
//! emitted states, and adding a content argument would quietly make it a
//! function of something else.
//!
//! The same reasoning rules out an author: Theorem 6.6 shows no function of
//! emitted states selects an originator, so nothing here takes one.
//!
//! ## Why this is a struct and not an enum
//!
//! Theorem 7.5(ii) proves the two conditions independent. An act can be a
//! report and a question at once, or neither; a four-variant enum would encode
//! that too, but it would also invite a `match` that treats the two as
//! exclusive. [`Act`] carries two booleans, so the independence survives
//! contact with a caller.
//!
//! ## The gap
//!
//! Definition 7.2 gives the residual gap as the shortfall of an agent's
//! resolution against the cost of the resting cut at its terminus:
//!
//! ```text
//! D(A) = w(cut(S*(v))) - rho(A)
//! ```
//!
//! [`residual_gap`] computes it. When the graph does not determine the resting
//! cut (Prop. 3.4) every minimiser has the same weight by definition, so the
//! gap is well defined even though the cut is not — which is exactly why
//! [`MediumGraph::separation`] exposes the cost separately from the sets.

use closure_kernel::graph::Position;
use closure_kernel::psychon::Emitted;
use closure_kernel::separation::MediumGraph;
use serde::{Deserialize, Serialize};

/// The residual gap `D(A)` of Definition 7.2.
///
/// `resolution` is `rho(A)`, what the agent can currently tell apart at its
/// terminus. The gap is the shortfall of that against the cost of separating
/// the terminus from the medium. It may be negative, which says the agent
/// resolves more finely than the boundary requires; nothing forbids that, and
/// the direction conditions of Theorem 7.5 are stated as inequalities, not as
/// sign tests.
///
/// Returns `None` if `terminus` is not a position of `graph`.
#[must_use]
pub fn residual_gap(graph: &MediumGraph, terminus: Position, resolution: f64) -> Option<f64> {
    Some(graph.separation(terminus)?.cost() - resolution)
}

/// The gap at a psychon's emitted terminus.
///
/// A convenience over [`residual_gap`] that makes the input explicit: an
/// [`Emitted`] state and nothing more.
#[must_use]
pub fn gap_at(graph: &MediumGraph, emitted: &Emitted, resolution: f64) -> Option<f64> {
    residual_gap(graph, emitted.terminus, resolution)
}

/// What an act did to the gaps on either side (Theorem 7.5).
///
/// Both fields can be true, both can be false. See the module documentation
/// for why this is not an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Act {
    /// The gap fell on the acting side.
    pub report: bool,
    /// The gap rose on the receiving side.
    pub question: bool,
}

impl Act {
    /// Whether the act moved neither gap.
    ///
    /// This is a real outcome, not a failure: Theorem 7.4 says exchange does
    /// not close, so an act that shifts nothing is permitted and must be
    /// representable. It is not an error and carries no verdict.
    #[must_use]
    pub const fn is_inert(self) -> bool {
        !self.report && !self.question
    }

    /// Whether the act was both at once.
    #[must_use]
    pub const fn is_both(self) -> bool {
        self.report && self.question
    }
}

/// The gaps on one side of an exchange, before and after.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Gaps {
    /// The gap before the act.
    pub before: f64,
    /// The gap after it.
    pub after: f64,
}

impl Gaps {
    /// The pair, in order.
    #[must_use]
    pub const fn new(before: f64, after: f64) -> Self {
        Self { before, after }
    }

    /// A side the act did not move.
    #[must_use]
    pub const fn unchanged(at: f64) -> Self {
        Self {
            before: at,
            after: at,
        }
    }
}

/// Classify an act from the gaps on both sides (Theorem 7.5).
///
/// Note the signature: four numbers, no content, no author, no identifiers.
/// That is the theorem's content, not an omission — see the module docs.
///
/// The two conditions are checked independently and are not exclusive.
#[must_use]
pub fn classify(acting: Gaps, receiving: Gaps) -> Act {
    Act {
        report: acting.after < acting.before,
        question: receiving.after > receiving.before,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use closure_kernel::graph::ContactGraph;

    fn gaps(before: f64, after: f64) -> Gaps {
        Gaps::new(before, after)
    }

    #[test]
    fn thm_7_5_report_is_a_fall_on_the_acting_side() {
        let a = classify(gaps(5.0, 3.0), Gaps::unchanged(2.0));
        assert!(a.report, "the acting gap fell");
        assert!(!a.question, "the receiving gap did not rise");
    }

    #[test]
    fn thm_7_5_question_is_a_rise_on_the_receiving_side() {
        let a = classify(Gaps::unchanged(5.0), gaps(2.0, 4.0));
        assert!(!a.report);
        assert!(a.question, "the receiving gap rose");
    }

    #[test]
    fn thm_7_5_ii_the_two_directions_are_independent() {
        // All four combinations are reachable, which is what independence
        // means. An enum with exclusive variants could not express row four.
        assert_eq!(
            classify(Gaps::unchanged(5.0), Gaps::unchanged(2.0)),
            Act {
                report: false,
                question: false
            }
        );
        assert_eq!(
            classify(gaps(5.0, 3.0), Gaps::unchanged(2.0)),
            Act {
                report: true,
                question: false
            }
        );
        assert_eq!(
            classify(Gaps::unchanged(5.0), gaps(2.0, 4.0)),
            Act {
                report: false,
                question: true
            }
        );
        assert_eq!(
            classify(gaps(5.0, 3.0), gaps(2.0, 4.0)),
            Act {
                report: true,
                question: true
            },
            "Theorem 7.5(ii): an act can be both at once"
        );
    }

    #[test]
    fn an_inert_act_is_a_value_not_an_error() {
        let a = classify(Gaps::unchanged(5.0), Gaps::unchanged(2.0));
        assert!(a.is_inert());
        assert!(!a.is_both());
    }

    #[test]
    fn classification_ignores_everything_but_the_gaps() {
        // Two exchanges with the same gap movements classify identically, no
        // matter what else differs about them — there is nothing else in the
        // signature to differ.
        let a = classify(gaps(100.0, 99.0), gaps(0.5, 0.75));
        let b = classify(gaps(3.0, 2.0), gaps(1.0, 1.5));
        assert_eq!(a, b, "Theorem 7.5(iii): a function of emitted states alone");
    }

    #[test]
    fn the_gap_is_the_cut_cost_less_the_resolution() {
        let mut base = ContactGraph::new(3);
        base.add_edge(0, 1, 1.0).unwrap();
        base.add_edge(0, 2, 1.0).unwrap();
        let g = MediumGraph::new(base, 1.0).unwrap();
        let sigma = g.separation(0).unwrap().cost();
        assert_eq!(residual_gap(&g, 0, 0.0), Some(sigma));
        assert_eq!(residual_gap(&g, 0, sigma), Some(0.0));
        let over = residual_gap(&g, 0, sigma + 1.0).unwrap();
        assert!(over < 0.0, "a finer resolution than required is permitted");
    }

    #[test]
    fn the_gap_reads_an_emitted_state_and_nothing_else() {
        use closure_kernel::psychon::Psychon;
        let mut base = ContactGraph::new(3);
        base.add_edge(0, 1, 2.0).unwrap();
        base.add_edge(1, 2, 2.0).unwrap();
        let g = MediumGraph::new(base, 1.0).unwrap();

        // Two psychons that took different routes to the same terminus.
        let mut p = Psychon::new(0);
        p.step(1);
        let mut q = Psychon::new(2);
        q.step(1);

        assert_eq!(p.emit().terminus, q.emit().terminus);
        assert_eq!(
            gap_at(&g, &p.emit(), 0.5),
            gap_at(&g, &q.emit(), 0.5),
            "Theorem 5.4: the route is not an input to the gap"
        );
    }

    #[test]
    fn an_unknown_terminus_has_no_gap() {
        let mut base = ContactGraph::new(2);
        base.add_edge(0, 1, 1.0).unwrap();
        let g = MediumGraph::new(base, 1.0).unwrap();
        assert!(residual_gap(&g, 99, 0.0).is_none());
    }
}
