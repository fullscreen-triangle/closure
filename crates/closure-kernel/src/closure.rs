//! Considerations, reach, and closure (Definitions 4.1-4.4).
//!
//! The central fact about this module is what it does *not* read. A
//! [`Consideration`] carries a `provenance` label and a `veridical` flag, and
//! no function here inspects either. They exist so that provenance- and
//! truth-blindness (Theorems 4.9 and 4.11) can be *stated* and tested: one
//! cannot permute what is not there.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A sufficient region — the unit a consideration resolves into.
pub type Region = BTreeSet<crate::Position>;

/// The set of regions reachable from a seed under some considerations.
pub type Reach = BTreeSet<Region>;

/// A means of advancing an inquiry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consideration {
    /// Stable name, used for bookkeeping only.
    pub name: String,

    /// Where the resolution came from: verification, testimony, rumour.
    ///
    /// **Read by nothing.** It is an arbitrary label; permuting it must leave
    /// every verdict unchanged (Theorem 4.9).
    pub provenance: String,

    /// The region invoking this consideration reaches, per seed.
    pub resolution: BTreeMap<crate::Position, Region>,

    /// Whether the resolution is correct.
    ///
    /// **Read by nothing.** Present only so that Theorem 4.11 can be stated:
    /// substituting a false consideration for a true one with the same
    /// resolution changes no verdict.
    pub veridical: bool,

    /// Event type, used by the type-averaged test (Def. 12.5). Never by the
    /// instance-fitted one, which has no null.
    pub ctype: String,
}

impl Consideration {
    /// A consideration resolving `seed` into `region`.
    #[must_use]
    pub fn new(name: impl Into<String>, seed: crate::Position, region: Region) -> Self {
        Self {
            name: name.into(),
            provenance: String::from("unspecified"),
            resolution: BTreeMap::from([(seed, region)]),
            veridical: true,
            ctype: String::from("generic"),
        }
    }

    /// Attach a provenance label. Changes no verdict, by construction.
    #[must_use]
    pub fn with_provenance(mut self, p: impl Into<String>) -> Self {
        self.provenance = p.into();
        self
    }

    /// Mark the resolution false. Changes no verdict, by construction.
    #[must_use]
    pub fn with_veridical(mut self, v: bool) -> Self {
        self.veridical = v;
        self
    }

    /// The region this consideration reaches from `seed`, if any.
    #[must_use]
    pub fn resolve(&self, seed: crate::Position) -> Option<&Region> {
        self.resolution.get(&seed)
    }
}

/// `Reach(v0, Gamma)` — the set of regions the considerations resolve into.
#[must_use]
pub fn reach<'a, I>(seed: crate::Position, considerations: I) -> Reach
where
    I: IntoIterator<Item = &'a Consideration>,
{
    considerations
        .into_iter()
        .filter_map(|c| c.resolve(seed).cloned())
        .collect()
}

/// `Cl(v0, invoked, available)` — whether every available but uninvoked
/// consideration resolves into a region already reached.
///
/// This is strictly stronger than any confidence threshold (Theorem 4.6):
/// it asks not how strongly something is believed but whether anything
/// available could still change the answer.
#[must_use]
pub fn is_closed(
    seed: crate::Position,
    invoked: &[Consideration],
    available: &[Consideration],
) -> bool {
    reach(seed, invoked.iter()) == reach(seed, available.iter())
}

/// Whether an agent commits to `region` (Definition 4.4).
#[must_use]
pub fn commits_to(
    seed: crate::Position,
    region: &Region,
    invoked: &[Consideration],
    available: &[Consideration],
) -> bool {
    is_closed(seed, invoked, available) && reach(seed, invoked.iter()).contains(region)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(xs: &[crate::Position]) -> Region {
        xs.iter().copied().collect()
    }

    fn setup() -> (Consideration, Consideration) {
        let a = Consideration::new("a", 0, region(&[0, 1, 2])).with_provenance("verified");
        let b = Consideration::new("b", 0, region(&[3, 4, 5])).with_provenance("rumour");
        (a, b)
    }

    #[test]
    fn closure_is_stronger_than_any_threshold() {
        let (a, b) = setup();
        // `a` alone would satisfy any confidence threshold, yet an available
        // consideration reaches elsewhere, so the search is not closed.
        assert!(
            !is_closed(0, std::slice::from_ref(&a), &[a.clone(), b]),
            "Theorem 4.6"
        );
        let only_a = [a];
        assert!(
            is_closed(0, &only_a, &only_a),
            "closed once nothing else reaches"
        );
    }

    #[test]
    fn provenance_is_not_read() {
        let (a, b) = setup();
        let base = is_closed(0, std::slice::from_ref(&a), &[a.clone(), b.clone()]);
        let a2 = a.with_provenance("something else entirely");
        let b2 = b.with_provenance("and another");
        assert_eq!(
            base,
            is_closed(0, std::slice::from_ref(&a2), &[a2.clone(), b2]),
            "Theorem 4.9"
        );
    }

    #[test]
    fn truth_is_not_read() {
        let (a, b) = setup();
        let base = is_closed(0, std::slice::from_ref(&a), &[a.clone(), b.clone()]);
        let a2 = a.with_veridical(false);
        let b2 = b.with_veridical(false);
        assert_eq!(
            base,
            is_closed(0, std::slice::from_ref(&a2), &[a2.clone(), b2]),
            "Theorem 4.11"
        );
    }

    #[test]
    fn availability_is_not_monotone() {
        let a = Consideration::new("a", 0, region(&[0, 1]));
        let existing = [a.clone()];
        assert!(is_closed(0, &existing, &existing));
        // A newcomer reaching a *new* region destroys the closure, and nobody
        // changed their mind about anything already considered (Rem. 4.13).
        let newcomer = Consideration::new("n", 0, region(&[7, 8]));
        assert!(!is_closed(0, &existing, &[a, newcomer]));
    }
}
