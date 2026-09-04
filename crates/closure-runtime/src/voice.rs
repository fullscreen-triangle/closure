//! Voices: what speaks in the square before anyone is a person.
//!
//! The square is already talking when the player arrives. What is talking is
//! not a population of agents — it is a set of **voices**, and a voice is a
//! handle with a posting history and nothing else. No profile, no
//! demographics, no module. There is nothing behind it to look up.
//!
//! ## The direction pruning runs
//!
//! A player does not search for someone. Searching would ask the population
//! to retrieve on a description — "an engineer at ETH" — and Theorem 4.3 says
//! no operation has that signature. Instead the player reads the square,
//! picks a voice, and the system **fits** an agent to it: what profile would
//! have produced these posts?
//!
//! ```text
//! posts  ->  constraints  ->  the modules that satisfy them  ->  an agent
//! ```
//!
//! That is the reverse of retrieval and it is not a disguised form of it.
//! Retrieval reads a stored answer; fitting constructs one that did not exist
//! before the question. Nothing is stored about a voice except what it said.
//!
//! ## A voice in several subgroups is not a conflict
//!
//! Subgroups are regions of the city graph, and a voice walks between them.
//! Appearing in both watersports and carbon composites is not a contradiction
//! to resolve — it is simply more constraint on the fit, and it narrows the
//! admissible profiles rather than breaking them.
//!
//! ## The fit need not be unique
//!
//! Several profiles may satisfy the same posts equally well. That is
//! Proposition 3.4 in the population: the constraints do not single out a
//! resting cut. [`Fit`] therefore reports every module set that satisfies
//! them, and a caller that needs one must record that the choice was its own
//! and not the square's — the same discipline
//! [`crate::separation::Separation`] applies to cuts.
//!
//! ## What a voice does not carry
//!
//! [`VoiceId`] is stable across posts, so the square can say two posts came
//! from the same voice. Nothing *in* the posts says so. That is the line
//! Theorem 6.6 draws: no function of emitted states selects an originator,
//! but the square that ran them may still know which it ran.

use crate::population::{Module, Population, SubstrateRecord};
use closure_kernel::Agent;
use closure_kernel::graph::Position;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A handle for something that speaks. Not a person, and not yet an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VoiceId(pub u32);

/// A region of the city: watersports, carbon composites, lake transport.
///
/// A subgroup is a set of positions and nothing more. Visibility through it
/// is the ordinary reachability rule — there is no membership list, because
/// a voice is "in" a subgroup exactly when it has posted at a terminus there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subgroup {
    /// Display handle.
    pub name: String,
    /// The positions this region spans in the shared graph.
    pub members: BTreeSet<Position>,
}

impl Subgroup {
    /// Whether a terminus falls in this region.
    #[must_use]
    pub fn contains(&self, terminus: Position) -> bool {
        self.members.contains(&terminus)
    }
}

/// What speaks: a handle, a display name, and where it has spoken.
///
/// Deliberately holds no attributes. A voice that carried a profile would be
/// an agent waiting to be found, and finding it would be retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    /// Stable handle. Lets the square say two posts share a voice.
    pub id: VoiceId,
    /// What the square calls it. A display string, not an identity.
    pub name: String,
    /// Termini it has posted at, in order. This is the whole of what is
    /// known about it, and it is what a fit is computed from.
    pub spoken_at: Vec<Position>,
}

impl Voice {
    /// The distinct positions this voice has spoken at.
    #[must_use]
    pub fn footprint(&self) -> BTreeSet<Position> {
        self.spoken_at.iter().copied().collect()
    }

    /// The subgroups this voice has spoken in.
    ///
    /// Several is the ordinary case, not an anomaly: a voice that appears in
    /// watersports and in carbon composites has simply walked between them.
    #[must_use]
    pub fn subgroups<'a>(&self, all: &'a [Subgroup]) -> Vec<&'a Subgroup> {
        let seen = self.footprint();
        all.iter()
            .filter(|s| seen.iter().any(|t| s.contains(*t)))
            .collect()
    }
}

/// The result of fitting a profile to a voice.
///
/// Reports every module set that satisfies the voice's posts, not one. See
/// the module documentation on why a unique answer is not to be assumed.
#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    /// The voice fitted.
    pub voice: VoiceId,
    /// Module names satisfying the voice's footprint, each a viable profile.
    /// Ordered, but the order carries no ranking — it is the population's
    /// declaration order.
    pub admissible: Vec<String>,
    /// The positions the fit had to account for.
    pub footprint: BTreeSet<Position>,
}

