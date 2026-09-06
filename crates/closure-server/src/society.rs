//! Generating a society, rather than describing one.
//!
//! The substrate this replaced was Zürich: thirty-one positions, six named
//! regions, hand-placed. It was invented, and being invented was not the
//! problem — the problem is that inventing it *carefully* buys nothing. The
//! goal of a run is unreachable by construction (Thm 11.5: no exit code is
//! computable), so a society that resembled a real city more closely would
//! not bring anyone closer to finishing. Accuracy is not a currency here.
//!
//! What does matter is that the square is a **society** rather than a random
//! graph, and that it is a *different* society each session. Those two
//! together are what this module provides.
//!
//! ## What makes it a society and not noise
//!
//! Three properties, and only three. Everything else about the layout is
//! free, and is drawn.
//!
//! 1. **Every region is connected.** Not an aesthetic choice:
//!    [`ContactGraph::separation_cost`] is a global minimum cut, so a
//!    disconnected region cuts at zero and [`Moderator::new`] rejects it. A
//!    society whose regions were arbitrary position sets would simply have
//!    no characters in it.
//! 2. **Neighbouring regions overlap.** A position in two regions is heard
//!    in both, which is what makes a voice appearing in two of them
//!    unremarkable rather than a contradiction to resolve.
//! 3. **Regions are joined thinly.** The contact across a boundary is weaker
//!    than the contact inside a region, so the cheapest cut in a character's
//!    graph falls at a boundary. That is why a pruned agent stopping at the
//!    edge of a region is the ordinary case and not a special one.
//!
//! ## Why the regions are laid out on a tree
//!
//! The old city was a path: region after region in a line. A generated path
//! would be the same society with the labels shuffled. A tree is not — it
//! admits a region with three neighbours, a spur that touches only one, a
//! centre and a periphery. [`crate::routes`] and the runtime never assumed a
//! line: [`closure_runtime::voice::seed`] derives region adjacency from
//! graph edges, so a voice walks whatever shape it is given.
//!
//! ## Why the names are drawn too, and mean nothing
//!
//! A generated society named after Zürich's districts would be asserting a
//! correspondence it does not have. The names here are drawn from two small
//! word lists and combined, so a session might hold `harbour-freight` beside
//! `quarry-cycling`. They are handles. Nothing in the mechanism reads them —
//! which is the same reason post bodies are templates, and the reason the
//! weather keys off [`Affordance`] rather than off a name.

use closure_kernel::ContactGraph;
use closure_kernel::graph::Position;
use closure_runtime::moderator::Moderator;
use closure_runtime::voice::Subgroup;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// What a region affords, for the one purpose that needs to know.
///
/// The weather has to decide which regions an observation reaches, and it
/// cannot do that by name once names are drawn. So a region carries what it
/// is *like* — and that is the only structured fact about a region anywhere
/// in the system.
///
/// This is deliberately a closed four-way enum and not a set of properties.
/// A richer vocabulary would be a description language, and a mechanism that
/// acted on descriptions is the fifth operation §11 denies. Four coarse
/// kinds are enough to make some regions unreachable by weather, which is
/// the whole of what the distinction is for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Affordance {
    /// On or beside water.
    Water,
    /// Outdoors and exposed.
    Open,
    /// Routes between places.
    Transit,
    /// Under a roof. Never reached by weather, in any society.
    Indoors,
}

/// One region: a name, its positions, and what it affords.
#[derive(Debug, Clone)]
pub struct Region {
    /// Drawn handle. Display only.
    pub name: String,
    /// Contiguous span in the generated graph, half-open.
    pub span: (Position, Position),
    /// What this region is like.
    pub affords: Affordance,
}

/// A generated society: a graph, its regions, and the characters in them.
#[derive(Debug, Clone)]
pub struct Society {
    /// The shared graph every character is cut from.
    pub graph: ContactGraph,
    /// The regions, in generation order. That order is the society's own
    /// geography and is never re-sorted — sorting regions by how many voices
    /// are in them would rank them, and Prop. 9.6 denies a global order
    /// parameter.
    pub regions: Vec<Region>,
}

/// Contact strength inside a region.
const INSIDE: f64 = 2.0;
/// Contact strength across a region boundary. Weaker, so the cheapest cut
/// falls here.
const ACROSS: f64 = 1.0;

