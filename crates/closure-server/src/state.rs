//! Server state: sessions and the worlds they inhabit.

use closure_kernel::SessionToken;
use closure_runtime::forum::Speaker;
use closure_runtime::forum::open_square;
use closure_runtime::moderator::{Identity, Moderator, post_round};
use closure_runtime::voice::{Seeding, Square, seed};
use closure_runtime::{Forum, Runtime};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// How many instances a character splits into when it takes a turn.
///
/// Three is enough for the deficit to be visible without the thread
/// becoming a wall. Nothing depends on the number being this one.
const SPLITS: u32 = 3;

/// One player's session in one city.
#[derive(Debug)]
pub struct Session {
    /// Which city.
    pub city: String,
    /// The node runtime for this world.
    pub runtime: Runtime,
    /// The forum: posts, threads, and the world tick.
    pub forum: Forum,
    /// The square: who is talking, and in which regions.
    pub square: Square,
    /// The characters talking in this city, one per region. A voice is a
    /// split of one of these, never an entry looked up among them.
    pub moderators: Vec<Moderator>,
    /// The society this session opened onto: the graph, and the regions cut
    /// from it. Generated from `seed` and nothing else, so restating the seed
    /// reopens the same world (Cor. 11.14).
    pub society: crate::society::Society,
    /// The seed this square was opened with. Part of the protocol, so a run
    /// is reproducible as a sequence of requests (Cor. 11.14).
    pub seed: u64,
    /// When the session opened.
    pub opened: time::OffsetDateTime,
    /// The last observation registered here. Display only.
    pub weather: Option<crate::weather::Observation>,
    /// What has been asked about each voice's character, and settled.
    ///
    /// A character is *rebuilt* from caps on every request rather than
    /// stored, which is what keeps it a reassembly and not a record looked
    /// up. But an answer, once settled, has to survive the request that
    /// settled it — otherwise `settled` counts a map that was discarded, and
    /// the client is told "one thing is settled" forever. Only the answers
    /// live here; the character itself is still rebuilt each time.
    pub identities: std::collections::BTreeMap<u32, Identity>,
}

impl Session {
    /// A fresh session in `city`, opening onto a square already in
    /// conversation.
    ///
    /// The forum is *not* empty at tick zero. A player who arrived to
    /// silence would have no way to reach anyone except by describing them,
    /// and description is the retrieval Theorem 4.3 denies. Prior activity is
    /// what makes recognition possible.
    #[must_use]
    pub fn new(city: impl Into<String>, seed: u64) -> Self {
        let city = city.into();
        // One generation, used for all three. They used to be three
        // independent calls that each rebuilt the graph and had to agree by
        // construction; a generated world makes that agreement a thing to
        // hold rather than assume.
        let society = crate::society::Society::generate(seed);
        let moderators = society.moderators();
        let mut square = Square::new(society.subgroups());
        let utterances = seed_square(&mut square, &society.graph, seed);
        let mut forum = Forum::new();
        let _ = open_square(&mut forum, &square, &utterances, |u| {
            crate::substrate::utterance_body(&square, u)
        });
        Self {
            city,
            runtime: Runtime::new(),
            forum,
            square,
            moderators,
            society,
            seed,
            opened: time::OffsetDateTime::now_utc(),
            weather: None,
            identities: std::collections::BTreeMap::new(),
        }
    }
}

fn seed_square(
    square: &mut Square,
    city: &closure_kernel::ContactGraph,
    s: u64,
) -> Vec<closure_runtime::voice::Utterance> {
    seed(square, city, &Seeding::default(), s)
}

