//! HTTP surface.
//!
//! The route list is short and deliberately incomplete in one direction:
//! there is nothing here that reports whether a player is succeeding, and
//! nothing that attributes a change in the world to something they did. See
//! the crate documentation for why those are absent rather than pending.

pub mod forum;

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
    floor: f64,
}

async fn cities() -> Json<Vec<City>> {
    Json(vec![City {
        id: "zuerich",
        name: "Zürich",
        substrate: "Statistik Stadt Zürich, open government data",
        floor: 0.0197,
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
}

fn default_city() -> String {
    String::from("zuerich")
}

#[derive(Debug, Serialize)]
struct Opened {
    token: String,
    city: String,
}

/// Exchange a CLI-minted token for a live session.
async fn open_session(
    State(st): State<AppState>,
    Json(body): Json<OpenSession>,
) -> Result<Json<Opened>, (StatusCode, Json<ApiError>)> {
    let token = SessionToken::parse(&body.token).map_err(bad_request)?;
    st.open(&token, &body.city);
    Ok(Json(Opened {
        token: token.as_str().to_owned(),
        city: body.city,
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
        let st = AppState::new(std::path::PathBuf::from("."));
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        assert!(!st.contains(&token));
        st.open(&token, "zuerich");
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
    const DECLARED_ROUTES: [&str; 8] = [
        "/health",
        "/v1/cities",
        "/v1/invariants",
        "/v1/session",
        "/v1/session/{token}",
        "/v1/session/{token}/posts",
        "/v1/session/{token}/thread/{id}",
        "/v1/session/{token}/tick",
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
