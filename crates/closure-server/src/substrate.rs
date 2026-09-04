//! The city substrate: regions, characters, and what the square says.
//!
//! Everything here is coarse by construction. A subgroup is a set of
//! positions with a label, and the [`Moderator`] over it is one character
//! with an incomplete graph — not a roster, not a catalogue, and not derived
//! from any identified individual.
//!
//! ## Why there is no population here any more
//!
//! There was a list of modules a voice could be matched against. Matching a
//! voice to an entry in a list is retrieval whatever the entries are called,
//! and Theorem 4.3 says no operation has that signature. What replaced it is
//! not a better list: it is the absence of one. A voice is a split of the
//! character talking in its region, and the character behind a voice is built
//! by capping and amalgamating those moderators — from where the voice
//! actually spoke, never from a table.
//!
//! ## Why the bodies are templates
//!
//! The utterance text is generated from the voice handle and the region.
//! That is deliberately thin: nothing in the runtime reads a post's body —
//! not visibility, not the act classifier, not the feed order. Text that
//! carried meaning the mechanism then acted on would be a fifth operation
//! (§11), and there is no fifth. When a language model is attached later it
//! replaces this function and nothing else, because nothing else consumes
//! what it produces.

use closure_kernel::ContactGraph;
use closure_runtime::Square;
use closure_runtime::moderator::Moderator;
use closure_runtime::voice::{Subgroup, Utterance};

/// Zürich's regions, in the graph the city is cut from.
///
/// Positions are contiguous per region so that a module spanning a region is
/// a subset test rather than a lookup. The regions overlap where the city
/// does: a lake position sits in both watersports and lake transport, which
/// is why a voice can be heard in both without that meaning anything.
const REGIONS: &[(&str, u32, u32)] = &[
    ("watersports", 0, 6),
    ("lake-transport", 4, 10),
    ("carbon-composites", 10, 15),
    ("eth-materials", 13, 19),
    ("commuting", 19, 26),
    ("extreme-sports", 26, 31),
];

/// Total positions in the city graph.
pub const ORDER: u32 = 31;

/// The regions of `city`.
///
/// Only Zürich is modelled. An unknown city gets the same regions rather than
/// none, because an empty square would leave a player nothing to read and
/// therefore no route to anyone that is not description.
#[must_use]
pub fn subgroups(_city: &str) -> Vec<Subgroup> {
    REGIONS
        .iter()
        .map(|(name, lo, hi)| Subgroup {
            name: (*name).to_owned(),
            members: (*lo..*hi).collect(),
        })
        .collect()
}

/// The shared contact graph the city is cut from.
///
/// A path through the positions, with weaker contacts where regions meet.
/// The weak joins matter: they are where separation cost is cheapest to cut,
/// so a pruned agent that stops at a region boundary is the ordinary case
/// rather than a special one.
#[must_use]
pub fn city_graph() -> ContactGraph {
    let mut g = ContactGraph::new(ORDER);
    for u in 0..ORDER - 1 {
        // Region boundaries are the thin contacts.
        let boundary = REGIONS.iter().any(|(_, lo, _)| *lo == u + 1);
        let w = if boundary { 1.0 } else { 2.0 };
        let _ = g.add_edge(u, u + 1, w);
    }
    g
}

/// The characters talking in `city`: one moderator per region.
///
/// This is what replaced a population catalogue. A catalogue listed profiles
/// a voice could be matched against, which is a lookup table however it is
/// dressed — the retrieval Theorem 4.3 denies. A moderator is not a list of
/// anyone; it is one character with an incomplete graph, and the voices in
/// its region are its own splits.
///
/// A region too thin to have a separation cost yields no moderator, and none
/// is invented for it.
#[must_use]
pub fn moderators(city: &str) -> Vec<Moderator> {
    let g = city_graph();
    subgroups(city)
        .into_iter()
        .filter_map(|s| Moderator::new(&g, s.members))
        .collect()
}

/// What a seeded utterance says.
///
/// A placeholder body naming the region and the voice. Nothing in the
/// runtime reads it — see the module documentation on why that is a property
/// worth keeping rather than a gap to fill.
#[must_use]
pub fn utterance_body(square: &Square, u: &Utterance) -> String {
    let region = square
        .subgroups
        .iter()
        .find(|s| s.contains(u.terminus))
        .map_or("the square", |s| s.name.as_str());
    format!("[{region}] voice {} speaking at {}", u.voice.0, u.terminus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_overlap_where_the_city_does() {
        let subs = subgroups("zuerich");
        let water = &subs[0];
        let lake = &subs[1];
        assert!(
            water.members.intersection(&lake.members).next().is_some(),
            "the lake is in both, so a voice in both means nothing special"
        );
    }

    #[test]
    fn every_region_has_a_character_that_cannot_close_its_goal() {
        let mods = moderators("zuerich");
        assert!(!mods.is_empty());
        for m in &mods {
            assert!(
                m.is_open(),
                "a moderator that closed its goal would be an oracle (Thm 7.4)"
            );
        }
    }

    #[test]
    fn a_moderator_is_not_a_roster() {
        // The only thing a moderator holds is a region and a graph over it.
        // There is nowhere for a list of people to be.
        for m in moderators("zuerich") {
            assert!(m.region.len() > 1);
            assert_eq!(m.graph.order() as usize, m.region.len());
        }
    }

    #[test]
    fn splitting_a_region_character_gives_it_someone_to_ask() {
        let m = &moderators("zuerich")[0];
        let parts = m.split(3);
        assert!(
            parts.iter().any(|i| i.is_short_of(m)),
            "a split that costs nothing gives nobody a reason to speak"
        );
        assert!(!m.round(&parts).is_empty());
    }

    #[test]
    fn the_city_graph_covers_every_position() {
        let g = city_graph();
        assert_eq!(g.order(), ORDER);
        for (_, _, hi) in REGIONS {
            assert!(*hi <= ORDER);
        }
    }
}
