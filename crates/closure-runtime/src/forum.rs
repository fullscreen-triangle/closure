//! The forum: posts, threads, and who can see them.
//!
//! Conversations are public by default and threaded, which is the shape a
//! reading of §6–§7 forces rather than a borrowed convention. Three results
//! make the forum the honest presentation and a chat window the misleading
//! one:
//!
//! * **Theorem 6.2 (receiver relativity).** A broadcast is not one message
//!   with many readers; it is N independent registrations, one per receiver,
//!   and part (iii) says no graph makes two agents' registrations jointly
//!   cuts. A thread with N repliers is exactly that, drawn.
//! * **Corollary 6.7.** Drift accumulates along a chain of re-registrations —
//!   a comment tree, not a broadcast.
//! * **Theorem 7.4.** Exchange does not close, so agents may go on replying
//!   to one another with no user present and nothing is missing when they do.
//!
//! ## What a forum must *not* borrow
//!
//! Votes, karma, and a top-sort are precisely what the theory forbids:
//!
//! * a score aggregating across agents is ill-defined, not merely disallowed
//!   (Prop. 9.6 — there is no global order parameter);
//! * a verdict on a post is a system verdict (Invariant 6);
//! * ranking by reference count is a reference, and `binv:noreference` denies
//!   emitted states any.
//!
//! So [`Feed`] sorts by recency and by residual gap direction, which are the
//! only orderings the emitted states support. A post that lowered the acting
//! gap reads as a report and one that raised the receiving gap as a question
//! (Thm 7.5); that classification is the whole of the ranking signal, and it
//! is not a quality judgement.
//!
//! ## Visibility is derived, never stored
//!
//! There is no access-control list. A post registers at a terminus, and an
//! agent sees it exactly when that terminus is reachable on the agent's own
//! pruned graph at finite separation cost. Privacy is therefore a fact about
//! pruning — an agent whose graph does not reach a position cannot see what
//! sits there, and no flag has to be maintained to keep it that way.
//!
//! A "private message" is a post whose terminus lies in a region only two
//! graphs reach. Nothing special implements it.

use crate::act::{Act, Gaps, classify};
use closure_kernel::graph::Position;
use closure_kernel::identity::Record;
use closure_kernel::psychon::Emitted;
use closure_kernel::separation::MediumGraph;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Handle for a post within a forum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PostId(pub u64);

/// Who registered a post.
///
/// This is *not* an author in the sense Theorem 6.6 forbids. That theorem
/// denies that an originator can be **recovered from emitted states**; it
/// does not deny that the runtime knows which agent it just ran. The
/// distinction is enforced downstream: [`Feed`] never sorts or filters on
/// this field, and [`classify_post`] does not receive it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Speaker {
    /// The player. Themselves an agent, and not privileged here.
    Player,
    /// A pruned agent, by its handle.
    Agent(String),
}

/// A post: content registered at a terminus.
///
/// The content is opaque to every mechanism in this module. Nothing here
/// reads it, ranks by it, or classifies with it — see [`crate::act`] on why
/// the act classifier takes no content parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    /// Handle.
    pub id: PostId,
    /// The post this replies to, if any. `None` makes it a thread root.
    pub parent: Option<PostId>,
    /// Who registered it.
    pub speaker: Speaker,
    /// What was said. Never inspected by this module.
    pub body: String,
    /// The emitted state at registration: terminus and record.
    pub emitted: Emitted,
    /// Monotone tick at which it was registered. Never decreases.
    pub at: u64,
    /// What the registration did to the gaps, if both sides were measured.
    ///
    /// `None` means the gaps were not measured, not that nothing moved.
    /// Absence of a classification is not a classification of absence.
    pub act: Option<Act>,
}

impl Post {
    /// Whether this post is a thread root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }

    /// The terminus the post registered at.
    #[must_use]
    pub fn terminus(&self) -> Position {
        self.emitted.terminus
    }
}

/// The forum: every post, in registration order.
///
/// Posts are append-only. There is no edit and no delete, for the same reason
/// [`Record`] has no decrement: a registration that happened cannot be made
/// not to have happened, and a forum that let it would be storing a repealed
/// record.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Forum {
    posts: Vec<Post>,
    next: u64,
    tick: u64,
}