/// Register an observation: one post per reached region, and one emission.
///
/// ## Why one post per region rather than one post
///
/// Theorem 6.2: a broadcast is not one message with many readers, it is N
/// independent registrations, one per receiver. A single post would also be
/// invisible to most of the city, whose graphs do not reach whatever one
/// position it landed on. The posts are roots and are never threaded to each
/// other — threading would make Corollary 6.7's drift chain count a
/// re-registration that never happened.
///
/// ## Why the theta carries the region and never the reading
///
/// `protocol_fingerprint` hashes node **keys** and nothing else. Naming a
/// node `weather/{region}` therefore makes the fingerprint see *reach* and
/// leaves it blind to the reading: two runs fed wildly different weather
/// that trips the same rules fingerprint identically. That is truth-blindness
/// executable in the protocol hash rather than asserted in a comment.
///
/// The full reading still goes in the *value*, which is the one place it can
/// safely be recorded: a value is opaque to the runtime, which is the fact
/// that makes Theorem 11.5 go through.
fn register_weather(session: &mut Session, o: &crate::weather::Observation) {
    let source = format!("weather/{}", o.source);
    session.runtime.emit(
        &source,
        serde_json::to_value(o).unwrap_or(serde_json::Value::Null),
        None,
    );
    for region in crate::weather::reach(o, &session.society) {
        // The region's own character. A region too thin to have a moderator
        // has nobody to register with, and is skipped rather than invented.
        let Some(members) = session
            .square
            .subgroups
            .iter()
            .find(|g| g.name == *region)
            .map(|g| &g.members)
        else {
            continue;
        };
        let Some(t) = session
            .moderators
            .iter()
            .find(|m| m.region == *members)
            .and_then(crate::weather::terminus)
        else {
            continue;
        };
        session.runtime.emit(
            &format!("weather/{region}"),
            serde_json::to_value(o).unwrap_or(serde_json::Value::Null),
            Some(&source),
        );
        let _ = session.forum.register(
            None,
            Speaker::World(o.source.clone()),
            crate::weather::body(o, &region),
            closure_runtime::forum::at(t, closure_kernel::identity::Record::new()),
            // No gaps were measured. Absence of a classification is not a
            // classification of absence.
            None,
        );
    }
}

/// What the API reports about a session.
///
/// Note the absence of a score, a completion percentage, or an outcome
/// verdict. The fields here are the record of what propagated: the protocol
/// fingerprint, which is reproducible, and the emission count, which is
/// monotone. Both are facts about the run rather than judgements of it.
#[derive(Debug, Serialize)]
pub struct SessionView {
    /// City handle.
    pub city: String,
    /// Nodes in the world.
    pub nodes: usize,
    /// Total emissions; monotone, never decremented.
    pub record: u64,
    /// Stable hash of the node set. Reproducibility attaches to this rather
    /// than to any reading (Corollary 11.14).
    pub protocol_fingerprint: String,
    /// RFC 3339 timestamp.
    pub opened: String,
    /// Posts registered so far. A count, not a ranking.
    pub posts: usize,
    /// The world tick. Advances only when a client asks it to.
    pub tick: u64,
    /// Voices in the square. Not people, and not a headcount of anyone real.
    pub voices: usize,
    /// The seed this square opened with.
    pub seed: u64,
    /// Positions in this session's city. Generated, so it differs per
    /// session — the client cannot assume a fixed width.
    pub order: u32,
    /// The last observation registered in this session, if any. Display
    /// only: what it *did* is already in the termini of its posts.
    pub weather: Option<crate::weather::Observation>,
}

impl Session {
    /// A serialisable view.
    #[must_use]
    pub fn view(&self) -> SessionView {
        SessionView {
            city: self.city.clone(),
            nodes: self.runtime.len(),
            record: self.runtime.record(),
            protocol_fingerprint: self.runtime.protocol_fingerprint(),
            opened: self
                .opened
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            posts: self.forum.posts().len(),
            tick: self.forum.tick(),
            voices: self.square.voices().len(),
            seed: self.seed,
            order: self.society.order(),
            weather: self.weather.clone(),
        }
    }
}

/// Shared application state.
#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    sessions: RwLock<HashMap<String, Session>>,
    weather: crate::weather::Source,
}

impl AppState {
    /// Build state with no weather.
    ///
    /// There is no data directory. There used to be a `--data-dir` flag
    /// pointing at "city substrates", and it was read by nothing — the city
    /// was hardcoded, and is now generated from the seed. A flag promising a
    /// data source that does not exist is worse than no flag: it tells a
    /// reader the substrate came from somewhere.
    ///
    /// The default is no world feed, so tests and CI are hermetic and a run
    /// is reproducible as a sequence of requests. Note this is about body
    /// determinism and not about Corollary 11.14: the theta design makes the
    /// protocol fingerprint blind to the reading either way, so turning the
    /// live feed on does not cost reproducibility of the protocol.
    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub fn new() -> Self {
        Self::with_weather(crate::weather::Source::Fixed(None))
    }

    /// Build state with a weather source.
    #[must_use]
    pub fn with_weather(weather: crate::weather::Source) -> Self {
        Self {
            inner: Arc::new(Inner {
                sessions: RwLock::new(HashMap::new()),
                weather,
            }),
        }
    }

