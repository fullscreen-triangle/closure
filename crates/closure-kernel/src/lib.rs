//! # closure-kernel
//!
//! Contact graphs, closure, and the six implementation invariants.
//!
//! This crate is the load-bearing half of the system. Every type here
//! corresponds to a numbered definition in the manuscript
//! (`zuerich/docs/zuerich-common-closure`), and the constraints it enforces
//! are the invariants of its Section 12.
//!
//! Three of those invariants are enforced *by construction* here, which is
//! why they are types rather than conventions:
//!
//! * [`Record`] cannot be decremented — there is no such method.
//! * [`Agent::determine`] refuses to run outside a commitment phase and
//!   returns an error if the determination failed to deposit.
//! * No item in this crate compares an achieved state to an expected one.
//!   That absence is the point: an exit code is not computable from a state
//!   space that stores no expectation.
//!
//! What follows are the pieces the runtime and the server build on.

#![doc(html_root_url = "https://docs.rs/closure-kernel/0.1.0")]

pub mod closure;
pub mod graph;
pub mod identity;
pub mod invariants;
pub mod outcome;
pub mod token;

pub use closure::{Consideration, Reach, is_closed, reach};
pub use graph::{ContactGraph, EdgeWeight, Position};
pub use identity::{Agent, Phase, Record};
pub use invariants::{INVARIANTS, InvariantViolation};
pub use outcome::{Outcome, Route};
pub use token::SessionToken;

/// Errors this crate can return.
///
/// Note what is absent: there is no `Failure` or `Rejected` variant that
/// reports an outcome as unsuccessful. Declination is not an error — it is
/// [`Outcome::Declined`], a value (Cor. 10.12).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An implementation invariant would have been broken.
    #[error("invariant violated: {0}")]
    Invariant(#[from] InvariantViolation),

    /// A graph was given that does not satisfy the axioms.
    #[error("malformed contact graph: {0}")]
    Graph(String),

    /// A session token was not well formed.
    #[error("malformed token: {0}")]
    Token(String),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;