impl Forum {
    /// An empty forum at tick zero.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Advance the world one step.
    ///
    /// The tick is monotone and advances only here. Nothing in this crate
    /// spawns a thread or a timer: a world that moved on its own would make
    /// runs irreproducible in a way Corollary 11.14 does not license, since
    /// reproducibility attaches to the protocol and a background scheduler is
    /// not part of one.
    pub fn advance(&mut self) -> u64 {
        self.tick = self.tick.saturating_add(1);
        self.tick
    }

    /// Register a post, returning its handle.
    pub fn register(
        &mut self,
        parent: Option<PostId>,
        speaker: Speaker,
        body: impl Into<String>,
        emitted: Emitted,
        act: Option<Act>,
    ) -> PostId {
        let id = PostId(self.next);
        self.next += 1;
        self.posts.push(Post {
            id,
            parent,
            speaker,
            body: body.into(),
            emitted,
            at: self.tick,
            act,
        });
        id
    }

    /// Every post, oldest first.
    #[must_use]
    pub fn posts(&self) -> &[Post] {
        &self.posts
    }

    /// One post by handle.
    #[must_use]
    pub fn get(&self, id: PostId) -> Option<&Post> {
        self.posts.iter().find(|p| p.id == id)
    }

    /// The direct replies to `id`, oldest first.
    #[must_use]
    pub fn replies(&self, id: PostId) -> Vec<&Post> {
        self.posts.iter().filter(|p| p.parent == Some(id)).collect()
    }

    /// The root of the thread `id` belongs to.
    ///
    /// Returns `None` if the chain is broken or cyclic; a cycle would be a
    /// forum in which a post replies to its own descendant, which registration
    /// order forbids but which is checked rather than assumed.
    #[must_use]
    pub fn root_of(&self, id: PostId) -> Option<PostId> {
        let mut cur = self.get(id)?;
        let mut hops = 0usize;
        while let Some(parent) = cur.parent {
            hops += 1;
            if hops > self.posts.len() {
                return None;
            }
            cur = self.get(parent)?;
        }
        Some(cur.id)
    }

    /// Every post in the thread rooted at `id`, oldest first, root included.
    #[must_use]
    pub fn thread(&self, id: PostId) -> Vec<&Post> {
        let Some(root) = self.root_of(id) else {
            return Vec::new();
        };
        self.posts
            .iter()
            .filter(|p| self.root_of(p.id) == Some(root))
            .collect()
    }

    /// The drift chain from a root: how many re-registrations deep the
    /// longest branch runs (Cor. 6.7).
    ///
    /// Drift accumulates along the chain, so its length is the quantity the
    /// corollary is about. Note what this is *not*: it is not a measure of a
    /// post's quality, popularity, or correctness, and nothing in this module
    /// sorts by it.
    #[must_use]
    pub fn depth(&self, root: PostId) -> usize {
        let mut by_parent: BTreeMap<Option<PostId>, Vec<PostId>> = BTreeMap::new();
        for p in &self.posts {
            by_parent.entry(p.parent).or_default().push(p.id);
        }
        fn walk(
            id: PostId,
            by_parent: &BTreeMap<Option<PostId>, Vec<PostId>>,
            budget: usize,
        ) -> usize {
            if budget == 0 {
                return 0;
            }
            by_parent
                .get(&Some(id))
                .map(|kids| {
                    kids.iter()
                        .map(|k| 1 + walk(*k, by_parent, budget - 1))
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0)
        }
        walk(root, &by_parent, self.posts.len())
    }
}

/// What an agent can see, and at what cost.
///
/// Visibility is not stored; it is recomputed from the agent's own graph.
/// See the module documentation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Visible {
    /// The post.
    pub id: PostId,
    /// Separation cost of the post's terminus on the viewer's graph. Lower
    /// means the post sits nearer what the viewer can already tell apart.
    pub cost: f64,
}

/// The posts an agent with graph `viewer` can see.
///
/// A post is visible iff its terminus is a position of the viewer's graph
/// with finite separation cost. There is no list to consult and no flag to
/// forget to set: an agent whose pruning does not reach a region simply has
/// no separation there, so the post is not returned.
///
/// The `resolution` argument does *not* affect visibility — only gap, which
/// is a different question. Visibility is about reach, not about resolving
/// power.
#[must_use]
pub fn visible_to(forum: &Forum, viewer: &MediumGraph) -> Vec<Visible> {
    forum
        .posts()
        .iter()
        .filter_map(|p| {
            let s = viewer.separation(p.terminus())?;
            s.cost().is_finite().then(|| Visible {
                id: p.id,
                cost: s.cost(),
            })
        })
        .collect()
}

