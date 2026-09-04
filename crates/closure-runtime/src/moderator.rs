//! The moderator, its splits, and the amalgamation that precedes pruning.
//!
//! A subgroup does not contain a population. It contains **one character**,
//! split into instances that each hold a different part of what that
//! character can tell apart. The instances talk because they are short of
//! each other — the deficit is manufactured, and it is the whole reason a
//! question is ever asked.
//!
//! ## The moderator is not an oracle
//!
//! It would be easy to read the split as a projection of complete knowledge:
//! one thing that knows everything, handing out restrictions. That reading is
//! wrong, and the difference is load-bearing.
//!
//! The moderator is a **character**, which means it has an invariant and a
//! residual gap like anything else. Its own gap is never zero. If it held the
//! closed answer, its private questioning would be theatre — it would be
//! prompting for something it already had, and Theorem 7.4 says exchange does
//! not close, which forbids exactly that arrangement. So the moderator asks
//! what *it* cannot close, the instances inherit that unclosed gap, and the
//! discussion propagates uncertainty rather than distributing an answer.
//!
//! This is also why the game cannot be won. The goal is unreachable for the
//! moderator too.
//!
//! ## Talking to itself
//!
//! The moderator questions instances privately; they answer publicly. A
//! thread is therefore one character in conversation with itself, and
//! Theorem 6.2 is not violated by that: each private question is an
//! independent registration against a *different* instance graph, and no
//! graph makes two of those jointly cuts. Splitting is what makes the
//! registrations independent.
//!
//! ## What a subgroup boundary is worth
//!
//! Nothing. Two posts in a subgroup, and two subgroups, differ by which
//! region of the one character's graph they sit in — and by nothing else. A
//! voice heard in two subgroups is not a case to reconcile; it is one
//! character being audible in two of its own regions.
//!
//! ## Pruning runs the split backwards
//!
//! When a user settles on a voice, the moderators of the regions that voice
//! was audible in each **cap** themselves to that region, and the caps
//! [`amalgamate`] into a single character. Only then is identity asked, and
//! it is asked as a question about *sources*: who is most likely to have this
//! character? The answer is deliberately partial — see [`Identity`].

use crate::act::{Act, Gaps, classify};
use crate::forum::{Forum, PostId, Speaker, at};
use closure_kernel::graph::{ContactGraph, EdgeWeight, Position};
use closure_kernel::identity::{Agent, Record};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// How much a blunted contact is worth to the instance reading it blunt.
///
/// Strictly between zero and one: at zero the contact would be gone and the
/// instance would lose its region rather than its resolution; at one there
/// would be no split at all. It also bounds the goal — see
/// [`Moderator::new`] — so that no split can close what the whole cannot.
const BLUNT: EdgeWeight = 0.5;

/// A handle for one instance of a moderator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct InstanceId(pub u32);

/// One character, holding a region of the city and a gap it cannot close.
///
/// The moderator is what a subgroup *is*. There is no roster behind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Moderator {
    /// Which region of the city this character occupies.
    pub region: BTreeSet<Position>,
    /// The character's own graph: the distinctions it can draw.
    pub graph: ContactGraph,
    /// What the character is trying to close and cannot.
    ///
    /// Strictly positive by construction — see [`Moderator::new`]. A
    /// moderator with a closed goal would be an oracle, and its questioning
    /// would be theatre (Thm 7.4).
    goal: EdgeWeight,
}

impl Moderator {
    /// A moderator over `region`, cut from `city`.
    ///
    /// The goal is set strictly *below* what the character can resolve, so
    /// the residual gap is positive and stays positive. This is what makes
    /// the discussion genuine: the character wants a resolution finer than
    /// its graph can deliver.
    ///
    /// It is set below what the character's *bluntest split* can resolve, not
    /// merely below the character's own — see [`Self::split`]. If the goal sat
    /// between the two, an instance would be able to close it while the whole
    /// character could not, and that instance would have finished the game its
    /// own character cannot win. The goal is unreachable for every part of the
    /// character, which is the only way "the goal will never be achieved"
    /// survives being split.
    ///
    /// Returns `None` if the region is too thin to have a separation cost at
    /// all — a character that cannot tell anything apart is not a character.
    #[must_use]
    pub fn new(city: &ContactGraph, region: BTreeSet<Position>) -> Option<Self> {
        let graph = induced(city, &region);
        let sep = graph.separation_cost()?;
        // Strictly inside the coarsest reading any split can produce. The
        // fraction is arbitrary; that it is strictly less than `BLUNT` is not.
        let goal = sep * BLUNT * 0.5;
        Some(Self {
            region,
            graph,
            goal,
        })
    }