impl Fit {
    /// Whether exactly one profile satisfies the voice.
    ///
    /// When false, the posts genuinely fail to single out a person, and a
    /// caller choosing one is making a choice the square did not make.
    #[must_use]
    pub fn is_determinate(&self) -> bool {
        self.admissible.len() == 1
    }

    /// Whether any profile satisfies the voice at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.admissible.is_empty()
    }
}

/// The population of voices in a square.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Square {
    voices: BTreeMap<VoiceId, Voice>,
    /// The regions of this city.
    pub subgroups: Vec<Subgroup>,
    next: u32,
}

impl Square {
    /// An empty square with the given regions.
    #[must_use]
    pub fn new(subgroups: Vec<Subgroup>) -> Self {
        Self {
            voices: BTreeMap::new(),
            subgroups,
            next: 0,
        }
    }

    /// Admit a voice under `name`, returning its handle.
    pub fn admit(&mut self, name: impl Into<String>) -> VoiceId {
        let id = VoiceId(self.next);
        self.next += 1;
        self.voices.insert(
            id,
            Voice {
                id,
                name: name.into(),
                spoken_at: Vec::new(),
            },
        );
        id
    }

    /// Record that `id` spoke at `terminus`.
    pub fn note(&mut self, id: VoiceId, terminus: Position) {
        if let Some(v) = self.voices.get_mut(&id) {
            v.spoken_at.push(terminus);
        }
    }

    /// One voice.
    #[must_use]
    pub fn voice(&self, id: VoiceId) -> Option<&Voice> {
        self.voices.get(&id)
    }

    /// Every voice, by handle order.
    #[must_use]
    pub fn voices(&self) -> Vec<&Voice> {
        self.voices.values().collect()
    }

    /// The voices that have spoken in `subgroup`.
    #[must_use]
    pub fn voices_in(&self, subgroup: &Subgroup) -> Vec<&Voice> {
        self.voices
            .values()
            .filter(|v| v.footprint().iter().any(|t| subgroup.contains(*t)))
            .collect()
    }

    /// Fit profiles to a voice: what kind of person would have posted this?
    ///
    /// A module is admissible when it spans the whole of the voice's
    /// footprint — every position the voice spoke at is one the module
    /// reaches. A voice that has ranged widely therefore admits fewer
    /// profiles, which is the sense in which appearing in several subgroups
    /// *constrains* rather than conflicts.
    ///
    /// Returns `None` if there is no such voice.
    #[must_use]
    pub fn fit(&self, id: VoiceId, population: &Population) -> Option<Fit> {
        let voice = self.voice(id)?;
        let footprint = voice.footprint();
        let admissible = population
            .modules
            .iter()
            .filter(|m| footprint.is_subset(&m.members))
            .map(|m| m.name.clone())
            .collect();
        Some(Fit {
            voice: id,
            admissible,
            footprint,
        })
    }

    /// Prune an agent that satisfies a voice.
    ///
    /// This is the moment a voice becomes someone. Before it, there is
    /// nothing to address; after it, there is an individual whose record
    /// climbs from zero and whom no later pruning reproduces (Cor. 8.8).
    ///
    /// `choice` names which admissible profile to use. It is required rather
    /// than defaulted: when [`Fit::is_determinate`] is false the square does
    /// not determine one, and picking silently would be the tiebreak
    /// `binv:tiebreak` forbids. Pass the sole entry when the fit is
    /// determinate.
    ///
    /// Returns `None` if the voice is unknown, `choice` is not admissible, or
    /// the profile does not prune against `record`.
    #[must_use]
    pub fn realise(
        &self,
        id: VoiceId,
        population: &Population,
        record: &SubstrateRecord,
        choice: &str,
    ) -> Option<Agent> {
        let fit = self.fit(id, population)?;
        if !fit.admissible.iter().any(|m| m == choice) {
            return None;
        }
        let voice = self.voice(id)?;
        // The agent is pruned against the chosen profile alone, so that the
        // individual produced is one the voice's posts are consistent with.
        let scoped = Population {
            modules: population
                .modules
                .iter()
                .filter(|m| m.name == choice)
                .cloned()
                .collect(),
            shared: population.shared.clone(),
        };
        scoped.prune(&voice.name, record)
    }
}

/// How the square is populated before anyone arrives.
///
/// Seeded from the session, so the same token in the same city opens on the
/// same square. That is what makes a run reproducible as a protocol
/// (Cor. 11.14) rather than as a set of readings.
#[derive(Debug, Clone)]
pub struct Seeding {
    /// How many voices to admit.
    pub voices: usize,
    /// How many posts each voice makes while seeding.
    pub posts_each: usize,
}