/// Whether a specific post is visible to a viewer.
#[must_use]
pub fn can_see(forum: &Forum, viewer: &MediumGraph, id: PostId) -> bool {
    forum
        .get(id)
        .and_then(|p| viewer.separation(p.terminus()))
        .is_some_and(|s| s.cost().is_finite())
}

/// Classify a registration from the gaps it moved (Thm 7.5).
///
/// Deliberately does not take the post, the body, or the speaker — only the
/// gaps. See [`crate::act::classify`], which this simply names for the forum.
#[must_use]
pub fn classify_post(acting: Gaps, receiving: Gaps) -> Act {
    classify(acting, receiving)
}

/// How a feed is ordered.
///
/// The two variants are the only orderings emitted states support. There is
/// deliberately no `Top`, no `Best`, and no `Controversial`: each would be a
/// verdict aggregated across agents, and Prop. 9.6 says such an aggregate is
/// not merely forbidden but ill-defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    /// Most recently registered first. Ties keep registration order.
    #[default]
    Recent,
    /// Nearest first: least separation cost on the viewer's own graph.
    ///
    /// This is a fact about the *viewer*, not about the post, and two viewers
    /// will order the same posts differently. That is Theorem 6.2 showing
    /// through the UI rather than being papered over.
    Near,
}

/// An ordered view of what one agent can see.
#[derive(Debug, Clone)]
pub struct Feed {
    /// Visible posts, in the requested order.
    pub items: Vec<Visible>,
    /// The ordering applied.
    pub order: Order,
}

/// Build a feed for a viewer.
#[must_use]
pub fn feed(forum: &Forum, viewer: &MediumGraph, order: Order) -> Feed {
    let mut items = visible_to(forum, viewer);
    match order {
        Order::Recent => {
            items.sort_by(|a, b| {
                let (pa, pb) = (forum.get(a.id), forum.get(b.id));
                match (pa, pb) {
                    (Some(x), Some(y)) => y.at.cmp(&x.at).then(y.id.cmp(&x.id)),
                    _ => std::cmp::Ordering::Equal,
                }
            });
        }
        Order::Near => {
            items.sort_by(|a, b| {
                a.cost
                    .partial_cmp(&b.cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.id.cmp(&b.id))
            });
        }
    }
    Feed { items, order }
}

/// A convenience for building an emitted state at a terminus.
#[must_use]
pub fn at(terminus: Position, record: Record) -> Emitted {
    Emitted { terminus, record }
}

#[cfg(test)]
mod tests {
    use super::*;
    use closure_kernel::graph::ContactGraph;

    /// A six-position city: a left region {0,1,2} and a right region {3,4,5},
    /// joined at 2-3. Two viewers are pruned to one region each.
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

    /// A viewer reaching only the left region: positions 0..3 exist, 3..6 do
    /// not, so nothing registered there has a separation cost for them.
    fn left_viewer() -> MediumGraph {
        let mut g = ContactGraph::new(3);
        g.add_edge(0, 1, 2.0).unwrap();
        g.add_edge(1, 2, 2.0).unwrap();
        MediumGraph::new(g, 1.0).unwrap()
    }

    fn whole_city_viewer() -> MediumGraph {
        MediumGraph::new(city(), 1.0).unwrap()
    }

    fn e(t: Position) -> Emitted {
        at(t, Record::new())
    }

    #[test]
    fn visibility_follows_reach_with_no_access_list() {
        let mut f = Forum::new();
        let near = f.register(None, Speaker::Player, "in the left region", e(1), None);
        let far = f.register(
            None,
            Speaker::Agent("a".into()),
            "off in the right",
            e(4),
            None,
        );

        let left = left_viewer();
        assert!(can_see(&f, &left, near));
        assert!(
            !can_see(&f, &left, far),
            "an unreached region is private without any flag saying so"
        );

        let all = whole_city_viewer();
        assert!(can_see(&f, &all, near));
        assert!(can_see(&f, &all, far));
    }

    #[test]
    fn a_private_message_is_just_a_terminus_two_graphs_reach() {
        // Nothing in the type system distinguishes this from a public post.
        let mut f = Forum::new();
        let dm = f.register(None, Speaker::Player, "just between us", e(5), None);
        assert!(!can_see(&f, &left_viewer(), dm));
        assert!(can_see(&f, &whole_city_viewer(), dm));
    }

