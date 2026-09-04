//! Square routes: subgroups, voices, fits, and realisation.
//!
//! These four endpoints are one path walked in order, and the order is the
//! design:
//!
//! 1. `GET .../subgroups` — the regions of the city.
//! 2. `GET .../subgroups/{name}/voices` — who is talking there.
//! 3. `GET .../voices/{id}/fit` — what profiles would have said this.
//! 4. `POST .../voices/{id}/realise` — become one of them.
//!
//! ## What is deliberately missing
//!
//! There is **no search endpoint**. No `?q=engineer`, no attribute filter, no
//! "find me someone who". Such a route would ask the population to return an
//! individual matching a description, and Theorem 4.3 says no operation has
//! the retrieval signature. The only way to a person here is to read the
//! square and point at a voice, which is recognition rather than search.
//!
//! ## Why realisation demands a choice
//!
//! `GET .../fit` usually returns several admissible profiles, because a
//! voice's posts genuinely fail to single one out (Prop. 3.4 in the
//! population). The realise endpoint therefore *requires* the client to name
//! which profile it means, and refuses one that the posts do not support.
//! Defaulting to the first would be the silent tiebreak `binv:tiebreak`
//! forbids — the choice would have been made, and nothing would record that
//! the square did not make it.

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use closure_kernel::SessionToken;
use closure_runtime::population::SubstrateRecord;
use closure_runtime::voice::VoiceId;
use serde::{Deserialize, Serialize};

/// A region of the city.
#[derive(Debug, Serialize)]
pub struct SubgroupView {
    name: String,
    /// Positions this region spans. Visibility follows from these.
    positions: Vec<u32>,
    /// Voices heard here. A count of handles, not of people.
    voices: usize,
}

/// `GET /v1/session/{token}/subgroups`
pub async fn subgroups(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Vec<SubgroupView>>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    st.with_session(&token, |s| {
        s.square
            .subgroups
            .iter()
            .map(|g| SubgroupView {
                name: g.name.clone(),
                positions: g.members.iter().copied().collect(),
                voices: s.square.voices_in(g).len(),
            })
            .collect()
    })
    .map(Json)
    .ok_or_else(super::not_found)
}

/// A voice as the API reports it.
///
/// Carries a handle, a display name, and where it has spoken. There is no
/// profile field, because there is no profile until one is fitted — a voice
/// that arrived with attributes would be a person waiting to be looked up.
#[derive(Debug, Serialize)]
pub struct VoiceView {
    id: u32,
    name: String,
    /// Distinct positions it has spoken at.
    spoken_at: Vec<u32>,
    /// The regions those positions fall in. Several is ordinary.
    subgroups: Vec<String>,
    /// How many times it has posted.
    posts: usize,
}

/// `GET /v1/session/{token}/subgroups/{name}/voices`
///
/// The only way to enumerate voices. Note what the path requires: a region,
/// not a description. A client must already have decided where to listen.
pub async fn voices(
    State(st): State<AppState>,
    Path((token, name)): Path<(String, String)>,
) -> Result<Json<Vec<VoiceView>>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    st.with_session(&token, |s| {
        let group = s.square.subgroups.iter().find(|g| g.name == name)?;
        Some(
            s.square
                .voices_in(group)
                .into_iter()
                .map(|v| VoiceView {
                    id: v.id.0,
                    name: v.name.clone(),
                    spoken_at: v.footprint().into_iter().collect(),
                    subgroups: v
                        .subgroups(&s.square.subgroups)
                        .into_iter()
                        .map(|g| g.name.clone())
                        .collect(),
                    posts: v.spoken_at.len(),
                })
                .collect(),
        )
    })
    .flatten()
    .map(Json)
    .ok_or_else(super::not_found)
}