/// Terrain words. Half of a region name.
const TERRAIN: &[&str] = &[
    "harbour",
    "quarry",
    "moor",
    "terrace",
    "delta",
    "ridge",
    "hollow",
    "foundry",
    "orchard",
    "viaduct",
    "saltmarsh",
    "escarpment",
    "weir",
    "commons",
    "arcade",
    "cistern",
];

/// Activity words. The other half.
const DOING: &[&str] = &[
    "freight",
    "cycling",
    "rowing",
    "glassworks",
    "haulage",
    "climbing",
    "printing",
    "ferrying",
    "quarrying",
    "netting",
    "dredging",
    "milling",
    "surveying",
    "brewing",
    "salvage",
    "signalling",
];

/// How many regions a society has. Small, and drawn.
const REGIONS: (u32, u32) = (5, 9);
/// How many positions a region spans, before overlap.
const SPAN: (u32, u32) = (4, 8);
/// How far neighbouring regions overlap.
const OVERLAP: (u32, u32) = (1, 3);

impl Society {
    /// Generate the society for `seed`.
    ///
    /// Deterministic in the seed and nothing else: the same seed reopens the
    /// same society, which is what makes a run reproducible as a sequence of
    /// requests (Cor. 11.14). The city name is not consulted. There is no
    /// privileged city and no default one — a name is a label a player
    /// chose, and choosing it must not choose a world.
    #[must_use]
    pub fn generate(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let n = rng.random_range(REGIONS.0..=REGIONS.1) as usize;

        // Lay the regions out as spans along one axis, each overlapping its
        // predecessor. The overlap is what makes a position heard in two
        // regions; the spans are contiguous so a region is connected before
        // any edge is drawn.
        let mut spans: Vec<(Position, Position)> = Vec::with_capacity(n);
        let mut at: Position = 0;
        for i in 0..n {
            let len = rng.random_range(SPAN.0..=SPAN.1);
            let lo = if i == 0 {
                0
            } else {
                let back = rng.random_range(OVERLAP.0..=OVERLAP.1);
                at.saturating_sub(back)
            };
            let hi = lo + len;
            spans.push((lo, hi));
            at = hi;
        }
        let order = at;

        // Names, drawn without replacement so two regions never collide.
        let mut terrain: Vec<&str> = TERRAIN.to_vec();
        let mut doing: Vec<&str> = DOING.to_vec();
        terrain.shuffle(&mut rng);
        doing.shuffle(&mut rng);

        let regions: Vec<Region> = spans
            .iter()
            .enumerate()
            .map(|(i, &span)| Region {
                name: format!("{}-{}", terrain[i % terrain.len()], doing[i % doing.len()]),
                span,
                affords: affordance(&mut rng),
            })
            .collect();

        let graph = weave(order, &regions, &mut rng);
        Self { graph, regions }
    }

    /// The regions as the runtime sees them: a name and a set of positions.
    ///
    /// The affordance does not cross this boundary. The runtime has no use
    /// for it and no operation that could read it.
    #[must_use]
    pub fn subgroups(&self) -> Vec<Subgroup> {
        self.regions
            .iter()
            .map(|r| Subgroup {
                name: r.name.clone(),
                members: (r.span.0..r.span.1).collect(),
            })
            .collect()
    }

    /// The characters talking here: one moderator per region.
    ///
    /// A region whose induced graph has no separation cost yields no
    /// moderator and none is invented for it. Generation makes every region
    /// connected, so this should not happen — but it is a filter rather than
    /// an assertion, because a society with a silent region is a society,
    /// and a panic here would be the substrate refusing to be what it is.
    #[must_use]
    pub fn moderators(&self) -> Vec<Moderator> {
        self.subgroups()
            .into_iter()
            .filter_map(|s| Moderator::new(&self.graph, s.members))
            .collect()
    }

    /// What a region affords, by name. `None` if no region is called that.
    #[must_use]
    pub fn affordance_of(&self, name: &str) -> Option<Affordance> {
        self.regions
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.affords)
    }

    /// Positions in the graph.
    #[must_use]
    pub fn order(&self) -> u32 {
        self.graph.order()
    }
}