    /// The resolution the character is reaching for and will not reach.
    #[must_use]
    pub fn goal(&self) -> EdgeWeight {
        self.goal
    }

    /// The moderator's own residual gap (Def. 7.2): what its graph can
    /// separate, less what it is trying to resolve.
    ///
    /// Never zero. That is the point.
    #[must_use]
    pub fn gap(&self) -> Option<EdgeWeight> {
        Some(self.graph.separation_cost()? - self.goal)
    }

    /// Whether this character's goal is out of reach. Always true, and
    /// asserted rather than assumed.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.gap().is_some_and(|g| g > 0.0)
    }

    /// Split into `n` instances, each holding part of what the character can
    /// tell apart.
    ///
    /// Every instance keeps the whole region and every contact in it. What it
    /// loses is **resolution**: some contacts come through blunted, so the
    /// instance can still reach a part of the region it can no longer
    /// separate cleanly. That asymmetry is the deficit — an instance asks
    /// about what it now reads too coarsely, and a sibling that kept that
    /// contact sharp can say something about it.
    ///
    /// It is deliberately *not* a loss of contacts. Dropping edges
    /// disconnects the graph and sends its minimum cut to zero, which is not
    /// an instance that knows less — it is an instance that is no longer in
    /// the region, and its gap goes negative rather than positive. A split
    /// that costs an instance its reach has manufactured absence, not
    /// ignorance.
    ///
    /// Deterministic in `(region, n)` — the same moderator always splits the
    /// same way, so a run is restatable (Cor. 11.14).
    #[must_use]
    pub fn split(&self, n: u32) -> Vec<Instance> {
        if n == 0 {
            return Vec::new();
        }
        let edges: Vec<(Position, Position, EdgeWeight)> = self.graph.edges().collect();
        (0..n)
            .map(|k| {
                let mut g = ContactGraph::new(self.graph.order());
                for (i, (u, v, w)) in edges.iter().enumerate() {
                    // Instance k reads every n-th contact coarsely, offset by
                    // k. What one instance reads blunt, another reads sharp.
                    let blunt = edges.len() > 1 && (i as u32 % n) == k;
                    let _ = g.add_edge(*u, *v, if blunt { w * BLUNT } else { *w });
                }
                Instance {
                    id: InstanceId(k),
                    graph: g,
                    goal: self.goal,
                }
            })
            .collect()
    }
}

/// One split of a moderator: the same character, short of itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    /// Which split this is.
    pub id: InstanceId,
    /// What this instance can tell apart. A subgraph of the moderator's.
    pub graph: ContactGraph,
    /// The goal, inherited unchanged. Instances do not get their own agenda.
    goal: EdgeWeight,
}

impl Instance {
    /// This instance's residual gap.
    ///
    /// Larger than the moderator's whenever the withheld contacts mattered,
    /// which is the information deficit stated as a number.
    #[must_use]
    pub fn gap(&self) -> Option<EdgeWeight> {
        Some(self.graph.separation_cost()? - self.goal)
    }

    /// Whether this instance is worse off than the whole character.
    ///
    /// It is short when it resolves its region *less* finely than the
    /// moderator does — a smaller separation cost, hence a smaller gap
    /// against the same goal. An instance that is not short has nothing to
    /// ask, and a thread of such instances would be decoration.
    #[must_use]
    pub fn is_short_of(&self, m: &Moderator) -> bool {
        match (self.gap(), m.gap()) {
            (Some(a), Some(b)) => a < b,
            _ => false,
        }
    }
}

/// The cap a moderator makes of itself for one region.
///
/// Produced when a user has settled on a voice and the character behind it
/// has to be reassembled. A cap is knowledge deliberately bounded to a
/// region — not everything the moderator has, and not less than the region
/// requires.
#[derive(Debug, Clone, PartialEq)]
pub struct Cap {
    /// The region capped to.
    pub region: BTreeSet<Position>,
    /// The contacts retained, as `(u, v, w)` in city coordinates.
    pub contacts: Vec<(Position, Position, EdgeWeight)>,
}