/// What profiles would have produced a voice's posts.
#[derive(Debug, Serialize)]
pub struct FitView {
    voice: u32,
    /// Every profile the posts are consistent with. Not ranked: the order is
    /// the population's declaration order and carries no preference.
    admissible: Vec<String>,
    /// The positions the fit had to account for.
    footprint: Vec<u32>,
    /// Whether exactly one profile satisfies the posts.
    ///
    /// When false, the client's choice at `/realise` is the client's own and
    /// not something the square determined. Reported rather than resolved
    /// (`binv:tiebreak`).
    determinate: bool,
}

/// `GET /v1/session/{token}/voices/{id}/fit`
pub async fn fit(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, u32)>,
) -> Result<Json<FitView>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    st.with_session(&token, |s| {
        let f = s.square.fit(VoiceId(id), &s.population)?;
        Some(FitView {
            voice: f.voice.0,
            determinate: f.is_determinate(),
            admissible: f.admissible,
            footprint: f.footprint.into_iter().collect(),
        })
    })
    .flatten()
    .map(Json)
    .ok_or_else(super::not_found)
}

/// Which profile to realise a voice as.
#[derive(Debug, Deserialize)]
pub struct Realise {
    /// A profile name from the voice's fit. Required: see the module docs on
    /// why there is no default.
    profile: String,
    /// Coarse substrate attributes, if any. Never an identifier.
    #[serde(default)]
    record: SubstrateRecord,
}

/// A realised agent.
#[derive(Debug, Serialize)]
pub struct AgentView {
    id: String,
    /// Positions in the pruned graph.
    order: u32,
    /// The monotone record. Zero here: pruning is idempotent *before* contact
    /// and irreversible after (Cor. 8.8), and this agent has not acted.
    record: u64,
    /// The profile the client chose.
    profile: String,
    /// Whether the fit determined that profile, or the client did.
    determinate: bool,
}

/// `POST /v1/session/{token}/voices/{id}/realise`
///
/// Turns a voice into someone. Refuses a profile the voice's posts do not
/// support: a fit that does not satisfy the footprint would be an agent
/// asserted to have said things it could not have said.
pub async fn realise(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, u32)>,
    Json(body): Json<Realise>,
) -> Result<Json<AgentView>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    let vid = VoiceId(id);
    let out = st.with_session(&token, |s| {
        let f = s.square.fit(vid, &s.population)?;
        let agent = s
            .square
            .realise(vid, &s.population, &body.record, &body.profile)?;
        Some(AgentView {
            id: agent.id.clone(),
            order: agent.graph.order(),
            record: agent.record().get(),
            profile: body.profile.clone(),
            determinate: f.is_determinate(),
        })
    });
    match out {
        Some(Some(v)) => Ok(Json(v)),
        Some(None) => Err(super::bad_request_msg(
            "that profile does not satisfy the voice's posts",
        )),
        None => Err(super::not_found()),
    }
}

#[cfg(test)]
mod tests {
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use closure_kernel::SessionToken;
    use tower::ServiceExt;

    fn app() -> (axum::Router, String) {
        let st = AppState::new(std::path::PathBuf::from("."));
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        st.open(&token, "zuerich", 20260904);
        (crate::routes::router(st), token.as_str().to_owned())
    }