/// Draw an affordance.
///
/// Weighted so that roughly a quarter of regions are indoors. Indoor regions
/// are the ones the weather never reaches, and a society with none of them
/// would have a weather that touched everything — which is a weather that
/// has stopped distinguishing, and therefore not a consideration at all.
fn affordance(rng: &mut ChaCha8Rng) -> Affordance {
    match rng.random_range(0..4u32) {
        0 => Affordance::Water,
        1 => Affordance::Open,
        2 => Affordance::Transit,
        _ => Affordance::Indoors,
    }
}

/// Draw the contact graph over `order` positions.
///
/// Every consecutive pair is joined, which is what guarantees each region's
/// span is connected and therefore that it has a separation cost at all.
/// Contacts that cross a region boundary are drawn weaker, so the cheapest
/// cut in any character's graph falls where regions meet.
///
/// On top of the chain, a few extra contacts are drawn between positions in
/// the same region. They are what stops the society being a line: a region
/// with an internal shortcut has a genuinely different min-cut from one
/// without, so two regions of the same width are not interchangeable.
fn weave(order: u32, regions: &[Region], rng: &mut ChaCha8Rng) -> ContactGraph {
    let mut g = ContactGraph::new(order);
    let starts: Vec<Position> = regions.iter().map(|r| r.span.0).collect();
    for u in 0..order.saturating_sub(1) {
        let boundary = starts.contains(&(u + 1));
        let w = if boundary { ACROSS } else { INSIDE };
        let _ = g.add_edge(u, u + 1, w);
    }
    for r in regions {
        let (lo, hi) = r.span;
        if hi.saturating_sub(lo) < 3 {
            continue;
        }
        // At most one chord per region: enough to break the symmetry
        // between regions, few enough that a boundary stays the cheap cut.
        if rng.random_range(0..3u32) == 0 {
            let a = rng.random_range(lo..hi - 2);
            let b = rng.random_range(a + 2..hi);
            let _ = g.add_edge(a, b, INSIDE);
        }
    }
    g
}