impl Moderator {
    /// Cap this character's knowledge to `region`.
    ///
    /// Contacts wholly inside the region survive; the rest are dropped. A cap
    /// is therefore *less* than the moderator, which is what makes
    /// amalgamating several of them informative rather than redundant.
    #[must_use]
    pub fn cap(&self, city: &ContactGraph, region: &BTreeSet<Position>) -> Cap {
        let keep: BTreeSet<Position> = self.region.intersection(region).copied().collect();
        let contacts = city
            .edges()
            .filter(|(u, v, _)| keep.contains(u) && keep.contains(v))
            .collect();
        Cap {
            region: keep,
            contacts,
        }
    }
}

/// Combine caps from several moderators into one character.
///
/// This is pruning's first half and it runs the split backwards: instead of
/// one character becoming many instances, many regional caps become one
/// character. The result is the graph a user's chosen voice was speaking
/// from, reassembled from every region it was audible in.
///
/// Where two caps disagree on a contact's weight the **larger** is kept: a
/// distinction one region draws sharply is not blunted by another region that
/// draws it faintly. Coarsening is the safe direction for extraction error
/// (Cor. 8.6), and this is its counterpart — reassembly does not invent
/// contact, and does not discard it either.
///
/// Returns `None` if the caps are jointly empty; there is no character there
/// to build.
#[must_use]
pub fn amalgamate(caps: &[Cap]) -> Option<ContactGraph> {
    let members: BTreeSet<Position> = caps.iter().flat_map(|c| c.region.iter().copied()).collect();
    if members.len() < 2 {
        return None;
    }
    let index: BTreeMap<Position, Position> = members
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i as Position))
        .collect();
    let mut best: BTreeMap<(Position, Position), EdgeWeight> = BTreeMap::new();
    for c in caps {
        for (u, v, w) in &c.contacts {
            let (Some(a), Some(b)) = (index.get(u), index.get(v)) else {
                continue;
            };
            let key = if a < b { (*a, *b) } else { (*b, *a) };
            best.entry(key).and_modify(|x| *x = x.max(*w)).or_insert(*w);
        }
    }
    if best.is_empty() {
        return None;
    }
    let mut g = ContactGraph::new(members.len() as u32);
    for ((u, v), w) in best {
        let _ = g.add_edge(u, v, w);
    }
    Some(g)
}

/// How specific an identity has been made so far.
///
/// Deliberately partial. An identity is a *question about sources* — who is
/// most likely to have produced this character? — and its answer is a
/// solution space, not a person. Attributes are added only when something
/// asks for one.
///
/// This is Theorem 4.3 in the demographics: an attribute that existed before
/// it was asked for would be an attribute that could be retrieved. Here there
/// is nothing to retrieve, because the value does not exist until the
/// question does.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Identity {
    /// Attributes settled so far, in the order they were asked for.
    settled: BTreeMap<String, String>,
}

impl Identity {
    /// An identity with nothing settled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// What has been settled so far.
    #[must_use]
    pub fn settled(&self) -> &BTreeMap<String, String> {
        &self.settled
    }

    /// Whether `key` has been asked for yet.
    #[must_use]
    pub fn has(&self, key: &str) -> bool {
        self.settled.contains_key(key)
    }

    /// How many attributes exist. Starts small and stays small.
    #[must_use]
    pub fn len(&self) -> usize {
        self.settled.len()
    }

    /// Whether nothing has been asked yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.settled.is_empty()
    }

    /// Settle `key`, generating it if this is the first time it was asked.
    ///
    /// Deterministic in `(character, key)`: asking twice yields the same
    /// answer, because the second ask reads what the first settled. The
    /// generated value is drawn from `options` by a hash of the character's
    /// invariant, so two different characters may answer differently and the
    /// same character never contradicts itself.
    ///
    /// Note what this is not: it is not a lookup of a stored profile. Before
    /// the first call there is no value, and no operation could have returned
    /// one.
    pub fn ask(&mut self, character: &ContactGraph, key: &str, options: &[&str]) -> Option<String> {
        if let Some(v) = self.settled.get(key) {
            return Some(v.clone());
        }
        if options.is_empty() {
            return None;
        }
        let chi = character.separation_cost()?;
        // A stable index from the character and the question. Different
        // questions of the same character need not agree.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in key.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        for b in chi.to_bits().to_be_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        let pick = options[(h % options.len() as u64) as usize].to_owned();
        self.settled.insert(key.to_owned(), pick.clone());
        Some(pick)
    }
}

