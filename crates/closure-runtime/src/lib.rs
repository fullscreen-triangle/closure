//! # closure-runtime
//!
//! The semantically inert node runtime (Section 11).
//!
//! The runtime's whole responsibility is to execute nodes. Its vocabulary is
//! exactly four operations — identify, read, transform, emit — and there is
//! no fifth. In particular there is no operation comparing a value to an
//! expectation, and the graph stores no expectation, which is why
//! [`Runtime`] cannot compute an exit code (Theorem 11.5).
//!
//! Two consequences shape the API:
//!
//! * [`Runtime::execute`] runs *every* chunk on a node and emits every
//!   result. An error is an ordinary emitted value, so a run does not halt on
//!   anomaly (Corollary 11.6) and no emission suppresses another
//!   (Corollary 11.7).
//! * The node set is durable and can be fingerprinted; the trajectory is
//!   constituted by the run and cannot be scheduled in advance
//!   (Theorem 11.13). Reproducibility therefore attaches to the protocol,
//!   not to the readings (Corollary 11.14).

#![doc(html_root_url = "https://docs.rs/closure-runtime/0.1.0")]

pub mod act;
pub mod attention;
pub mod node;
pub mod population;

pub use act::{Act, Gaps, classify, gap_at, residual_gap};
pub use attention::{Allocation, Scene, water_fill};
pub use node::{Node, Runtime, Value};
pub use population::{Module, Population};

/// Errors the runtime can return.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An invariant of the kernel would have been broken.
    #[error(transparent)]
    Kernel(#[from] closure_kernel::Error),

    /// An invariant violation surfaced directly.
    #[error(transparent)]
    Invariant(#[from] closure_kernel::InvariantViolation),

    /// A scene was declared with a non-positive richness.
    #[error("scene {0:?} has non-positive richness")]
    Scene(String),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;
