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
    /// Whether this host binds a fixed substrate to the name. Always false
    /// here, and present so a client can say so rather than imply it.
    bound: bool,
}

/// The cities on offer, which is to say: none in particular.
///
/// ## Why this list does not enumerate cities
///
/// It used to hold one entry, Zürich, and that was the only city a player
/// could name. Lifting the restriction is not a matter of lengthening the
/// list. A host offering ten cities would be claiming ten bound substrates
/// and it has none: [`crate::society`] draws a world from the seed and never
/// reads the name. Ten names would make those ten privileged and every other
/// name illegitimate, which is exactly the privilege generating removed.
///
/// So the endpoint reports the rule instead of a menu. `any` is not a city;
/// it is the statement that a city here is a name a player writes, that the
/// name is theirs, and that no name is bound to a world. A client wanting a
/// picker may offer suggestions of its own — they will be suggestions about
/// vocabulary, not about substrate.
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
/// The name a player writes does seed the draw, so writing a different one
/// opens a different society. It does not *select* one: there is no table
/// from name to world, and no name reaches a world another name could not.
/// A city whose name chose a prepared substrate would make that world
/// privileged, and the whole point of generating is that none is — the goal
/// is unreachable in every society, so a carefully-built one buys nothing a
/// drawn one does not.
async fn cities() -> Json<Vec<City>> {
    Json(vec![City {
        id: "any",
        name: "any city you name",
        substrate: "generated per session from the seed and the name; no city is modelled",
        bound: false,
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
    /// Which city to inhabit. Any name; none is bound to a world.
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

/// How long a city name may be, in characters.
///
/// Generous enough that no real place name and no reasonable invention hits
/// it, small enough that a name cannot be used as storage.
const CITY_NAME_LIMIT: usize = 64;

/// The city a request that names none opens in.
///
/// Deliberately not a real place. A default of `zuerich` made one city the
/// one you got by saying nothing, and a player who never touched the field
/// would have concluded the world was Zürich — which it was not, then or
/// now. `somewhere` is honest about being a placeholder and reads as one.
fn default_city() -> String {
    String::from("somewhere")
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
    // Any name is a city, but it is stored for the life of the session and
    // echoed to every client that reads it, so it is bounded. The limit is
    // on length alone — refusing names for their characters would be this
    // host deciding which cities are allowed to exist, which is the same
    // privilege the list refuses.
    let city = body.city.trim();
    if city.is_empty() {
        return Err(bad_request_msg("a city needs a name; any name will do"));
    }
    if city.chars().count() > CITY_NAME_LIMIT {
        return Err(bad_request_msg(
            "that city name is too long to carry around",
        ));
    }
    let city = city.to_owned();
    // A drawn seed is derived from the token *and* the city name, so the
    // pair (token, city) is the whole of what a run needs to be restated.
    //
    // The name is mixed in because it is a player's to choose and ought to
    // matter. Deriving from the token alone made two players who named
    // different cities inhabit a bit-identical world, which would make the
    // field decorative — and a field that looks like a choice and is not is
    // worse than no field. An explicit `seed` still overrides both, so a run
    // stays restatable from the seed alone (Cor. 11.14).
    let seed = body
        .seed
        .unwrap_or_else(|| seed_from(token.as_str(), &city));
    st.open(&token, &city, seed);
    Ok(Json(Opened {
        token: token.as_str().to_owned(),
        city,
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
/// Draw a seed from what the player supplied and nothing else.
///
/// FNV-1a over the token and the city name, separated by a byte that cannot
/// occur in either, so that ("AB", "C") and ("A", "BC") do not collide into
/// the same world. Not a cryptographic hash and not asked to be one: it
/// draws a world, it does not protect anything.
fn seed_from(token: &str, city: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(token.as_bytes());
    eat(&[0]);
    eat(city.as_bytes());
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
        st.open(&token, "kigali", 20260904);
        assert!(st.contains(&token));
        let view = st.view(&token).expect("session exists");
        assert_eq!(view.city, "kigali");
        assert_eq!(view.record, 0, "a fresh session has deposited nothing");
    }

    #[test]
    fn any_name_is_a_city_and_none_is_privileged() {
        // The whole content of "you can play anywhere": the host has no
        // list to check a name against, so there is no name it prefers and
        // none it refuses. `zuerich` is in here to make the point that it
        // is now one name among all of them rather than the one.
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        let st = AppState::new();
        for name in [
            "zuerich",
            "kigali",
            "valparaíso",
            "the place I grew up",
            "x",
        ] {
            st.open(&token, name, seed_from(token.as_str(), name));
            assert_eq!(st.view(&token).expect("session exists").city, name);
        }
    }

    #[test]
    fn naming_a_different_city_opens_a_different_society() {
        // Why the name is mixed into the seed at all. If it were not, this
        // field would look like a choice and be scenery.
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        let a = seed_from(token.as_str(), "kigali");
        let b = seed_from(token.as_str(), "zuerich");
        assert_ne!(a, b);
        assert_ne!(
            crate::society::Society::generate(a).regions.len() * 100
                + crate::society::Society::generate(a).order() as usize,
            crate::society::Society::generate(b).regions.len() * 100
                + crate::society::Society::generate(b).order() as usize,
            "two names drew the same shape; unlucky, but check the mixing"
        );
    }

    #[test]
    fn the_same_name_and_token_reopen_the_same_square() {
        // Cor. 11.14: reproducibility attaches to the protocol, and the
        // protocol here is the pair a player restates.
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        assert_eq!(
            seed_from(token.as_str(), "kigali"),
            seed_from(token.as_str(), "kigali")
        );
    }

    #[test]
    fn the_separator_keeps_a_name_from_bleeding_into_a_token() {
        // ("AB", "C") and ("A", "BC") must not be the same world. Without
        // the separator byte they would be, and two unrelated players would
        // quietly share a square.
        assert_ne!(seed_from("AB", "C"), seed_from("A", "BC"));
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