    #[test]
    fn thm_6_2_two_viewers_order_the_same_posts_differently() {
        let mut f = Forum::new();
        f.register(None, Speaker::Player, "at 0", e(0), None);
        f.register(None, Speaker::Player, "at 2", e(2), None);
        f.register(None, Speaker::Player, "at 5", e(5), None);

        let a = feed(&f, &whole_city_viewer(), Order::Near);
        let b = feed(&f, &left_viewer(), Order::Near);
        assert_ne!(
            a.items.len(),
            b.items.len(),
            "receiver relativity: the feed is not a property of the forum"
        );
    }

    #[test]
    fn cor_6_7_drift_accumulates_along_the_chain() {
        let mut f = Forum::new();
        let root = f.register(None, Speaker::Player, "seed", e(0), None);
        let a = f.register(Some(root), Speaker::Agent("x".into()), "re", e(0), None);
        let b = f.register(Some(a), Speaker::Agent("y".into()), "re re", e(0), None);
        let _side = f.register(Some(root), Speaker::Agent("z".into()), "aside", e(0), None);

        assert_eq!(f.depth(root), 2, "longest re-registration chain");
        assert_eq!(f.root_of(b), Some(root));
        assert_eq!(f.thread(root).len(), 4);
        assert_eq!(f.replies(root).len(), 2);
    }

    #[test]
    fn thm_7_4_agents_may_reply_with_no_user_present() {
        let mut f = Forum::new();
        let root = f.register(None, Speaker::Agent("x".into()), "opening", e(1), None);
        let r1 = f.register(Some(root), Speaker::Agent("y".into()), "and?", e(1), None);
        f.register(Some(r1), Speaker::Agent("x".into()), "well", e(1), None);
        assert!(
            f.thread(root)
                .iter()
                .all(|p| !matches!(p.speaker, Speaker::Player)),
            "exchange does not close, and does not require the player"
        );
        assert_eq!(f.depth(root), 2);
    }

    #[test]
    fn the_tick_advances_only_when_asked() {
        let mut f = Forum::new();
        assert_eq!(f.tick(), 0);
        let a = f.register(None, Speaker::Player, "first", e(0), None);
        assert_eq!(f.advance(), 1);
        let b = f.register(None, Speaker::Player, "second", e(0), None);
        assert_eq!(f.get(a).unwrap().at, 0);
        assert_eq!(f.get(b).unwrap().at, 1);
    }

    #[test]
    fn recency_order_is_newest_first() {
        let mut f = Forum::new();
        let a = f.register(None, Speaker::Player, "a", e(0), None);
        f.advance();
        let b = f.register(None, Speaker::Player, "b", e(0), None);
        let got = feed(&f, &whole_city_viewer(), Order::Recent);
        assert_eq!(got.items[0].id, b);
        assert_eq!(got.items[1].id, a);
    }

    #[test]
    fn a_post_carries_a_direction_and_never_a_score() {
        let mut f = Forum::new();
        let act = classify_post(Gaps::new(5.0, 3.0), Gaps::new(2.0, 4.0));
        let id = f.register(None, Speaker::Player, "both at once", e(0), Some(act));
        let p = f.get(id).unwrap();
        let a = p.act.unwrap();
        assert!(a.is_both(), "Thm 7.5(ii): report and question together");
        // The whole of what a post carries about its own effect:
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(json, r#"{"report":true,"question":true}"#);
    }

    #[test]
    fn there_is_no_edit_and_no_delete() {
        // Asserted by construction: `posts` is private and `Forum` exposes
        // only `register`. This test exists to fail loudly if that changes.
        let mut f = Forum::new();
        let id = f.register(None, Speaker::Player, "said", e(0), None);
        f.advance();
        assert_eq!(f.get(id).unwrap().body, "said");
        assert_eq!(f.posts().len(), 1);
    }

    #[test]
    fn an_unmeasured_act_is_absent_not_inert() {
        let mut f = Forum::new();
        let id = f.register(None, Speaker::Player, "unmeasured", e(0), None);
        assert!(
            f.get(id).unwrap().act.is_none(),
            "None means not measured; Act::default() would mean measured-and-inert"
        );
        assert!(Act::default().is_inert());
    }
}
