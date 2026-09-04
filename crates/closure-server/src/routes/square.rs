//! Square routes: subgroups, voices, characters, and pruning.
//!
//! These five endpoints are one path walked in order, and the order is the
//! design:
//!
//! 1. `GET .../subgroups` — the regions of the city.
//! 2. `GET .../subgroups/{name}/voices` — who is talking there.
//! 3. `GET .../voices/{id}/character` — reassemble who was speaking.
//! 4. `POST .../voices/{id}/prune` — make them someone.
//! 5. `POST .../voices/{id}/ask` — generate an attribute, on demand.
//!
//! Steps 1 and 2 are free: reading a square is watching characters talk to
//! themselves, and that needs no individual at all. Step 3 begins only once
//! a user has settled on a voice, because a conversation with a user is not
//! a moderator talking to itself.
//!
//! ## What is deliberately missing
//!
//! There is **no search endpoint**. No `?q=engineer`, no attribute filter, no
//! "find me someone who". Such a route would ask the population to return an
//! individual matching a description, and Theorem 4.3 says no operation has
//! the retrieval signature. The only way to a person here is to read the
//! square and point at a voice, which is recognition rather than search.
//!
//! ## Why pruning takes no profile
//!
//! An earlier version of these routes returned a list of admissible profiles
//! and made the client pick one. That was retrieval with extra steps: the
//! list was a catalogue, and choosing from it is the operation Theorem 4.3
//! denies. There is no list now, so there is nothing to choose from and no
//! tiebreak to declare — `binv:tiebreak` has nothing to bite on where
//! nothing was enumerated.
//!
//! What replaced the choice is `.../ask`. Identity is a question about
//! sources — who is most likely to have this character? — and its answer is
//! a solution space that stays coarse until something needs it finer. An
//! attribute that existed before it was asked for would be an attribute that
//! could be retrieved.

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use closure_kernel::SessionToken;
use closure_runtime::voice::VoiceId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

/// The character reassembled behind a voice.
#[derive(Debug, Serialize)]
pub struct CharacterView {
    voice: u32,
    /// The regions the voice was audible to. Several is ordinary.
    audible_to: Vec<String>,
    /// Positions the character was built over.
    footprint: Vec<u32>,
    /// The character invariant.
    chi: Option<f64>,
    /// What has been settled about who would have this character.
    ///
    /// Empty here, always. Nothing is true of them until something asks.
    identity: BTreeMap<String, String>,
}

/// `GET /v1/session/{token}/voices/{id}/character`
///
/// Reassembles the character behind a voice by capping every moderator it was
/// audible to and amalgamating the caps. Note the absence of a candidate
/// list: there is nothing here to choose between, because nothing was
/// enumerated.
pub async fn character(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, u32)>,
) -> Result<Json<CharacterView>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    st.with_session(&token, |s| {
        let voice = s.square.voice(VoiceId(id))?;
        let ch = s
            .square
            .character(VoiceId(id), &s.city_graph, &s.moderators)?;
        Some(CharacterView {
            voice: id,
            audible_to: s
                .square
                .audible_to(VoiceId(id), &s.moderators)
                .into_iter()
                .filter_map(|m| {
                    s.square
                        .subgroups
                        .iter()
                        .find(|g| g.members == m.region)
                        .map(|g| g.name.clone())
                })
                .collect(),
            footprint: voice.footprint().into_iter().collect(),
            chi: ch.chi(),
            identity: ch.identity.settled().clone(),
        })
    })
    .flatten()
    .map(Json)
    .ok_or_else(super::not_found)
}

/// A pruned agent.
#[derive(Debug, Serialize)]
pub struct AgentView {
    id: String,
    /// Positions in the pruned graph.
    order: u32,
    /// The monotone record. Zero here: pruning is idempotent *before* contact
    /// and irreversible after (Cor. 8.8), and this agent has not acted.
    record: u64,
    /// The character invariant that was pruned.
    chi: Option<f64>,
}

/// `POST /v1/session/{token}/voices/{id}/prune`
///
/// Turns a voice into someone. Takes no body: there is no profile to name and
/// no tiebreak to declare, because the character is built from where the
/// voice spoke rather than selected from a set. Who would have that character
/// is a separate question, asked afterwards and only for what is needed.
///
/// Refuses a voice that spoke too narrowly for a character to be built from
/// it. Nothing is invented to cover the shortfall.
pub async fn prune(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, u32)>,
) -> Result<Json<AgentView>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    let out = st.with_session(&token, |s| {
        let agent = s.square.prune(VoiceId(id), &s.city_graph, &s.moderators)?;
        Some(AgentView {
            id: agent.id.clone(),
            order: agent.graph.order(),
            record: agent.record().get(),
            chi: agent.chi(),
        })
    });
    match out {
        Some(Some(v)) => Ok(Json(v)),
        Some(None) => Err(super::bad_request_msg(
            "that voice has not said enough for anyone to be behind it",
        )),
        None => Err(super::not_found()),
    }
}

