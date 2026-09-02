//! Nodes, the four operations, and the induced trajectory (Section 11).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A value carried on a node. Opaque to the runtime, which stores and
/// transports values and never inspects their content — the fact that makes
/// Theorem 11.5 go through.
pub type Value = serde_json::Value;

/// A unit of work: a subtask identity fused with the code that realises it
/// and the values that have accreted there (Definition 11.1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Node {
    /// The subtask identity. Nodes are individuated by this alone, which is
    /// what makes convergence a merge rather than a collision.
    pub theta: String,
    /// Values emitted here, in order. Grows monotonically; `emit` adjoins and
    /// never replaces.
    pub values: Vec<Value>,
    /// How many chunks this node carries. A subtask may be realised more than
    /// one way, and the runtime executes all of them rather than selecting.
    pub chunk_count: usize,
}

/// The semantically inert runtime.
#[derive(Debug, Default)]
pub struct Runtime {
    nodes: BTreeMap<String, Node>,
    edges: BTreeSet<(String, String)>,
    record: u64,
}

impl Runtime {
    /// A runtime with no nodes.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // ---- the four operations; there is no fifth ----------------------

    /// Obtain the node bearing `theta`, creating it if absent.
    pub fn identify(&mut self, theta: &str) -> &mut Node {
        self.nodes.entry(theta.to_owned()).or_insert_with(|| Node {
            theta: theta.to_owned(),
            ..Node::default()
        })
    }

    /// Read the values on a node. Returns them without examining them.
    #[must_use]
    pub fn read(&self, theta: &str) -> &[Value] {
        self.nodes.get(theta).map_or(&[], |n| n.values.as_slice())
    }

    /// Adjoin a value to a node.
    ///
    /// `cause` records the node whose value prompted this emission. That is
    /// what induces an edge — and the edge exists only once the reading has
    /// happened, which is why the relation is a *product* of a run rather
    /// than an input to it (Proposition 11.12).
    pub fn emit(&mut self, theta: &str, value: Value, cause: Option<&str>) {
        self.identify(theta).values.push(value);
        self.record = self.record.saturating_add(1);
        if let Some(c) = cause {
            if c != theta {
                self.edges.insert((c.to_owned(), theta.to_owned()));
            }
        }
    }

    // `transform` is deliberately absent from this type: it is internal to a
    // module and invisible to the runtime (Definition 11.2).

    // ---- convergence ---------------------------------------------------

    /// Merge a contribution onto the node bearing `theta`.
    ///
    /// Convergence is associative, commutative and idempotent, so the node
    /// set of a run does not depend on the order in which agents contribute
    /// (Theorem 11.9). Two agents that independently arrive at the same
    /// subtask land on one node with no merge protocol.
    pub fn converge(&mut self, theta: &str, chunks: usize, values: Vec<Value>) {
        let node = self.identify(theta);
        node.chunk_count += chunks;
        for v in values {
            if !node.values.contains(&v) {
                node.values.push(v);
            }
        }
    }

    // ---- execution -----------------------------------------------------

    /// Execute every chunk on a node and emit each result.
    ///
    /// A chunk that fails yields an error *value*, which is emitted like any
    /// other and does not stop the remaining chunks (Corollary 11.6). This
    /// has a real cost — the runtime will do work whose inputs a human reader
    /// already knows to be corrupt (Remark 11.8) — and it is the right trade
    /// for a research instrument, where discarding an informative anomaly is
    /// worse than wasting cycles.
    pub fn execute<F>(&mut self, theta: &str, chunks: Vec<F>) -> Vec<Value>
    where
        F: FnOnce() -> std::result::Result<Value, String>,
    {
        let mut out = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let v = match chunk() {
                Ok(v) => v,
                Err(e) => serde_json::json!({ "error": e }),
            };
            out.push(v.clone());
            self.emit(theta, v, None);
        }
        out
    }

    // ---- observation ----------------------------------------------------

    /// The nodes that carried propagated information (Definition 11.11).
    #[must_use]
    pub fn trajectory(&self) -> BTreeSet<&str> {
        self.edges
            .iter()
            .flat_map(|(a, b)| [a.as_str(), b.as_str()])
            .collect()
    }

    /// The induced causal edges of this run.
    #[must_use]
    pub fn edges(&self) -> &BTreeSet<(String, String)> {
        &self.edges
    }

    /// A stable fingerprint of the node set.
    ///
    /// The node set is durable and can be catalogued, frozen and imported;
    /// the trajectory cannot, so reproducibility attaches to this
    /// (Corollary 11.14).
    #[must_use]
    pub fn protocol_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        for theta in self.nodes.keys() {
            h.update(theta.as_bytes());
            h.update(b"\0");
        }
        let digest = format!("{:x}", h.finalize());
        digest[..16].to_owned()
    }

    /// Total emissions so far.
    #[must_use]
    pub fn record(&self) -> u64 {
        self.record
    }

    /// Number of nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the runtime holds no nodes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

// Note the absence of `succeeded`, `failed`, `exit_code`, `expect` and
// `compare`. Invariant 6 is enforced here by there being nothing to call.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convergence_is_order_independent() {
        let subs = ["a", "b", "c", "d"];
        let mut first = Runtime::new();
        for s in subs {
            first.converge(s, 1, vec![serde_json::json!(1)]);
        }
        let mut second = Runtime::new();
        for s in subs.iter().rev() {
            second.converge(s, 1, vec![serde_json::json!(1)]);
        }
        assert_eq!(
            first.protocol_fingerprint(),
            second.protocol_fingerprint(),
            "Theorem 11.9"
        );
    }

    #[test]
    fn convergence_is_idempotent() {
        let mut rt = Runtime::new();
        rt.converge("a", 1, vec![serde_json::json!(1)]);
        rt.converge("a", 0, vec![serde_json::json!(1)]);
        assert_eq!(rt.read("a").len(), 1);
        assert_eq!(rt.len(), 1);
    }

    #[test]
    fn a_failing_chunk_does_not_halt_the_run() {
        let mut rt = Runtime::new();
        let chunks: Vec<Box<dyn FnOnce() -> std::result::Result<Value, String>>> = vec![
            Box::new(|| Ok(serde_json::json!(1))),
            Box::new(|| Err(String::from("boom"))),
            Box::new(|| Ok(serde_json::json!(3))),
        ];
        let out = rt.execute("n", chunks);
        assert_eq!(out.len(), 3, "Corollary 11.6: every chunk ran");
        assert!(out[1].get("error").is_some(), "the anomaly is a value");
        assert_eq!(rt.read("n").len(), 3, "and it did not suppress the others");
    }

    #[test]
    fn edges_are_produced_by_reading_not_declared() {
        let mut rt = Runtime::new();
        rt.identify("a");
        rt.identify("b");
        assert!(rt.edges().is_empty(), "no edges before any reading");
        rt.emit("a", serde_json::json!(1), None);
        rt.emit("b", serde_json::json!(2), Some("a"));
        assert_eq!(rt.edges().len(), 1, "Proposition 11.12");
    }
}