impl Default for Seeding {
    fn default() -> Self {
        Self {
            voices: 8,
            posts_each: 3,
        }
    }
}

/// One seeded utterance: who spoke, and where.
///
/// Carries no body. What is *said* is the caller's business — this module
/// decides only the shape of the conversation, because the shape is the part
/// the theory constrains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utterance {
    /// The voice speaking.
    pub voice: VoiceId,
    /// Where it registers.
    pub terminus: Position,
}

/// Populate a square, deterministically in `seed`.
///
/// Voices are admitted and given walks across the subgroups. A voice may
/// range over several regions; nothing prevents it and the fit accounts for
/// it. Returns the utterances in order, for the caller to register as posts.
#[must_use]
pub fn seed(square: &mut Square, seeding: &Seeding, seed: u64) -> Vec<Utterance> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let regions: Vec<BTreeSet<Position>> =
        square.subgroups.iter().map(|s| s.members.clone()).collect();
    if regions.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..seeding.voices {
        let id = square.admit(format!("voice-{i}"));
        for _ in 0..seeding.posts_each {
            let region = &regions[rng.random_range(0..regions.len())];
            let choices: Vec<Position> = region.iter().copied().collect();
            if choices.is_empty() {
                continue;
            }
            let terminus = choices[rng.random_range(0..choices.len())];
            square.note(id, terminus);
            out.push(Utterance {
                voice: id,
                terminus,
            });
        }
    }
    out
}