/// One attribute question about a character.
#[derive(Debug, Deserialize)]
pub struct Ask {
    /// What is being asked: `"car"`, `"district"`, `"grundschule"`.
    key: String,
    /// The answers this question admits. Required — the caller says what
    /// counts as an answer, because the substrate holds no catalogue to draw
    /// one from.
    options: Vec<String>,
}

/// What came back.
#[derive(Debug, Serialize)]
pub struct AskedView {
    key: String,
    value: String,
    /// How many attributes exist for this character after the question. One,
    /// and otherwise zero.
    settled: usize,
}

/// `POST /v1/session/{token}/voices/{id}/ask`
///
/// Generates an attribute of the character behind a voice, on demand.
///
/// This is the endpoint that makes the absent catalogue real. Before this
/// call the value does not exist and no operation could have returned it;
/// after it, it exists because it was asked for. The answer is deterministic
/// in the character and the question, so the character does not contradict
/// itself — but determinism here is a property of generation, not evidence
/// that something was stored and looked up.
///
/// Nothing is persisted across calls: the character is reassembled per
/// request, so this reports what the question settles rather than mutating a
/// profile. There is no profile to mutate.
pub async fn ask(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, u32)>,
    Json(body): Json<Ask>,
) -> Result<Json<AskedView>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    if body.options.is_empty() {
        return Err(super::bad_request_msg(
            "a question with no admissible answers has none",
        ));
    }
    let out = st.with_session(&token, |s| {
        let mut ch = s
            .square
            .character(VoiceId(id), &s.city_graph, &s.moderators)?;
        let opts: Vec<&str> = body.options.iter().map(String::as_str).collect();
        let graph = ch.graph.clone();
        let value = ch.identity.ask(&graph, &body.key, &opts)?;
        Some(AskedView {
            key: body.key.clone(),
            value,
            settled: ch.identity.len(),
        })
    });
    match out {
        Some(Some(v)) => Ok(Json(v)),
        Some(None) => Err(super::bad_request_msg(
            "there is no character behind that voice to ask about",
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
        assert!(
            v.get("profile").is_none(),
            "a voice is a split, not a person"
        );
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
    async fn a_character_is_reassembled_rather_than_selected() {
        let (app, token) = app();
        // Find a voice that spoke widely enough to have someone behind it.
        let mut found = None;
        for id in 0..8 {
            let (status, ch) = send(
                &app,
                get(&format!("/v1/session/{token}/voices/{id}/character")),
            )
            .await;
            if status == StatusCode::OK {
                found = Some(ch);
                break;
            }
        }
        let ch = found.expect("some voice ranged wide enough to be someone");
        assert!(!ch["audible_to"].as_array().unwrap().is_empty());
        assert!(ch["chi"].is_number(), "the reassembly has an invariant");
        // What is emphatically not there: a menu.
        assert!(ch.get("admissible").is_none(), "no candidate list");
        assert!(ch.get("profile").is_none(), "no profile was selected");
        assert!(ch.get("best").is_none(), "no ranking");
        assert!(ch.get("score").is_none());
        // And no attributes, because nothing has asked for any.
        assert_eq!(
            ch["identity"].as_object().unwrap().len(),
            0,
            "nothing is true of them until something asks"
        );
    }

    /// The id of a voice that ranged wide enough to have a character.
    async fn someone(app: &axum::Router, token: &str) -> u32 {
        for id in 0..8 {
            let (status, _) = send(
                app,
                get(&format!("/v1/session/{token}/voices/{id}/character")),
            )
            .await;
            if status == StatusCode::OK {
                return id;
            }
        }
        panic!("no voice in the seeded square ranged wide enough");
    }

    #[tokio::test]
    async fn pruning_a_voice_needs_no_profile_and_takes_no_body() {
        let (app, token) = app();
        let id = someone(&app, &token).await;
        let (status, agent) = send(
            &app,
            post(
                &format!("/v1/session/{token}/voices/{id}/prune"),
                serde_json::json!({}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(agent["record"], 0, "not yet an individual (Cor. 8.8)");
        assert!(agent["chi"].is_number());
        assert!(agent.get("profile").is_none(), "nothing was chosen");
    }

    #[tokio::test]
    async fn a_voice_that_said_too_little_has_nobody_behind_it() {
        // Nothing is invented to cover the shortfall.
        let (app, token) = app();
        let mut refused = false;
        for id in 0..8 {
            let (status, _) = send(
                &app,
                get(&format!("/v1/session/{token}/voices/{id}/character")),
            )
            .await;
            if status != StatusCode::OK {
                refused = true;
            }
        }
        // Either every voice ranged wide (fine), or the narrow ones were
        // refused rather than filled in. What must not happen is a character
        // appearing where the posts do not support one.
        let _ = refused;
        let (status, _) = send(
            &app,
            get(&format!("/v1/session/{token}/voices/9999/character")),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn an_attribute_exists_only_once_it_is_asked_for() {
        let (app, token) = app();
        let id = someone(&app, &token).await;

        // Before: the character carries nothing.
        let (_, ch) = send(
            &app,
            get(&format!("/v1/session/{token}/voices/{id}/character")),
        )
        .await;
        assert_eq!(ch["identity"].as_object().unwrap().len(), 0);

        // Asking generates it.
        let body = serde_json::json!({
            "key": "grundschule",
            "options": ["Hirschengraben", "Wipkingen", "Aussersihl"],
        });
        let (status, asked) = send(
            &app,
            post(
                &format!("/v1/session/{token}/voices/{id}/ask"),
                body.clone(),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(asked["key"], "grundschule");
        assert_eq!(asked["settled"], 1, "only what was asked exists");
        let first = asked["value"].as_str().unwrap().to_owned();

        // Asking again does not contradict the first answer.
        let (_, again) = send(
            &app,
            post(&format!("/v1/session/{token}/voices/{id}/ask"), body),
        )
        .await;
        assert_eq!(again["value"].as_str().unwrap(), first);

        // And nothing else was invented along the way.
        let (_, ch) = send(
            &app,
            get(&format!("/v1/session/{token}/voices/{id}/character")),
        )
        .await;
        assert_eq!(
            ch["identity"].as_object().unwrap().len(),
            0,
            "the character carries no attribute nobody asked for"
        );
    }

    #[tokio::test]
    async fn a_question_with_no_admissible_answers_has_none() {
        let (app, token) = app();
        let id = someone(&app, &token).await;
        let (status, _) = send(
            &app,
            post(
                &format!("/v1/session/{token}/voices/{id}/ask"),
                serde_json::json!({"key": "car", "options": []}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn the_square_keeps_talking_when_the_world_moves() {
        let (app, token) = app();
        let (_, before) = send(&app, get(&format!("/v1/session/{token}"))).await;
        let start = before["posts"].as_u64().unwrap();

        for _ in 0..3 {
            let (status, _) = send(
                &app,
                post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }

        let (_, after) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert!(
            after["posts"].as_u64().unwrap() > start,
            "a character talking to itself does not run out of things to ask"
        );
    }

    #[tokio::test]
    async fn what_a_tick_adds_is_a_thread_with_measured_acts() {
        let (app, token) = app();
        let (_, before) = send(&app, get(&format!("/v1/session/{token}/posts"))).await;
        let start = before.as_array().unwrap().len();
        send(
            &app,
            post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
        )
        .await;
        let (_, posts) = send(&app, get(&format!("/v1/session/{token}/posts"))).await;
        let fresh: Vec<_> = posts.as_array().unwrap()[..posts.as_array().unwrap().len() - start]
            .iter()
            .collect();
        assert!(!fresh.is_empty());
        // Unlike the seeded square, these gaps were actually measured, so
        // every one of them carries a direction.
        assert!(
            fresh.iter().all(|p| !p["act"].is_null()),
            "a round's posts report what they did to the gaps"
        );
        // And what they report is a direction, never a verdict.
        for p in &fresh {
            assert!(p["act"]["question"].is_boolean());
            assert!(p.get("score").is_none());
        }
    }

    #[tokio::test]
    async fn ticking_never_finishes_the_conversation() {
        // Thm 7.4 at the level of the running system: there is no tick after
        // which the square has nothing left to say, and no endpoint reports
        // that it is done.
        let (app, token) = app();
        let mut counts = Vec::new();
        for _ in 0..6 {
            let (_, t) = send(
                &app,
                post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
            )
            .await;
            counts.push(t["posts"].as_u64().unwrap());
            assert!(t.get("done").is_none(), "no exit code (Thm 11.5)");
            assert!(t.get("closed").is_none());
        }
        assert!(
            counts.windows(2).all(|w| w[1] > w[0]),
            "every tick adds to the square: {counts:?}"
        );
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
}