/// A character reassembled from caps, plus whatever identity has been asked.
#[derive(Debug, Clone)]
pub struct Character {
    /// The amalgamated graph. This is the invariant-bearing part.
    pub graph: ContactGraph,
    /// What has been settled about who would have this character.
    pub identity: Identity,
}

impl Character {
    /// Reassemble a character from regional caps.
    #[must_use]
    pub fn amalgamated(caps: &[Cap]) -> Option<Self> {
        Some(Self {
            graph: amalgamate(caps)?,
            identity: Identity::new(),
        })
    }

    /// The character invariant.
    #[must_use]
    pub fn chi(&self) -> Option<EdgeWeight> {
        self.graph.separation_cost()
    }

    /// Prune an agent carrying this character.
    ///
    /// The agent's record starts at zero: it has an invariant but no history,
    /// so nothing distinguishes it from a faithful copy until it acts
    /// (Thm 8.7, Cor. 8.8).
    #[must_use]
    pub fn prune(&self, id: &str) -> Agent {
        Agent::new(id, self.graph.clone())
    }
}

/// The subgraph induced on `members`, renumbered from zero.
fn induced(graph: &ContactGraph, members: &BTreeSet<Position>) -> ContactGraph {
    let index: BTreeMap<Position, Position> = members
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i as Position))
        .collect();
    let mut out = ContactGraph::new(members.len() as u32);
    for (u, v, w) in graph.edges() {
        if let (Some(a), Some(b)) = (index.get(&u), index.get(&v)) {
            let _ = out.add_edge(*a, *b, w);
        }
    }
    out
}

/// A record that has not been touched. Convenience for callers building
/// emitted states for instance posts.
#[must_use]
pub fn fresh() -> Record {
    Record::new()
}

/// A private question the moderator put to one of its instances, and the
/// public answer that came back.
///
/// The asymmetry is the mechanism. The question is never registered in the
/// forum — it has no post, no id, and no reader — because it was addressed to
/// a part of the same character and never left it. The answer is public
/// because that is what a square is. A reader sees replies to nothing, which
/// is an accurate impression: there was no visible question, and there was no
/// second person.
#[derive(Debug, Clone)]
pub struct Turn {
    /// Which instance was asked.
    pub instance: InstanceId,
    /// Where in the region the question was about.
    pub about: Position,
    /// The instance's gap before and after — the acting side.
    pub acting: Gaps,
    /// The moderator's own gap before and after — the receiving side.
    ///
    /// It rises. The moderator asked because it could not close the thing,
    /// and hearing an instance that also cannot close it does not help.
    pub receiving: Gaps,
    /// What the exchange did to the two gaps (Thm 7.5).
    pub act: Act,
}

