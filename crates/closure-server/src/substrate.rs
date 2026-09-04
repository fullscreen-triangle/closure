//! The city substrate: regions, modules, and what the square says.
//!
//! Everything here is coarse by construction. A module is a pattern of
//! disposition and never a person (see [`closure_runtime::population`]), and
//! a subgroup is a set of positions with a label. Neither is derived from any
//! identified individual, and no combination of them recovers one.
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
use closure_runtime::population::Module;
use closure_runtime::voice::{Subgroup, Utterance};
use closure_runtime::{Population, Square};
use std::collections::BTreeSet;

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

/// The coarse population of `city`.
///
/// One module per region, plus three that span several. The spanning modules
/// are what make a fit non-unique in the ordinary case: a voice heard only in
/// watersports is consistent with the swimmer, the sailing engineer, and the
/// generalist alike, and the square does not choose between them.
#[must_use]
pub fn population(_city: &str) -> Population {
    let region = |name: &str| -> BTreeSet<u32> {
        REGIONS
            .iter()
            .find(|(n, _, _)| *n == name)
            .map_or_else(BTreeSet::new, |(_, lo, hi)| (*lo..*hi).collect())
    };
    let union =
        |names: &[&str]| -> BTreeSet<u32> { names.iter().flat_map(|n| region(n)).collect() };

    let mut modules: Vec<Module> = REGIONS
        .iter()
        .map(|(name, lo, hi)| Module {
            name: (*name).to_owned(),
            members: (*lo..*hi).collect(),
            matches: Vec::new(),
        })
        .collect();

    modules.push(Module {
        name: "sailing-engineer".to_owned(),
        members: union(&["watersports", "carbon-composites", "eth-materials"]),
        matches: Vec::new(),
    });
    modules.push(Module {
        name: "lake-commuter".to_owned(),
        members: union(&["lake-transport", "commuting"]),
        matches: Vec::new(),
    });
    modules.push(Module {
        name: "board-rider".to_owned(),
        members: union(&["watersports", "extreme-sports"]),
        matches: Vec::new(),
    });
    modules.push(Module {
        name: "generalist".to_owned(),
        members: (0..ORDER).collect(),
        matches: Vec::new(),
    });

    Population {
        modules,
        shared: city_graph(),
    }
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
    fn a_region_voice_admits_several_profiles() {
        // A module set where a single-region footprint is genuinely
        // ambiguous: this is Prop. 3.4 arriving in the substrate rather than
        // being arranged in a test.
        let pop = population("zuerich");
        let foot: BTreeSet<u32> = [1u32].into_iter().collect();
        let fits: Vec<&str> = pop
            .modules
            .iter()
            .filter(|m| foot.is_subset(&m.members))
            .map(|m| m.name.as_str())
            .collect();
        assert!(
            fits.len() > 1,
            "one post should not single out a person: {fits:?}"
        );
    }

    #[test]
    fn no_module_is_a_person() {
        // Every module is defined by a region, never by an attribute
        // constraint identifying anyone.
        for m in population("zuerich").modules {
            assert!(
                m.matches.is_empty(),
                "module {} carries an identifying constraint",
                m.name
            );
            assert!(
                m.members.len() > 1,
                "module {} is a single position",
                m.name
            );
        }
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