    /// The current observation, if the world is reporting.
    ///
    /// Called by the tick handler *before* it takes the session lock — the
    /// fetch must not happen inside it.
    pub async fn observation(&self) -> Option<crate::weather::Observation> {
        self.inner.weather.current().await
    }

    /// Register a session under `token`, opening its square at `seed`.
    pub fn open(&self, token: &SessionToken, city: &str, seed: u64) {
        if let Ok(mut s) = self.inner.sessions.write() {
            s.insert(token.as_str().to_owned(), Session::new(city, seed));
        }
    }

    /// Read a whole session. The square routes attach here.
    pub fn with_session<T>(
        &self,
        token: &SessionToken,
        f: impl FnOnce(&Session) -> T,
    ) -> Option<T> {
        let guard = self.inner.sessions.read().ok()?;
        guard.get(token.as_str()).map(f)
    }

    /// Read a session view.
    #[must_use]
    pub fn view(&self, token: &SessionToken) -> Option<SessionView> {
        self.inner
            .sessions
            .read()
            .ok()?
            .get(token.as_str())
            .map(Session::view)
    }

    /// Whether a session exists.
    #[cfg_attr(not(test), allow(dead_code))]
    #[must_use]
    pub fn contains(&self, token: &SessionToken) -> bool {
        self.inner
            .sessions
            .read()
            .is_ok_and(|s| s.contains_key(token.as_str()))
    }

    /// Number of live sessions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.sessions.read().map_or(0, |s| s.len())
    }

    /// Whether no sessions are live.
    #[allow(dead_code)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Mutate a whole session. The routes that settle something attach here.
    pub fn with_session_mut<T>(
        &self,
        token: &SessionToken,
        f: impl FnOnce(&mut Session) -> T,
    ) -> Option<T> {
        let mut guard = self.inner.sessions.write().ok()?;
        guard.get_mut(token.as_str()).map(f)
    }

    /// Mutate a session's forum. The forum routes attach here.
    pub fn with_forum<T>(
        &self,
        token: &SessionToken,
        f: impl FnOnce(&mut Forum) -> T,
    ) -> Option<T> {
        let mut guard = self.inner.sessions.write().ok()?;
        guard.get_mut(token.as_str()).map(|s| f(&mut s.forum))
    }

    /// Advance a session and let one character take a turn talking to itself.
    ///
    /// Each tick, one moderator — chosen by the tick itself, so the square
    /// does not favour a region — splits, questions its own instances
    /// privately, and their answers land in the forum as a thread. Nothing
    /// closes: the round is guaranteed to leave the character's gap where it
    /// was or worse (Thm 7.4), which is why there is always another tick
    /// worth taking.
    ///
    /// Returns the new tick and the post count after it.
    ///
    /// `observation`, when present, is the world reporting itself. It is
    /// passed in already fetched: this function holds a `std::sync::RwLock`
    /// across its whole body, so the network call cannot happen here.
    pub fn advance(
        &self,
        token: &SessionToken,
        observation: Option<&crate::weather::Observation>,
    ) -> Option<(u64, usize)> {
        let mut guard = self.inner.sessions.write().ok()?;
        let session = guard.get_mut(token.as_str())?;
        let tick = session.forum.advance();
        if let Some(o) = observation {
            register_weather(session, o);
            session.weather = Some(o.clone());
        }
        if !session.moderators.is_empty() {
            // Which character speaks is a function of the tick alone. No
            // region is preferred, and none is starved.
            let i = (tick as usize) % session.moderators.len();
            let m = session.moderators[i].clone();
            let instances = m.split(SPLITS);
            let turns = m.round(&instances);
            let region = session
                .square
                .subgroups
                .iter()
                .find(|g| g.members == m.region)
                .map_or_else(|| "the square".to_owned(), |g| g.name.clone());
            post_round(
                &mut session.forum,
                |id| Speaker::Agent(format!("{region}/{}", id.0)),
                &turns,
                |t| format!("[{region}] about {}", t.about),
            );
        }
        Some((tick, session.forum.posts().len()))
    }

    /// Read a session's forum without mutating it.
    pub fn with_forum_ref<T>(
        &self,
        token: &SessionToken,
        f: impl FnOnce(&Forum) -> T,
    ) -> Option<T> {
        let guard = self.inner.sessions.read().ok()?;
        guard.get(token.as_str()).map(|s| f(&s.forum))
    }
}
