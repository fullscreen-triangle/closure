//! Server state: sessions and the worlds they inhabit.

use closure_kernel::SessionToken;
use closure_runtime::Runtime;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// One player's session in one city.
#[derive(Debug)]
pub struct Session {
    /// Which city.
    pub city: String,
    /// The node runtime for this world.
    pub runtime: Runtime,
    /// When the session opened.
    pub opened: time::OffsetDateTime,
}

impl Session {
    /// A fresh session in `city`.
    #[must_use]
    pub fn new(city: impl Into<String>) -> Self {
        Self {
            city: city.into(),
            runtime: Runtime::new(),
            opened: time::OffsetDateTime::now_utc(),
        }
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
    #[allow(dead_code)]
    data_dir: PathBuf,
}

impl AppState {
    /// Build state rooted at `data_dir`.
    #[must_use]
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            inner: Arc::new(Inner {
                sessions: RwLock::new(HashMap::new()),
                data_dir,
            }),
        }
    }

    /// Register a session under `token`.
    pub fn open(&self, token: &SessionToken, city: &str) {
        if let Ok(mut s) = self.inner.sessions.write() {
            s.insert(token.as_str().to_owned(), Session::new(city));
        }
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

    /// Mutate a session's runtime. The conversation routes attach here.
    #[allow(dead_code)]
    pub fn with_runtime<T>(
        &self,
        token: &SessionToken,
        f: impl FnOnce(&mut Runtime) -> T,
    ) -> Option<T> {
        let mut guard = self.inner.sessions.write().ok()?;
        guard.get_mut(token.as_str()).map(|s| f(&mut s.runtime))
    }
}
