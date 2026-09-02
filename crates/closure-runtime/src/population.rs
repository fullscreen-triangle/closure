//! Modules, substrate, and pruning (Section 8).
//!
//! A population is stored coarsely as *modules* — patterns of disposition —
//! and individuals are produced on demand by pruning modules against a
//! demographic record. This is not an approximation of individuals: by
//! Corollary 3.4 the object individuated at any resolution is a region and
//! never a position, so a module is the object that exists at that
//! resolution (Remark 8.1). Storing many identical instances would in any
//! case add nothing, since reach is bounded by *distinct* resolutions rather
//! than by headcount (Cor. 4.22).
//!
//! **Modules are patterns, never persons.** No module is derived from, or
//! intended to represent, an identified individual; pruning yields fictional
//! agents consistent with aggregate statistics (Section 14).

use closure_kernel::{Agent, ContactGraph, Position};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A record drawn from the substrate: the aggregate attributes against which
/// modules are matched. Deliberately coarse — district, age band, origin —
/// and never an identifier.
pub type SubstrateRecord = std::collections::BTreeMap<String, String>;

/// A thought-pattern region: a class of disposition, not a person.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    /// Stable handle.
    pub name: String,
    /// The positions this pattern spans in the shared graph.
    pub members: BTreeSet<Position>,
    /// Attribute constraints a record must satisfy to match, as
    /// `(key, allowed values)`. All constraints must hold.
    pub matches: Vec<(String, Vec<String>)>,
}

impl Module {
    /// Whether a substrate record falls in this pattern.
    #[must_use]
    pub fn admits(&self, record: &SubstrateRecord) -> bool {
        self.matches.iter().all(|(k, allowed)| {
            record
                .get(k)
                .is_some_and(|v| allowed.iter().any(|a| a == v))
        })
    }
}

/// The coarse population, plus the shared graph individuals are cut from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    /// The modules covering this city.
    pub modules: Vec<Module>,
    /// The shared contact graph.
    pub shared: ContactGraph,
}

impl Population {
    /// Prune the modules matching `record` into an individual.
    ///
    /// Deterministic in `(modules, record)`: the same record pruned against
    /// the same modules always yields the same graph and hence the same
    /// character invariant (Theorem 8.7). The result is *not yet* an
    /// individual — it has an invariant but a record at zero, so nothing
    /// distinguishes it from a faithful copy until it first renders.
    ///
    /// Pruning is therefore idempotent before contact and irreversible after
    /// (Corollary 8.8): once an agent has committed an act its record exceeds
    /// zero, and no pruning reproduces it.
    #[must_use]
    pub fn prune(&self, id: &str, record: &SubstrateRecord) -> Option<Agent> {
        let hits: Vec<&Module> = self.modules.iter().filter(|m| m.admits(record)).collect();
        if hits.is_empty() {
            return None;
        }
        // The individual is the intersection of the patterns it falls in; if
        // those patterns do not overlap, fall back to the first, which yields
        // a coarser agent rather than none. Extraction error coarsens; it does
        // not corrupt (Cor. 8.6 of the source calculus).
        let mut members = hits[0].members.clone();
        for m in &hits[1..] {
            let overlap: BTreeSet<Position> = members.intersection(&m.members).copied().collect();
            if !overlap.is_empty() {
                members = overlap;
            }
        }
        Some(Agent::new(id, induced(&self.shared, &members)))
    }

    /// Names of the modules a record falls in. Exposed so a session can show
    /// a player which patterns were in play, without exposing any identity.
    #[must_use]
    pub fn matching_modules(&self, record: &SubstrateRecord) -> Vec<&str> {
        self.modules
            .iter()
            .filter(|m| m.admits(record))
            .map(|m| m.name.as_str())
            .collect()
    }
}

/// The subgraph induced on `members`, with positions renumbered from zero.
fn induced(graph: &ContactGraph, members: &BTreeSet<Position>) -> ContactGraph {
    let index: std::collections::BTreeMap<Position, Position> = members
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i as Position))
        .collect();
    let mut out = ContactGraph::new(members.len() as u32);
    for u in members {
        for v in members {
            if u >= v {
                continue;
            }
            let w = graph.cut_weight(&BTreeSet::from([*u]));
            // Preserve a contact only where the shared graph has one; weight
            // is inherited from the shared structure.
            if w > 0.0 {
                let _ = out.add_edge(index[u], index[v], edge_weight(graph, *u, *v));
            }
        }
    }
    out
}

/// Weight of the contact between two positions in the shared graph, or the
/// floor if they are not directly joined.
fn edge_weight(graph: &ContactGraph, u: Position, v: Position) -> f64 {
    let pair = BTreeSet::from([u]);
    let both = BTreeSet::from([u, v]);
    let w = (graph.cut_weight(&pair) + graph.cut_weight(&BTreeSet::from([v]))
        - graph.cut_weight(&both))
        / 2.0;
    if w > 0.0 {
        w
    } else {
        graph.floor().unwrap_or(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn population() -> Population {
        let mut shared = ContactGraph::new(6);
        for (u, v, w) in [
            (0, 1, 2.0),
            (1, 2, 3.0),
            (2, 3, 2.0),
            (3, 4, 4.0),
            (4, 5, 2.0),
            (0, 5, 3.0),
        ] {
            shared.add_edge(u, v, w).unwrap();
        }
        let modules = vec![
            Module {
                name: "commuter".into(),
                members: (0..4).collect(),
                matches: vec![("age_band".into(), vec!["20-39".into(), "40-59".into()])],
            },
            Module {
                name: "resident".into(),
                members: (2..6).collect(),
                matches: vec![("origin".into(), vec!["local".into()])],
            },
        ];
        Population { modules, shared }
    }

    fn record(pairs: &[(&str, &str)]) -> SubstrateRecord {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn pruning_is_deterministic_before_contact() {
        let pop = population();
        let r = record(&[("age_band", "20-39"), ("origin", "local")]);
        let a = pop.prune("x", &r).unwrap();
        let b = pop.prune("x", &r).unwrap();
        assert_eq!(a.chi(), b.chi(), "Theorem 8.7");
        assert_eq!(a.record(), b.record());
    }

    #[test]
    fn a_generated_agent_is_not_yet_an_individual() {
        let pop = population();
        let r = record(&[("age_band", "20-39"), ("origin", "local")]);
        let a = pop.prune("x", &r).unwrap();
        assert_eq!(a.record().get(), 0, "no history until it renders");
    }

    #[test]
    fn contact_makes_it_irreproducible() {
        let pop = population();
        let r = record(&[("age_band", "20-39"), ("origin", "local")]);
        let mut a = pop.prune("x", &r).unwrap();
        a.determine(|_| ()).unwrap();
        let fresh = pop.prune("x", &r).unwrap();
        assert_ne!(a.record(), fresh.record(), "Corollary 8.8");
    }

    #[test]
    fn a_record_matching_nothing_yields_no_agent() {
        let pop = population();
        assert!(pop.prune("x", &record(&[("age_band", "0-19")])).is_none());
    }

    #[test]
    fn matching_modules_are_reported_without_identity() {
        let pop = population();
        let r = record(&[("age_band", "20-39"), ("origin", "local")]);
        let names = pop.matching_modules(&r);
        assert_eq!(names, vec!["commuter", "resident"]);
    }
}
