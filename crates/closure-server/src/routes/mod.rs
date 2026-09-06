//! HTTP surface.
//!
//! The route list is short and deliberately incomplete in one direction:
//! there is nothing here that reports whether a player is succeeding, and
//! nothing that attributes a change in the world to something they did. See
//! the crate documentation for why those are absent rather than pending.

pub mod forum;
pub mod square;

use crate::state::AppState;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use closure_kernel::{INVARIANTS, SessionToken};
use serde::{Deserialize, Serialize};

/// Build the router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/cities", get(cities))
        .route("/v1/invariants", get(invariants))
        .route("/v1/session", post(open_session))
        .route("/v1/session/{token}", get(session_view))
        .route(
            "/v1/session/{token}/posts",
            get(forum::list).post(forum::create),
        )
        .route("/v1/session/{token}/thread/{id}", get(forum::thread))
        .route("/v1/session/{token}/tick", post(forum::tick))
        .route("/v1/session/{token}/subgroups", get(square::subgroups))
        .route(
            "/v1/session/{token}/subgroups/{name}/voices",
            get(square::voices),
        )
        .route(
            "/v1/session/{token}/voices/{id}/character",
            get(square::character),
        )
        .route("/v1/session/{token}/voices/{id}/prune", post(square::prune))
        .route("/v1/session/{token}/voices/{id}/ask", post(square::ask))
        .with_state(state)
}

// ---------------------------------------------------------------- health

#[derive(Debug, Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
    sessions: usize,
}

async fn health(State(st): State<AppState>) -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        sessions: st.len(),
    })
}

// ---------------------------------------------------------------- cities

#[derive(Debug, Serialize)]
struct City {
    id: &'static str,
    name: &'static str,
    substrate: &'static str,
}

/// The cities on offer.
///
/// ## Why there is no floor here any more
///
/// This used to advertise `floor: 0.0197` and a substrate of "Statistik
/// Stadt Zürich, open government data". Neither was true: the graph was
/// hand-written, and the floor was a number attached to a city that no
/// longer exists — the society is generated per session now, so a floor
/// published before the seed is known would be a measurement of nothing.
///
/// A number that had to be right and was not is worse than no number. If a
/// client wants the floor of the world it is actually in, that is a fact
/// about a session, and the session is where it can honestly be asked.
///
/// ## Why a city is still a name and not a world
///
/// The name a player picks does not choose a substrate — [`crate::society`]
/// draws from the seed alone. That is deliberate. A city whose name selected
/// a world would make one world privileged, and the whole point of
/// generating is that none is: the goal is unreachable in every society, so
/// a carefully-built one buys nothing a drawn one does not.
async fn cities() -> Json<Vec<City>> {
    Json(vec![City {
        id: "zuerich",
        name: "Zürich",
        substrate: "generated per session from the seed; no city is modelled",
    }])
}

// ------------------------------------------------------------ invariants

#[derive(Debug, Serialize)]
struct InvariantView {
    index: u8,
    name: &'static str,
    predicate: &'static str,
    certified_by: &'static str,
}

/// The commitments this host honours. Exposed so a client can display them,
/// and so a reviewer can check them without reading the source.
async fn invariants() -> Json<Vec<InvariantView>> {
    Json(
        INVARIANTS
            .iter()
            .map(|i| InvariantView {
                index: i.index,
                name: i.name,
                predicate: i.predicate,
                certified_by: i.certified_by,
            })
            .collect(),
    )
}

// --------------------------------------------------------------- session

#[derive(Debug, Deserialize)]
struct OpenSession {
    /// Token minted by the CLI.
    token: String,
    /// Which city to inhabit.
    #[serde(default = "default_city")]
    city: String,
    /// The seed the square opens at. Omitted means one is drawn.
    ///
    /// The seed is always reported back, because a run is reproducible as a
    /// protocol and the seed is part of it (Cor. 11.14). A session whose
    /// starting square could not be restated would not be reproducible at
    /// all.
    #[serde(default)]
    seed: Option<u64>,
}

fn default_city() -> String {
    String::from("zuerich")
}

#[derive(Debug, Serialize)]
struct Opened {
    token: String,
    city: String,
    /// The seed this square opened at. Restate it to reopen the same square.
    seed: u64,
}