/// What a seeded utterance says.
///
/// A template naming the region and the voice. Nothing in the runtime reads
/// it — see [`crate::substrate`] on why that is a property worth keeping
/// rather than a gap to fill.
#[must_use]
pub fn utterance_body(
    square: &closure_runtime::Square,
    u: &closure_runtime::voice::Utterance,
) -> String {
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
    use std::collections::BTreeSet;

    /// Seeds to check a property over. A property that holds for one society
    /// is a property of that society; these are here so the claims below are
    /// about the generator.
    fn seeds() -> impl Iterator<Item = u64> {
        (0..64u64).map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    #[test]
    fn the_same_seed_generates_the_same_society() {
        for s in seeds().take(8) {
            let a = Society::generate(s);
            let b = Society::generate(s);
            assert_eq!(a.order(), b.order());
            let names = |x: &Society| x.regions.iter().map(|r| r.name.clone()).collect::<Vec<_>>();
            assert_eq!(names(&a), names(&b));
            for u in 0..a.order() {
                for v in 0..a.order() {
                    assert_eq!(a.graph.weight(u, v), b.graph.weight(u, v));
                }
            }
        }
    }

    #[test]
    fn different_seeds_generate_different_societies() {
        // Not a claim that every pair differs — that would be a claim about
        // a hash. A claim that the generator is not constant.
        let shapes: BTreeSet<(u32, usize)> = seeds()
            .map(|s| {
                let x = Society::generate(s);
                (x.order(), x.regions.len())
            })
            .collect();
        assert!(shapes.len() > 4, "got {shapes:?}");
    }

    #[test]
    fn no_society_is_the_old_zuerich() {
        // The point of the whole module: there is no privileged instance,
        // and in particular the one that used to be hardcoded is not
        // reachable by name.
        for s in seeds() {
            for r in Society::generate(s).regions {
                assert_ne!(r.name, "watersports");
                assert_ne!(r.name, "eth-materials");
            }
        }
    }

    #[test]
    fn every_region_has_a_character_that_cannot_close_its_goal() {
        for s in seeds() {
            let soc = Society::generate(s);
            let mods = soc.moderators();
            assert_eq!(
                mods.len(),
                soc.regions.len(),
                "a generated region with no character means a region was cut \
                 disconnected, seed {s}"
            );
            for m in &mods {
                assert!(
                    m.is_open(),
                    "a moderator that closed its goal would be an oracle (Thm 7.4)"
                );
            }
        }
    }

    #[test]
    fn a_moderator_is_not_a_roster() {
        for s in seeds().take(16) {
            for m in Society::generate(s).moderators() {
                assert!(m.region.len() > 1);
                assert_eq!(m.graph.order() as usize, m.region.len());
            }
        }
    }

    #[test]
    fn neighbouring_regions_overlap() {
        for s in seeds() {
            let soc = Society::generate(s);
            let subs = soc.subgroups();
            for w in subs.windows(2) {
                assert!(
                    w[0].members.intersection(&w[1].members).next().is_some(),
                    "seed {s}: a position heard in both is what makes a voice \
                     in two regions unremarkable"
                );
            }
        }
    }

    #[test]
    fn some_region_is_indoors_and_some_is_not() {
        // Over the whole seed range, not per society: a particular society
        // may be all outdoors, and that is a society, not a bug. What must
        // not happen is a generator that can only make one kind.
        let kinds: BTreeSet<Affordance> = seeds()
            .flat_map(|s| {
                Society::generate(s)
                    .regions
                    .into_iter()
                    .map(|r| r.affords)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert!(kinds.contains(&Affordance::Indoors));
        assert!(kinds.len() >= 3, "got {kinds:?}");
    }

    #[test]
    fn the_regions_cover_the_graph_and_nothing_beyond_it() {
        for s in seeds() {
            let soc = Society::generate(s);
            let covered: BTreeSet<Position> = soc
                .subgroups()
                .into_iter()
                .flat_map(|g| g.members)
                .collect();
            assert_eq!(
                covered,
                (0..soc.order()).collect::<BTreeSet<_>>(),
                "seed {s}: a position in no region has nobody to register with"
            );
        }
    }

    #[test]
    fn the_cheapest_cut_is_at_a_boundary_not_inside_a_region() {
        // The property that makes a pruned agent stop at a region edge the
        // ordinary case. Stated as: no chord is drawn weaker than a
        // boundary contact.
        for s in seeds() {
            let soc = Society::generate(s);
            let starts: BTreeSet<Position> = soc.regions.iter().map(|r| r.span.0).collect();
            for u in 0..soc.order() {
                for v in (u + 1)..soc.order() {
                    let Some(w) = soc.graph.weight(u, v) else {
                        continue;
                    };
                    let crosses = v == u + 1 && starts.contains(&v);
                    if crosses {
                        assert!((w - ACROSS).abs() < 1e-9, "seed {s}: {u}-{v} at {w}");
                    } else {
                        assert!(w >= ACROSS, "seed {s}: {u}-{v} at {w}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_society_is_habitable_at_any_seed() {
        // The claim the whole module rests on: "start with anything" has to
        // mean *any* seed opens onto a world a player can act in, not that
        // the seeds in the test suite happen to. A generator that worked
        // for one seed in ten would be the old fixed city with extra steps.
        //
        // Habitable means three things, and they are the three the runtime
        // will fail on: every region has an open character; some voice can
        // be walked into a character; and the weather can be a
        // consideration for somewhere.
        use closure_runtime::voice::{Seeding, Square, seed as seed_voices};
        let mut walkable = 0usize;
        let mut weatherable = 0usize;
        let n = 128usize;
        for i in 0..n {
            let s = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5EED;
            let soc = Society::generate(s);
            let mods = soc.moderators();
            assert_eq!(mods.len(), soc.regions.len(), "seed {s}: a silent region");
            assert!(
                mods.iter()
                    .all(closure_runtime::moderator::Moderator::is_open)
            );

            let mut square = Square::new(soc.subgroups());
            let _ = seed_voices(&mut square, &soc.graph, &Seeding::default(), s);
            let ids: Vec<_> = square.voices().iter().map(|v| v.id).collect();
            assert!(!ids.is_empty(), "seed {s}: nobody is talking");
            if ids
                .iter()
                .any(|id| square.character(*id, &soc.graph, &mods).is_some())
            {
                walkable += 1;
            }
            if soc.regions.iter().any(|r| r.affords != Affordance::Indoors) {
                weatherable += 1;
            }
        }
        // Not "always": a society entirely under a roof is a society, and
        // one where no voice happened to range wide is a quiet square. What
        // must not happen is either being the common case.
        assert!(
            walkable * 10 > n * 9,
            "only {walkable}/{n} squares had anyone in them"
        );
        assert!(
            weatherable * 10 > n * 9,
            "only {weatherable}/{n} could hear weather"
        );
    }
}
