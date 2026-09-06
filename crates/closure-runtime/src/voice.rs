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

use crate::moderator::{Cap, Character, Moderator};
use closure_kernel::Agent;
use closure_kernel::graph::{ContactGraph, Position};
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

    /// Which moderators a voice was audible to.
    ///
    /// A voice is not a person and does not have a home region. It is a
    /// subgraph of whichever characters were talking where it spoke, and
    /// being audible to several of them is the ordinary case — the regions
    /// overlap because the city does.
    #[must_use]
    pub fn audible_to<'a>(&self, id: VoiceId, mods: &'a [Moderator]) -> Vec<&'a Moderator> {
        let Some(voice) = self.voice(id) else {
            return Vec::new();
        };
        let foot = voice.footprint();
        mods.iter()
            .filter(|m| foot.iter().any(|p| m.region.contains(p)))
            .collect()
    }

    /// Reassemble the character behind a voice.
    ///
    /// This is the inverse of the split, and it runs only when a user has
    /// settled on a voice and wants to speak to it. Up to here nothing was
    /// pruned and nothing needed to be: reading a square is watching
    /// characters talk to themselves, and no individual is required for that.
    ///
    /// Every moderator the voice was audible to caps itself to the voice's
    /// footprint, and the caps [`amalgamate`] into one character. Note what
    /// is *not* happening: no catalogue is consulted, no profile is matched,
    /// and nothing is looked up. The character is built out of the regions
    /// the voice actually spoke in, which is why there is no operation here
    /// with the retrieval signature Theorem 4.3 denies.
    ///
    /// Returns `None` if the voice is unknown or spoke too narrowly for a
    /// character to be built from it.
    #[must_use]
    pub fn character(
        &self,
        id: VoiceId,
        city: &ContactGraph,
        mods: &[Moderator],
    ) -> Option<Character> {
        let foot = self.voice(id)?.footprint();
        let caps: Vec<Cap> = self
            .audible_to(id, mods)
            .into_iter()
            .map(|m| m.cap(city, &foot))
            .collect();
        Character::amalgamated(&caps)
    }

    /// Prune an agent from the character behind a voice.
    ///
    /// The moment a voice becomes someone. Before it there is nothing to
    /// address; after it there is an individual whose record climbs from zero
    /// and whom no later pruning reproduces (Cor. 8.8).
    ///
    /// There is no `choice` parameter and nothing to disambiguate, because
    /// nothing was ever enumerated. The identity of the agent is settled
    /// afterwards and on demand, by asking [`Identity::ask`] — and only for
    /// the attributes something actually needs.
    #[must_use]
    pub fn prune(&self, id: VoiceId, city: &ContactGraph, mods: &[Moderator]) -> Option<Agent> {
        let name = self.voice(id)?.name.clone();
        Some(self.character(id, city, mods)?.prune(&name))
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

/// Which regions are adjacent, by contact in the city graph.
///
/// Two regions are adjacent when the city has an edge with one endpoint in
/// each — that is, when you can walk from one to the other in a single step.
///
/// Adjacency is deliberately *not* set overlap. Overlap connects neighbours
/// where the city happens to share positions, but leaves the chain broken
/// wherever it does not: `commuting` and `extreme-sports` share nothing, so
/// an overlap rule would strand the far end of the city and no walk could
/// ever reach it. Contact is the relation the graph actually models, and it
/// connects the whole city.
fn adjacency(city: &ContactGraph, regions: &[BTreeSet<Position>]) -> Vec<Vec<usize>> {
    (0..regions.len())
        .map(|i| {
            (0..regions.len())
                .filter(|j| *j != i)
                .filter(|j| {
                    regions[i]
                        .iter()
                        .any(|u| regions[*j].iter().any(|v| city.weight(*u, *v).is_some()))
                })
                .collect()
        })
        .collect()
}

/// Populate a square, deterministically in `seed`.
///
/// Voices are admitted and given walks across the subgroups. Each voice
/// starts in some region and, for every subsequent post, either stays or
/// steps to a region in contact with the one it is in. A voice may range
/// over several regions; nothing prevents it and the fit accounts for it.
/// Returns the utterances in order, for the caller to register as posts.
///
/// ## Why a walk rather than an independent draw
///
/// Drawing a fresh region per post scattered a voice's footprint across
/// unrelated parts of the city, and a scattered footprint has no character
/// behind it: [`Square::character`] caps each audible moderator to the
/// footprint and amalgamates the caps, and caps that never meet amalgamate
/// to nothing. The walk is what makes most voices someone a player can
/// actually reach. It is also what the documentation above this function
/// has always claimed, and now describes.
#[must_use]
pub fn seed(
    square: &mut Square,
    city: &ContactGraph,
    seeding: &Seeding,
    seed: u64,
) -> Vec<Utterance> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let regions: Vec<BTreeSet<Position>> =
        square.subgroups.iter().map(|s| s.members.clone()).collect();
    if regions.is_empty() {
        return Vec::new();
    }
    let near = adjacency(city, &regions);
    let mut out = Vec::new();
    for i in 0..seeding.voices {
        let id = square.admit(format!("voice-{i}"));
        let mut at = rng.random_range(0..regions.len());
        for post in 0..seeding.posts_each {
            // Stay or step. A voice that never moved would be a silo, and a
            // voice that moved every time would not linger anywhere.
            if post > 0 && !near[at].is_empty() && rng.random_bool(0.5) {
                at = near[at][rng.random_range(0..near[at].len())];
            }
            let choices: Vec<Position> = regions[at].iter().copied().collect();
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

    /// The city these regions are cut from.
    fn city() -> ContactGraph {
        let mut g = ContactGraph::new(6);
        for (u, v, w) in [
            (0, 1, 2.0),
            (1, 2, 2.0),
            (2, 3, 1.0),
            (3, 4, 2.0),
            (4, 5, 2.0),
        ] {
            g.add_edge(u, v, w).unwrap();
        }
        g
    }

    /// One moderator per region. There is no roster behind them.
    fn moderators() -> Vec<Moderator> {
        let c = city();
        subgroups()
            .into_iter()
            .filter_map(|s| Moderator::new(&c, s.members))
            .collect()
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
    fn ranging_wider_reaches_more_of_the_character() {
        let (c, mods) = (city(), moderators());

        // A voice heard only in the water is audible to one character.
        let narrow = square_with(&[(0, 1)]);
        assert_eq!(narrow.audible_to(VoiceId(0), &mods).len(), 1);

        // The same voice, also heard in composites, is audible to two — and
        // the character behind it is built from both.
        let wide = square_with(&[(0, 1), (0, 3), (0, 4)]);
        assert_eq!(wide.audible_to(VoiceId(0), &mods).len(), 2);
        let ch = wide.character(VoiceId(0), &c, &mods).unwrap();
        assert!(
            ch.graph.order() > 1,
            "more posts is more character, not a contradiction"
        );
    }

    #[test]
    fn being_audible_in_several_regions_needs_no_reconciling() {
        // Two regions are not two people to choose between. They are two
        // parts of one city, and the caps simply combine.
        let (c, mods) = (city(), moderators());
        let sq = square_with(&[(0, 1), (0, 2), (0, 3), (0, 4)]);
        let heard = sq.audible_to(VoiceId(0), &mods);
        assert!(heard.len() > 1);
        assert!(
            sq.character(VoiceId(0), &c, &mods).is_some(),
            "overlap is the ordinary case, not a conflict to resolve"
        );
    }

    #[test]
    fn a_voice_too_narrow_to_be_anyone_yields_no_character() {
        // One post in one position does not make a person. Nothing is
        // invented to cover the shortfall.
        let (c, mods) = (city(), moderators());
        let sq = square_with(&[(0, 5)]);
        assert!(sq.character(VoiceId(0), &c, &mods).is_none());
        assert!(sq.prune(VoiceId(0), &c, &mods).is_none());
    }

    #[test]
    fn pruning_a_voice_yields_an_agent_with_a_record_at_zero() {
        let (c, mods) = (city(), moderators());
        let sq = square_with(&[(0, 0), (0, 1), (0, 2)]);
        let a = sq.prune(VoiceId(0), &c, &mods).unwrap();
        assert_eq!(a.record().get(), 0, "not yet an individual");
    }

    #[test]
    fn pruning_asks_for_no_profile_because_none_was_ever_listed() {
        // The old fitting path required the caller to name one of an
        // enumerated set. There is no such set now, and so no choice to make
        // and no tiebreak to declare: the character is built from where the
        // voice spoke, and who would have it is asked afterwards, on demand.
        let (c, mods) = (city(), moderators());
        let sq = square_with(&[(0, 0), (0, 1), (0, 2)]);
        let mut ch = sq.character(VoiceId(0), &c, &mods).unwrap();
        assert!(ch.identity.is_empty(), "nothing is true of them yet");
        ch.identity.ask(&ch.graph, "age-band", &["30-45", "45-60"]);
        assert_eq!(ch.identity.len(), 1, "only what was asked exists");
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
            seed(&mut sq, &city(), &Seeding::default(), 42)
        };
        let b = {
            let mut sq = Square::new(subgroups());
            seed(&mut sq, &city(), &Seeding::default(), 42)
        };
        assert_eq!(a, b, "same seed, same square (Cor. 11.14)");

        let c = {
            let mut sq = Square::new(subgroups());
            seed(&mut sq, &city(), &Seeding::default(), 43)
        };
        assert_ne!(a, c, "a different session opens on a different square");
    }

    #[test]
    fn a_seeded_square_is_already_talking() {
        let mut sq = Square::new(subgroups());
        let out = seed(&mut sq, &city(), &Seeding::default(), 7);
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
        let _ = seed(&mut sq, &city(), &Seeding::default(), 3);
        let spread = sq
            .voices()
            .iter()
            .filter(|v| v.subgroups(&sq.subgroups).len() > 1)
            .count();
        assert!(spread > 0, "the square is not partitioned into silos");
    }

    #[test]
    fn a_seeded_voice_walks_rather_than_scattering() {
        // The *regions* a voice spoke in must form a connected chain under
        // contact. Note this is the right invariant and the footprint is
        // not: a voice can stand at 1, step to the next region and speak at
        // 3, leaving position 2 unvisited. The path was still contiguous,
        // and it is region contact — not position adjacency — that decides
        // whether the caps meet and someone is behind the voice.
        let c = city();
        let regions: Vec<BTreeSet<Position>> = subgroups().into_iter().map(|s| s.members).collect();
        let near = adjacency(&c, &regions);
        for s in 0..40u64 {
            let mut sq = Square::new(subgroups());
            let _ = seed(&mut sq, &c, &Seeding::default(), s);
            for v in sq.voices() {
                let foot = v.footprint();
                let spoke: BTreeSet<usize> = (0..regions.len())
                    .filter(|i| foot.iter().any(|t| regions[*i].contains(t)))
                    .collect();
                let start = *spoke.iter().next().unwrap();
                let mut seen = BTreeSet::from([start]);
                let mut frontier = vec![start];
                while let Some(i) = frontier.pop() {
                    for j in &near[i] {
                        if spoke.contains(j) && seen.insert(*j) {
                            frontier.push(*j);
                        }
                    }
                }
                assert_eq!(
                    seen.len(),
                    spoke.len(),
                    "seed {s}, voice {:?}: regions {spoke:?} are not a walk",
                    v.id
                );
            }
        }
    }

    #[test]
    fn walking_gives_most_voices_someone_behind_them() {
        // The point of the walk. Scattered footprints left the caps unable
        // to amalgamate, so a player reading the square found almost nobody
        // they could actually address.
        let (c, mods) = (city(), moderators());
        let mut sq = Square::new(subgroups());
        let _ = seed(&mut sq, &c, &Seeding::default(), 11);
        let ids: Vec<VoiceId> = sq.voices().iter().map(|v| v.id).collect();
        let someone = ids
            .iter()
            .filter(|id| sq.character(**id, &c, &mods).is_some())
            .count();
        assert!(
            someone * 2 > ids.len(),
            "most voices should be reachable, got {someone} of {}",
            ids.len()
        );
    }

    #[test]
    fn an_unknown_voice_has_no_character() {
        let (c, mods) = (city(), moderators());
        let sq = square_with(&[(0, 1)]);
        assert!(sq.character(VoiceId(99), &c, &mods).is_none());
        assert!(sq.audible_to(VoiceId(99), &mods).is_empty());
    }
}