/// Exchange a CLI-minted token for a live session.
async fn open_session(
    State(st): State<AppState>,
    Json(body): Json<OpenSession>,
) -> Result<Json<Opened>, (StatusCode, Json<ApiError>)> {
    let token = SessionToken::parse(&body.token).map_err(bad_request)?;
    // A drawn seed is derived from the token, so the pair (token, city) is
    // the whole of what a run needs to be restated.
    let seed = body.seed.unwrap_or_else(|| seed_from(token.as_str()));
    st.open(&token, &body.city, seed);
    Ok(Json(Opened {
        token: token.as_str().to_owned(),
        city: body.city,
        seed,
    }))
}

/// Report what has propagated in a session.
async fn session_view(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<crate::state::SessionView>, (StatusCode, Json<ApiError>)> {
    let token = SessionToken::parse(&token).map_err(bad_request)?;
    st.view(&token).map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                error: String::from("no such session"),
            }),
        )
    })
}

// ----------------------------------------------------------------- error

#[derive(Debug, Serialize)]
pub struct ApiError {
    error: String,
}

/// No such session.
fn not_found() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            error: String::from("no such session"),
        }),
    )
}

/// A stable seed from a token: FNV-1a over its bytes.
///
/// Deliberately not random. A session opened twice with the same token opens
/// on the same square, which is what makes the token alone sufficient to
/// restate a run.
fn seed_from(token: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in token.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn bad_request_msg(msg: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: msg.to_owned(),
        }),
    )
}

fn bad_request(e: closure_kernel::Error) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            error: e.to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_session_round_trips_through_its_token() {
        let st = AppState::new();
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        assert!(!st.contains(&token));
        st.open(&token, "zuerich", 20260904);
        assert!(st.contains(&token));
        let view = st.view(&token).expect("session exists");
        assert_eq!(view.city, "zuerich");
        assert_eq!(view.record, 0, "a fresh session has deposited nothing");
    }

    /// Invariant 6, checked against the declared routes.
    ///
    /// The list below is the complete route table. Adding a route that
    /// reports success, progress, or attribution would require editing this
    /// constant, which is the point: the check is a tripwire on the API
    /// surface rather than a grep over prose.
    const DECLARED_ROUTES: [&str; 13] = [
        "/health",
        "/v1/cities",
        "/v1/invariants",
        "/v1/session",
        "/v1/session/{token}",
        "/v1/session/{token}/posts",
        "/v1/session/{token}/thread/{id}",
        "/v1/session/{token}/tick",
        "/v1/session/{token}/subgroups",
        "/v1/session/{token}/subgroups/{name}/voices",
        "/v1/session/{token}/voices/{id}/character",
        "/v1/session/{token}/voices/{id}/prune",
        "/v1/session/{token}/voices/{id}/ask",
    ];

    #[test]
    fn no_route_reports_a_verdict_or_an_attribution() {
        let forbidden = [
            "score",
            "progress",
            "win",
            "lose",
            "attribution",
            "succeeded",
            "rank",
            "vote",
            "karma",
            "top",
            "best",
        ];
        for route in DECLARED_ROUTES {
            for word in forbidden {
                assert!(
                    !route.contains(word),
                    "route {route} would report a verdict, which Theorem 11.5 forbids"
                );
            }
        }
    }

    #[test]
    fn the_declared_route_list_matches_the_router() {
        // Guards against the tripwire above going stale: if a route is added
        // to `router` without being declared here, this fails.
        // Scan the body of `router` rather than line starts: a `.route(`
        // whose path sits on the following line must still be counted, or
        // the tripwire can be defeated by rustfmt.
        let src = include_str!("mod.rs");
        let body = src
            .split_once("pub fn router(")
            .and_then(|(_, rest)| rest.split_once(".with_state(state)"))
            .map(|(body, _)| body)
            .expect("router() is where routes are declared");
        let declared_in_router = body.match_indices(concat!('.', "route(")).count();
        assert_eq!(
            declared_in_router,
            DECLARED_ROUTES.len(),
            "DECLARED_ROUTES is out of date with router()"
        );
    }
}