    async fn send(app: &axum::Router, req: Request<Body>) -> (StatusCode, serde_json::Value) {
        let res = app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 1 << 20)
            .await
            .unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
        )
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    fn post(uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    #[tokio::test]
    async fn the_square_is_already_talking_when_a_session_opens() {
        let (app, token) = app();
        let (status, view) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(view["tick"], 0, "nothing has been asked to move yet");
        assert!(
            view["posts"].as_u64().unwrap() > 0,
            "a player arrives to activity, not to silence"
        );
        assert!(view["voices"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn a_voice_is_reached_through_a_region_never_a_description() {
        let (app, token) = app();
        let (status, groups) = send(&app, get(&format!("/v1/session/{token}/subgroups"))).await;
        assert_eq!(status, StatusCode::OK);
        let name = groups[0]["name"].as_str().unwrap().to_owned();

        let (status, voices) = send(
            &app,
            get(&format!("/v1/session/{token}/subgroups/{name}/voices")),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(!voices.as_array().unwrap().is_empty());
        // What a voice carries, and what it does not.
        let v = &voices[0];
        assert!(v.get("spoken_at").is_some());
        assert!(v.get("age").is_none(), "no demographics on a voice");
        assert!(v.get("profession").is_none());
        assert!(v.get("profile").is_none(), "no profile until it is fitted");
    }

    #[tokio::test]
    async fn there_is_no_search_endpoint() {
        let (app, token) = app();
        for uri in [
            format!("/v1/session/{token}/search?q=engineer"),
            format!("/v1/session/{token}/voices?profession=engineer"),
            format!("/v1/session/{token}/agents?q=eth"),
        ] {
            let (status, _) = send(&app, get(&uri)).await;
            assert_eq!(
                status,
                StatusCode::NOT_FOUND,
                "Thm 4.3: no operation has the retrieval signature ({uri})"
            );
        }
    }

    #[tokio::test]
    async fn a_fit_reports_its_alternatives_rather_than_choosing() {
        let (app, token) = app();
        let (status, fit) = send(&app, get(&format!("/v1/session/{token}/voices/0/fit"))).await;
        assert_eq!(status, StatusCode::OK);
        let admissible = fit["admissible"].as_array().unwrap();
        assert!(!admissible.is_empty(), "someone could have said this");
        assert_eq!(
            fit["determinate"],
            admissible.len() == 1,
            "determinacy is reported, not assumed"
        );
        assert!(fit.get("best").is_none(), "no ranking among the fits");
        assert!(fit.get("score").is_none());
    }

    #[tokio::test]
    async fn realising_a_voice_requires_an_admissible_profile() {
        let (app, token) = app();
        let (_, fit) = send(&app, get(&format!("/v1/session/{token}/voices/0/fit"))).await;
        let profile = fit["admissible"][0].as_str().unwrap().to_owned();

        let (status, agent) = send(
            &app,
            post(
                &format!("/v1/session/{token}/voices/0/realise"),
                serde_json::json!({"profile": profile}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(agent["record"], 0, "not yet an individual (Cor. 8.8)");
        assert_eq!(agent["profile"], profile);
    }

    #[tokio::test]
    async fn a_profile_the_posts_do_not_support_is_refused() {
        let (app, token) = app();
        let (status, _) = send(
            &app,
            post(
                &format!("/v1/session/{token}/voices/0/realise"),
                serde_json::json!({"profile": "not-a-module"}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn realisation_has_no_default_profile() {
        // Omitting the choice is an error, not a silent pick of the first
        // admissible fit (`binv:tiebreak`).
        let (app, token) = app();
        let (status, _) = send(
            &app,
            post(
                &format!("/v1/session/{token}/voices/0/realise"),
                serde_json::json!({}),
            ),
        )
        .await;
        assert_ne!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_same_seed_opens_the_same_square() {
        let build = || {
            let st = AppState::new(std::path::PathBuf::from("."));
            let mut rng = rand::rng();
            let token = SessionToken::generate(&mut rng);
            st.open(&token, "zuerich", 77);
            (crate::routes::router(st), token.as_str().to_owned())
        };
        let (a, ta) = build();
        let (b, tb) = build();
        let (_, pa) = send(&a, get(&format!("/v1/session/{ta}/posts"))).await;
        let (_, pb) = send(&b, get(&format!("/v1/session/{tb}/posts"))).await;
        assert_eq!(pa, pb, "reproducible as a protocol (Cor. 11.14)");
    }

    #[tokio::test]
    async fn an_unknown_voice_has_no_fit() {
        let (app, token) = app();
        let (status, _) = send(&app, get(&format!("/v1/session/{token}/voices/9999/fit"))).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}