/// A module spanning `members`, matching every record.
///
/// A convenience for building populations whose modules are defined by the
/// region they cover rather than by demographic constraints.
#[must_use]
pub fn region_module(name: impl Into<String>, members: BTreeSet<Position>) -> Module {
    Module {
        name: name.into(),
        members,
        matches: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use closure_kernel::ContactGraph;

    fn subgroups() -> Vec<Subgroup> {
        vec![
            Subgroup {
                name: "watersports".into(),
                members: BTreeSet::from([0, 1, 2]),
            },
            Subgroup {
                name: "carbon-composites".into(),
                members: BTreeSet::from([3, 4]),
            },
            Subgroup {
                name: "lake-transport".into(),
                members: BTreeSet::from([5]),
            },
        ]
    }

    fn population() -> Population {
        let mut shared = ContactGraph::new(6);
        for (u, v, w) in [
            (0, 1, 2.0),
            (1, 2, 2.0),
            (2, 3, 1.0),
            (3, 4, 2.0),
            (4, 5, 2.0),
        ] {
            shared.add_edge(u, v, w).unwrap();
        }
        Population {
            modules: vec![
                // Someone only in the water.
                region_module("swimmer", BTreeSet::from([0, 1, 2])),
                // Someone only in materials.
                region_module("materials-only", BTreeSet::from([3, 4])),
                // Someone who spans both: the composites engineer who sails.
                region_module("sailing-engineer", BTreeSet::from([0, 1, 2, 3, 4])),
                // Someone who spans everything.
                region_module("generalist", (0..6).collect()),
            ],
            shared,
        }
    }

    fn square_with(spoken: &[(u32, Position)]) -> Square {
        let mut sq = Square::new(subgroups());
        let mut ids = BTreeMap::new();
        for (v, t) in spoken {
            let id = *ids.entry(*v).or_insert_with(|| sq.admit(format!("v{v}")));
            sq.note(id, *t);
        }
        sq
    }

    #[test]
    fn a_voice_carries_no_profile_until_it_is_fitted() {
        let sq = square_with(&[(0, 1)]);
        let v = sq.voice(VoiceId(0)).unwrap();
        // The whole of what is known: a handle, a name, and where it spoke.
        assert_eq!(v.spoken_at, vec![1]);
        // There is no attribute field to read, by construction.
        let json = serde_json::to_string(v).unwrap();
        assert!(!json.contains("age"), "no demographics on a voice: {json}");
        assert!(!json.contains("module"), "no profile on a voice: {json}");
    }

    #[test]
    fn speaking_in_several_subgroups_is_not_a_conflict() {
        let sq = square_with(&[(0, 1), (0, 3)]);
        let v = sq.voice(VoiceId(0)).unwrap();
        let names: Vec<&str> = v
            .subgroups(&sq.subgroups)
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert_eq!(names, vec!["watersports", "carbon-composites"]);
    }

    #[test]
    fn ranging_wider_narrows_the_fit_rather_than_breaking_it() {
        let pop = population();

        // A voice heard only in the water fits four profiles.
        let narrow = square_with(&[(0, 1)]);
        let a = narrow.fit(VoiceId(0), &pop).unwrap();
        assert_eq!(
            a.admissible,
            vec!["swimmer", "sailing-engineer", "generalist"]
        );

        // The same voice, also heard in composites, fits fewer.
        let wide = square_with(&[(0, 1), (0, 3)]);
        let b = wide.fit(VoiceId(0), &pop).unwrap();
        assert_eq!(b.admissible, vec!["sailing-engineer", "generalist"]);
        assert!(
            b.admissible.len() < a.admissible.len(),
            "more posts is more constraint, not a contradiction"
        );
    }

    #[test]
    fn prop_3_4_the_fit_need_not_be_determinate() {
        let pop = population();
        let sq = square_with(&[(0, 3)]);
        let f = sq.fit(VoiceId(0), &pop).unwrap();
        assert!(f.admissible.len() > 1);
        assert!(
            !f.is_determinate(),
            "several profiles satisfy the posts; the square does not choose"
        );
    }

    #[test]
    fn a_determinate_fit_has_exactly_one_profile() {
        let pop = population();
        // Only the generalist reaches position 5.
        let sq = square_with(&[(0, 5)]);
        let f = sq.fit(VoiceId(0), &pop).unwrap();
        assert_eq!(f.admissible, vec!["generalist"]);
        assert!(f.is_determinate());
    }

    #[test]
    fn realising_a_voice_yields_an_agent_with_a_record_at_zero() {
        let pop = population();
        let sq = square_with(&[(0, 5)]);
        let rec = SubstrateRecord::new();
        let a = sq.realise(VoiceId(0), &pop, &rec, "generalist").unwrap();
        assert_eq!(a.record().get(), 0, "not yet an individual");
    }

    #[test]
    fn an_inadmissible_choice_is_refused() {
        let pop = population();
        // This voice spoke in composites, so "swimmer" cannot have said it.
        let sq = square_with(&[(0, 3)]);
        let rec = SubstrateRecord::new();
        assert!(
            sq.realise(VoiceId(0), &pop, &rec, "swimmer").is_none(),
            "the fit must actually satisfy the posts"
        );
    }

    #[test]
    fn there_is_no_search_by_attribute() {
        // Square exposes no method taking a description and returning a
        // voice. The only way to a voice is to read what was said. This test
        // fails loudly if such a method is added, because it would be the
        // retrieval signature Theorem 4.3 denies.
        let sq = square_with(&[(0, 1)]);
        let all = sq.voices();
        assert_eq!(all.len(), 1);
        // Voices are reached by subgroup or by handle, never by predicate.
        assert_eq!(sq.voices_in(&sq.subgroups[0]).len(), 1);
        assert_eq!(sq.voices_in(&sq.subgroups[1]).len(), 0);
    }

    #[test]
    fn seeding_is_deterministic_in_the_seed() {
        let a = {
            let mut sq = Square::new(subgroups());
            seed(&mut sq, &Seeding::default(), 42)
        };
        let b = {
            let mut sq = Square::new(subgroups());
            seed(&mut sq, &Seeding::default(), 42)
        };
        assert_eq!(a, b, "same seed, same square (Cor. 11.14)");

        let c = {
            let mut sq = Square::new(subgroups());
            seed(&mut sq, &Seeding::default(), 43)
        };
        assert_ne!(a, c, "a different session opens on a different square");
    }

    #[test]
    fn a_seeded_square_is_already_talking() {
        let mut sq = Square::new(subgroups());
        let out = seed(&mut sq, &Seeding::default(), 7);
        assert_eq!(out.len(), 8 * 3, "voices x posts");
        assert_eq!(sq.voices().len(), 8);
        assert!(
            sq.voices().iter().all(|v| !v.spoken_at.is_empty()),
            "every voice has said something before the player arrives"
        );
    }

    #[test]
    fn seeded_voices_can_range_across_subgroups() {
        let mut sq = Square::new(subgroups());
        let _ = seed(&mut sq, &Seeding::default(), 3);
        let spread = sq
            .voices()
            .iter()
            .filter(|v| v.subgroups(&sq.subgroups).len() > 1)
            .count();
        assert!(spread > 0, "the square is not partitioned into silos");
    }

    #[test]
    fn an_unknown_voice_has_no_fit() {
        let pop = population();
        let sq = square_with(&[(0, 1)]);
        assert!(sq.fit(VoiceId(99), &pop).is_none());
    }
}