impl Moderator {
    /// Which position this character is least able to resolve, and therefore
    /// what it will ask about next.
    ///
    /// Gap-driven: the moderator asks where its own separation is thinnest,
    /// not where a script says to go. If it held the closed answer there
    /// would be no such position and nothing to ask.
    #[must_use]
    pub fn wants(&self) -> Option<Position> {
        self.region.iter().copied().min_by(|a, b| {
            let f = |p: Position| {
                let mut part = BTreeSet::new();
                part.insert(p);
                self.graph.cut_weight(&part)
            };
            f(*a)
                .partial_cmp(&f(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Run one round of the moderator questioning its instances.
    ///
    /// For each instance in turn: the moderator asks privately about the
    /// position it least resolves, the instance answers with whatever its own
    /// (poorer) graph can say, and the exchange is classified.
    ///
    /// The round is guaranteed not to close anything. The moderator's gap
    /// after hearing an instance is computed against the same graph it had
    /// before — nothing an instance says can add a contact to the character it
    /// is a part of. That is Theorem 7.4 arriving as arithmetic rather than as
    /// a comment: the conversation continues because it cannot finish.
    #[must_use]
    pub fn round(&self, instances: &[Instance]) -> Vec<Turn> {
        let Some(about) = self.wants() else {
            return Vec::new();
        };
        let Some(mine) = self.gap() else {
            return Vec::new();
        };
        instances
            .iter()
            .filter_map(|inst| {
                let before = inst.gap()?;
                // Being asked is itself informative: the question tells the
                // instance that `about` is worth resolving, and its gap
                // closes to the character's own reading and stops there: no
                // part of a character out-resolves the whole.
                let after = mine;
                let acting = Gaps::new(before, after);
                // The moderator is left worse off. It now holds a statement
                // about a position it still cannot resolve, made by something
                // that resolves it more coarsely than itself; the rise is
                // exactly the deficit the instance was carrying.
                let receiving = Gaps::new(mine, mine + (mine - before));
                Some(Turn {
                    instance: inst.id,
                    about,
                    acting,
                    receiving,
                    act: classify(acting, receiving),
                })
            })
            .collect()
    }
}

/// Post a round into the forum as a thread.
///
/// The first turn opens a thread and the rest reply to it, so a reader sees a
/// discussion. Each post carries the classified act, because unlike the seeded
/// square these gaps were actually measured — fabricating one there would have
/// asserted a verdict the run never computed, and declining to fabricate it
/// here would discard one it did.
///
/// `body` is handed the turn and returns what was said. Nothing in the runtime
/// reads the result.
pub fn post_round(
    forum: &mut Forum,
    speaker_for: impl Fn(InstanceId) -> Speaker,
    turns: &[Turn],
    mut body: impl FnMut(&Turn) -> String,
) -> Vec<PostId> {
    let mut root: Option<PostId> = None;
    let mut out = Vec::new();
    for t in turns {
        let id = forum.register(
            root,
            speaker_for(t.instance),
            body(t),
            at(t.about, Record::new()),
            Some(t.act),
        );
        root.get_or_insert(id);
        out.push(id);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A city with three overlapping regions.
    fn city() -> ContactGraph {
        let mut g = ContactGraph::new(12);
        for u in 0..11 {
            g.add_edge(u, u + 1, if u % 4 == 0 { 1.0 } else { 2.0 })
                .unwrap();
        }
        // A few chords, so regions are not bare paths.
        g.add_edge(0, 3, 1.5).unwrap();
        g.add_edge(4, 7, 1.5).unwrap();
        g.add_edge(8, 11, 1.5).unwrap();
        g
    }

    fn region(lo: Position, hi: Position) -> BTreeSet<Position> {
        (lo..hi).collect()
    }

    #[test]
    fn the_moderator_cannot_close_its_own_goal() {
        let m = Moderator::new(&city(), region(0, 5)).unwrap();
        assert!(
            m.is_open(),
            "an oracle's questioning would be theatre (Thm 7.4)"
        );
        assert!(m.gap().unwrap() > 0.0);
    }

    #[test]
    fn splitting_manufactures_a_deficit() {
        let m = Moderator::new(&city(), region(0, 6)).unwrap();
        let parts = m.split(3);
        assert_eq!(parts.len(), 3);
        // At least one instance is genuinely worse off than the whole
        // character; otherwise nobody has anything to ask.
        assert!(
            parts.iter().any(|i| i.is_short_of(&m)),
            "a split that costs nothing gives nobody a reason to speak"
        );
    }

    #[test]
    fn a_blunted_contact_is_sharp_for_a_sibling() {
        let m = Moderator::new(&city(), region(0, 8)).unwrap();
        let parts = m.split(3);
        // Every instance still reaches the whole region — the split takes
        // resolution, never reach.
        for p in &parts {
            assert!(
                p.graph.is_connected(),
                "an instance that lost its region is absent, not ignorant"
            );
        }
        // And every contact is read sharply by someone: the character is
        // distributed, not diminished.
        for (u, v, w) in m.graph.edges() {
            assert!(
                parts.iter().any(|p| p.graph.weight(u, v) == Some(w)),
                "contact {u}-{v} is blunt for everyone"
            );
        }
    }

    #[test]
    fn an_instances_gap_is_positive_like_the_characters() {
        // A negative gap would say the instance resolves more finely than its
        // boundary requires, which is the opposite of a deficit.
        let m = Moderator::new(&city(), region(0, 8)).unwrap();
        for i in m.split(3) {
            assert!(
                i.gap().unwrap() > 0.0,
                "instance {} closed its goal",
                i.id.0
            );
        }
    }

    #[test]
    fn instances_are_the_same_character_not_different_ones() {
        let m = Moderator::new(&city(), region(0, 6)).unwrap();
        let parts = m.split(3);
        // They share the goal exactly. An instance with its own agenda would
        // be a second character, and the thread would stop being one
        // character talking to itself.
        assert!(parts.iter().all(|i| i.goal == m.goal()));
    }

    #[test]
    fn splitting_is_deterministic() {
        let m = Moderator::new(&city(), region(0, 7)).unwrap();
        let a: Vec<_> = m.split(3).iter().map(|i| i.gap()).collect();
        let b: Vec<_> = m.split(3).iter().map(|i| i.gap()).collect();
        assert_eq!(a, b, "the same character always splits the same way");
    }

    #[test]
    fn two_subgroups_differ_only_by_region() {
        // There is no property of a subgroup beyond which positions it holds.
        // Two moderators over the same region are the same character.
        let c = city();
        let a = Moderator::new(&c, region(0, 5)).unwrap();
        let b = Moderator::new(&c, region(0, 5)).unwrap();
        assert_eq!(a.graph.separation_cost(), b.graph.separation_cost());
        assert_eq!(a.goal(), b.goal());
    }

    #[test]
    fn amalgamating_caps_reassembles_a_character() {
        let c = city();
        let m1 = Moderator::new(&c, region(0, 6)).unwrap();
        let m2 = Moderator::new(&c, region(5, 12)).unwrap();
        // A voice audible in both regions.
        let heard: BTreeSet<Position> = [1, 2, 3, 7, 8, 9].into_iter().collect();
        let caps = vec![m1.cap(&c, &heard), m2.cap(&c, &heard)];
        let ch = Character::amalgamated(&caps).expect("a character to reassemble");
        assert!(ch.chi().is_some(), "the reassembly has an invariant");
    }

    #[test]
    fn amalgamation_keeps_the_sharper_of_two_readings() {
        // Two caps naming the same contact at different weights: the sharper
        // distinction survives, because reassembly does not blunt.
        let region: BTreeSet<Position> = [0, 1].into_iter().collect();
        let faint = Cap {
            region: region.clone(),
            contacts: vec![(0, 1, 1.0)],
        };
        let sharp = Cap {
            region,
            contacts: vec![(0, 1, 4.0)],
        };
        let g = amalgamate(&[faint, sharp]).unwrap();
        assert_eq!(g.weight(0, 1), Some(4.0));
    }

    #[test]
    fn a_character_prunes_to_an_agent_with_no_history() {
        let c = city();
        let m = Moderator::new(&c, region(0, 6)).unwrap();
        let heard: BTreeSet<Position> = [1, 2, 3, 4].into_iter().collect();
        let ch = Character::amalgamated(&[m.cap(&c, &heard)]).unwrap();
        let a = ch.prune("someone");
        assert_eq!(a.record().get(), 0, "not yet an individual (Cor. 8.8)");
        assert_eq!(a.chi(), ch.chi(), "the character is what was pruned");
    }

    #[test]
    fn an_identity_starts_with_nothing_settled() {
        let ch = {
            let c = city();
            let m = Moderator::new(&c, region(0, 6)).unwrap();
            let heard: BTreeSet<Position> = [1, 2, 3].into_iter().collect();
            Character::amalgamated(&[m.cap(&c, &heard)]).unwrap()
        };
        assert!(
            ch.identity.is_empty(),
            "nothing is true of them until something asks"
        );
        assert!(!ch.identity.has("car"));
    }

    #[test]
    fn an_attribute_exists_only_once_it_is_asked_for() {
        let c = city();
        let m = Moderator::new(&c, region(0, 6)).unwrap();
        let heard: BTreeSet<Position> = [1, 2, 3, 4].into_iter().collect();
        let mut ch = Character::amalgamated(&[m.cap(&c, &heard)]).unwrap();

        assert!(!ch.identity.has("grundschule"));
        let first = ch
            .identity
            .ask(&ch.graph, "grundschule", &["Hirschengraben", "Wipkingen"])
            .unwrap();
        assert!(ch.identity.has("grundschule"));

        // Asking again does not re-roll: the character does not contradict
        // itself once something is settled.
        let again = ch
            .identity
            .ask(&ch.graph, "grundschule", &["Hirschengraben", "Wipkingen"])
            .unwrap();
        assert_eq!(first, again);

        // And nothing else was invented along the way.
        assert_eq!(ch.identity.len(), 1, "only what was asked exists");
        assert!(!ch.identity.has("car"));
        assert!(!ch.identity.has("music"));
    }

    #[test]
    fn identity_is_a_solution_space_not_a_person() {
        // The coarse questions can be settled while everything fine remains
        // unasked, which is the intended resting state.
        let c = city();
        let m = Moderator::new(&c, region(0, 8)).unwrap();
        let heard: BTreeSet<Position> = [1, 2, 3, 4, 5].into_iter().collect();
        let mut ch = Character::amalgamated(&[m.cap(&c, &heard)]).unwrap();
        ch.identity.ask(&ch.graph, "age-band", &["30-45", "45-60"]);
        ch.identity
            .ask(&ch.graph, "education", &["apprenticeship", "tertiary"]);
        assert_eq!(ch.identity.len(), 2);
        assert!(
            !ch.identity.has("car") && !ch.identity.has("district"),
            "the rest is generated on demand, or never"
        );
    }

    #[test]
    fn a_round_is_the_character_asking_about_what_it_cannot_resolve() {
        let m = Moderator::new(&city(), region(0, 7)).unwrap();
        let turns = m.round(&m.split(3));
        assert!(!turns.is_empty());
        let about = m.wants().unwrap();
        assert!(
            turns.iter().all(|t| t.about == about),
            "one question per round, and it is the moderator's own weakest point"
        );
        assert!(m.region.contains(&about));
    }

    #[test]
    fn a_round_never_closes_the_moderators_gap() {
        // Thm 7.4, as arithmetic: after hearing every instance the character
        // is no better off, and generally worse.
        let m = Moderator::new(&city(), region(0, 8)).unwrap();
        let mine = m.gap().unwrap();
        for t in m.round(&m.split(3)) {
            assert!(
                t.receiving.after >= mine - 1e-12,
                "an instance told the character something it did not already have"
            );
        }
        assert!(m.is_open(), "still open after the round");
    }

    #[test]
    fn an_instance_that_is_short_asks_rather_than_only_reports() {
        let m = Moderator::new(&city(), region(0, 8)).unwrap();
        let parts = m.split(3);
        let turns = m.round(&parts);
        // Where an instance genuinely lacked something, the exchange raises
        // the receiving gap: that is `question` in the sense of Thm 7.5, and
        // it is the reason the thread continues.
        let questioning = turns.iter().filter(|t| t.act.question).count();
        let short = parts.iter().filter(|i| i.is_short_of(&m)).count();
        assert_eq!(
            questioning, short,
            "exactly the deficient instances turn the exchange into a question"
        );
    }

    #[test]
    fn an_inert_turn_is_a_permitted_outcome() {
        // An instance holding everything the moderator holds moves neither
        // gap. That is a value, not a failure (Thm 7.4).
        let m = Moderator::new(&city(), region(0, 6)).unwrap();
        let whole = m.split(1);
        let turns = m.round(&whole);
        assert_eq!(turns.len(), 1);
        assert!(turns[0].act.is_inert() || turns[0].act.question);
    }

    #[test]
    fn a_round_reads_as_a_thread_with_no_visible_question() {
        use crate::forum::Forum;
        let m = Moderator::new(&city(), region(0, 7)).unwrap();
        let turns = m.round(&m.split(3));
        let mut forum = Forum::default();
        let ids = post_round(
            &mut forum,
            |i| Speaker::Agent(format!("instance-{}", i.0)),
            &turns,
            |t| format!("about {}", t.about),
        );
        assert_eq!(ids.len(), turns.len());
        // One root, the rest replies: a discussion.
        assert_eq!(forum.thread(ids[0]).len(), ids.len());
        // And nothing in the forum is the question. The moderator's side of
        // it was never registered, because it never left the character.
        assert_eq!(
            forum.posts().len(),
            ids.len(),
            "the private question has no post"
        );
        // Every post carries a measured act, unlike the seeded square.
        assert!(forum.posts().iter().all(|p| p.act.is_some()));
    }
}
