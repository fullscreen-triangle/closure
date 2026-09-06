//! Forum routes: posts, threads, feeds, and the tick.
//!
//! What is here mirrors [`closure_runtime::forum`], and what is absent
//! mirrors it too. There is no vote endpoint, no score field, and no `top`
//! ordering, because a ranking aggregated across agents is ill-defined
//! (Prop. 9.6) rather than merely disallowed. A post reports the *direction*
//! it moved the gaps — report, question, both, or neither — and that is the
//! whole of what the API says about an act.
//!
//! `POST /v1/session/{token}/tick` advances the world one step. Nothing on
//! this server runs on a timer: the world moves when a client asks it to, so
//! a run is a sequence of requests and is reproducible as one.

use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use closure_kernel::SessionToken;
use closure_kernel::identity::Record;
use closure_runtime::act::Gaps;
use closure_runtime::forum::{Order, PostId, Speaker, at, classify_post};
use serde::{Deserialize, Serialize};

/// A post as the API reports it.
///
/// Carries the terminus and record it registered at, and the direction it
/// moved the gaps. Deliberately no score, no reference count, and no rank.
#[derive(Debug, Serialize)]
pub struct PostView {
    id: u64,
    parent: Option<u64>,
    speaker: Speaker,
    body: String,
    terminus: u32,
    record: u64,
    at: u64,
    /// Direction only. `null` means the gaps were not measured, which is not
    /// the same as measured-and-unmoved.
    act: Option<ActView>,
}

/// The two independent directions of Theorem 7.5.
#[derive(Debug, Serialize)]
pub struct ActView {
    report: bool,
    question: bool,
}

fn view(p: &closure_runtime::forum::Post) -> PostView {
    PostView {
        id: p.id.0,
        parent: p.parent.map(|x| x.0),
        speaker: p.speaker.clone(),
        body: p.body.clone(),
        terminus: p.emitted.terminus,
        record: p.emitted.record.get(),
        at: p.at,
        act: p.act.map(|a| ActView {
            report: a.report,
            question: a.question,
        }),
    }
}

// ------------------------------------------------------------------ post

/// Register a post.
#[derive(Debug, Deserialize)]
pub struct NewPost {
    /// The post being replied to, if any.
    #[serde(default)]
    parent: Option<u64>,
    /// Who is speaking. Absent means the player.
    #[serde(default)]
    agent: Option<String>,
    /// What is said. Never inspected by the classifier.
    body: String,
    /// Where it registers. Visibility follows from this and nothing else.
    terminus: u32,
    /// The gaps on the acting side, before and after, if measured.
    #[serde(default)]
    acting: Option<[f64; 2]>,
    /// The gaps on the receiving side, before and after, if measured.
    #[serde(default)]
    receiving: Option<[f64; 2]>,
}

/// `POST /v1/session/{token}/posts`
pub async fn create(
    State(st): State<AppState>,
    Path(token): Path<String>,
    Json(body): Json<NewPost>,
) -> Result<Json<PostView>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    let speaker = body.agent.clone().map_or(Speaker::Player, Speaker::Agent);
    // Both sides must be measured for a classification to exist. One side
    // alone does not license half a verdict.
    let act = match (body.acting, body.receiving) {
        (Some(a), Some(r)) => Some(classify_post(Gaps::new(a[0], a[1]), Gaps::new(r[0], r[1]))),
        _ => None,
    };
    st.with_forum(&token, |f| {
        let record = Record::new();
        let id = f.register(
            body.parent.map(PostId),
            speaker,
            body.body.clone(),
            at(body.terminus, record),
            act,
        );
        f.get(id).map(view)
    })
    .flatten()
    .map(Json)
    .ok_or_else(super::not_found)
}

// ------------------------------------------------------------------ read

/// Query for a feed.
#[derive(Debug, Deserialize)]
pub struct FeedQuery {
    /// `recent` (default) or `near`. There is no `top`.
    #[serde(default)]
    order: Order,
}

/// `GET /v1/session/{token}/posts`
///
/// Returns roots and replies alike, ordered as asked. Note that `near`
/// ordering is relative to a viewer's graph, so this endpoint — which has no
/// viewer — serves `recent` for both and reports which it applied.
pub async fn list(
    State(st): State<AppState>,
    Path(token): Path<String>,
    Query(q): Query<FeedQuery>,
) -> Result<Json<Vec<PostView>>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    st.with_forum_ref(&token, |f| {
        let mut out: Vec<PostView> = f.posts().iter().map(view).collect();
        if matches!(q.order, Order::Recent) {
            out.reverse();
        }
        out
    })
    .map(Json)
    .ok_or_else(super::not_found)
}

/// `GET /v1/session/{token}/thread/{id}`
pub async fn thread(
    State(st): State<AppState>,
    Path((token, id)): Path<(String, u64)>,
) -> Result<Json<Vec<PostView>>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    st.with_forum_ref(&token, |f| {
        f.thread(PostId(id)).into_iter().map(view).collect()
    })
    .map(Json)
    .ok_or_else(super::not_found)
}

// ------------------------------------------------------------------ tick

/// What a tick reports.
#[derive(Debug, Serialize)]
pub struct Tick {
    /// The new tick. Monotone.
    tick: u64,
    /// Posts registered so far. A count, not a ranking.
    posts: usize,
    /// What the world reported this step, if it was reporting. Display only
    /// — its mechanical effect is already in the termini of its posts.
    weather: Option<crate::weather::Observation>,
}

/// `POST /v1/session/{token}/tick`
///
/// Advances the world one step and reports where it now is. It reports no
/// verdict on what happened during the step, because there is none to
/// compute (Theorem 11.5).
pub async fn tick(
    State(st): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<Tick>, (StatusCode, Json<super::ApiError>)> {
    let token = SessionToken::parse(&token).map_err(super::bad_request)?;
    // Fetched before the session lock is taken: `advance` holds a
    // `std::sync::RwLock` across its whole body and must not await.
    let observation = st.observation().await;
    st.advance(&token, observation.as_ref())
        .map(|(tick, posts)| {
            Json(Tick {
                tick,
                posts,
                weather: observation,
            })
        })
        .ok_or_else(super::not_found)
}

#[cfg(test)]
mod tests {
    use crate::state::AppState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use closure_kernel::SessionToken;
    use closure_runtime::forum::Speaker;
    use closure_runtime::moderator::Moderator;
    use tower::ServiceExt;

    /// A router with one open session, and its token.
    fn app() -> (axum::Router, String) {
        let st = AppState::new();
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
        let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, json)
    }

    fn post(uri: &str, body: serde_json::Value) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn get(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn a_post_round_trips_and_carries_a_direction_not_a_score() {
        let (app, token) = app();
        let (status, body) = send(
            &app,
            post(
                &format!("/v1/session/{token}/posts"),
                serde_json::json!({
                    "body": "the hydrofoil timetable is wrong",
                    "terminus": 0,
                    "acting": [5.0, 3.0],
                    "receiving": [2.0, 4.0]
                }),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["speaker"], "Player");
        assert_eq!(body["act"]["report"], true);
        assert_eq!(body["act"]["question"], true, "Thm 7.5(ii): both at once");
        assert!(body.get("score").is_none(), "Invariant 6");
        assert!(body.get("votes").is_none());
        assert!(body.get("rank").is_none());
    }

    #[tokio::test]
    async fn an_unmeasured_exchange_reports_null_not_inert() {
        let (app, token) = app();
        let (_, body) = send(
            &app,
            post(
                &format!("/v1/session/{token}/posts"),
                serde_json::json!({"body": "no gaps given", "terminus": 1}),
            ),
        )
        .await;
        assert!(
            body["act"].is_null(),
            "absence of a classification is not a classification of absence"
        );
    }

    #[tokio::test]
    async fn one_side_alone_does_not_license_half_a_verdict() {
        let (app, token) = app();
        let (_, body) = send(
            &app,
            post(
                &format!("/v1/session/{token}/posts"),
                serde_json::json!({"body": "half", "terminus": 1, "acting": [5.0, 3.0]}),
            ),
        )
        .await;
        assert!(body["act"].is_null());
    }

    #[tokio::test]
    async fn agents_thread_without_the_player() {
        let (app, token) = app();
        let (_, root) = send(
            &app,
            post(
                &format!("/v1/session/{token}/posts"),
                serde_json::json!({"agent": "kaeferberg", "body": "opening", "terminus": 0}),
            ),
        )
        .await;
        let id = root["id"].as_u64().unwrap();
        send(
            &app,
            post(
                &format!("/v1/session/{token}/posts"),
                serde_json::json!({"agent": "wipkingen", "body": "and?", "terminus": 0, "parent": id}),
            ),
        )
        .await;

        let (status, thread) = send(&app, get(&format!("/v1/session/{token}/thread/{id}"))).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(thread.as_array().unwrap().len(), 2, "Thm 7.4");
        assert!(
            thread
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["speaker"] != "Player"),
            "no user was present"
        );
    }

    #[tokio::test]
    async fn the_world_moves_only_when_asked() {
        let (app, token) = app();
        let (_, before) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert_eq!(before["tick"], 0);
        let (status, t) = send(
            &app,
            post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(t["tick"], 1);
        let (_, after) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert_eq!(after["tick"], 1, "no background scheduler moved it further");
    }

    #[tokio::test]
    async fn the_feed_has_no_top_ordering() {
        let (app, token) = app();
        send(
            &app,
            post(
                &format!("/v1/session/{token}/posts"),
                serde_json::json!({"body": "a", "terminus": 0}),
            ),
        )
        .await;
        let (ok, _) = send(
            &app,
            get(&format!("/v1/session/{token}/posts?order=recent")),
        )
        .await;
        assert_eq!(ok, StatusCode::OK);
        let (rejected, _) = send(&app, get(&format!("/v1/session/{token}/posts?order=top"))).await;
        assert_ne!(
            rejected,
            StatusCode::OK,
            "Prop. 9.6: a cross-agent ranking is ill-defined, so `top` must not parse"
        );
    }

    // ------------------------------------------------------- the world

    fn obs(t: f64, w: f64) -> crate::weather::Observation {
        crate::weather::Observation {
            temperature_c: t,
            wind_kph: w,
            precipitation_mm: 0.0,
            code: 0,
            taken: format!("2026-09-06T18:{:02}", (t.abs() as u32) % 60),
            source: String::from("open-meteo"),
        }
    }

    /// A router whose world reports `o`, and its token.
    fn app_with(o: Option<crate::weather::Observation>) -> (axum::Router, String) {
        let st = AppState::with_weather(crate::weather::Source::Fixed(o));
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        st.open(&token, "zuerich", 20260904);
        (crate::routes::router(st), token.as_str().to_owned())
    }

    #[tokio::test]
    async fn the_default_server_has_no_weather() {
        let (app, token) = app();
        let (_, t) = send(
            &app,
            post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
        )
        .await;
        assert!(t["weather"].is_null(), "hermetic unless asked otherwise");
        let (_, v) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert_eq!(v["nodes"], 0, "no world, no nodes");
    }

    #[tokio::test]
    async fn the_fingerprint_sees_reach_and_not_the_reading() {
        // The heart of it. 26.7 °C and −5 °C are as different as readings
        // get, and both trip exactly the same rules: windy, and temperature
        // at an extreme. Identical reach, so identical protocol — while the
        // bodies a player reads differ completely.
        //
        // This is Theorem truth-blind as an executable claim rather than a
        // comment: the mechanism cannot tell these two runs apart.
        let mut fps = Vec::new();
        let mut termini = Vec::new();
        let mut bodies = Vec::new();
        for o in [obs(26.7, 30.0), obs(-5.0, 30.0)] {
            let (app, token) = app_with(Some(o));
            send(
                &app,
                post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
            )
            .await;
            let (_, v) = send(&app, get(&format!("/v1/session/{token}"))).await;
            fps.push((
                v["protocol_fingerprint"].as_str().unwrap().to_owned(),
                v["nodes"].as_u64().unwrap(),
                v["record"].as_u64().unwrap(),
            ));
            let (_, posts) = send(&app, get(&format!("/v1/session/{token}/posts"))).await;
            let world: Vec<_> = posts
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["speaker"].get("World").is_some())
                .collect();
            assert!(!world.is_empty(), "the world said something");
            termini.push(
                world
                    .iter()
                    .map(|p| p["terminus"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
            );
            bodies.push(
                world
                    .iter()
                    .map(|p| p["body"].as_str().unwrap().to_owned())
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(fps[0], fps[1], "the protocol cannot see the reading");
        assert_eq!(termini[0], termini[1], "reach is identical");
        assert_ne!(bodies[0], bodies[1], "what a reader sees is not identical");
    }

    #[tokio::test]
    async fn the_world_wakes_the_runtime_up() {
        // Before this work `nodes`/`record` were 0 for every session that
        // ever existed, because nothing fed the runtime.
        let (app, token) = app_with(Some(obs(26.7, 30.0)));
        let (_, before) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert_eq!(before["nodes"], 0);
        send(
            &app,
            post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
        )
        .await;
        let (_, after) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert!(after["nodes"].as_u64().unwrap() > 0);
        assert!(after["record"].as_u64().unwrap() > 0);
        assert_ne!(
            after["protocol_fingerprint"].as_str().unwrap(),
            "e3b0c44298fc1c14",
            "no longer the hash of the empty string"
        );
    }

    #[tokio::test]
    async fn a_source_that_is_down_does_not_stop_the_world() {
        // It works offline. A day the world did not report is a day, not an
        // error: there is no exit code to return (Thm 11.5).
        let (app, token) = app_with(None);
        let mut counts = Vec::new();
        for _ in 0..6 {
            let (status, t) = send(
                &app,
                post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            counts.push(t["posts"].as_u64().unwrap());
        }
        assert!(counts.windows(2).all(|w| w[1] > w[0]), "{counts:?}");
        let (_, v) = send(&app, get(&format!("/v1/session/{token}"))).await;
        assert_eq!(v["nodes"], 0, "nothing is invented when nothing reported");
    }

    #[tokio::test]
    async fn a_world_post_is_a_root_and_carries_no_act() {
        let (app, token) = app_with(Some(obs(26.7, 30.0)));
        send(
            &app,
            post(&format!("/v1/session/{token}/tick"), serde_json::json!({})),
        )
        .await;
        let (_, posts) = send(&app, get(&format!("/v1/session/{token}/posts"))).await;
        let world: Vec<_> = posts
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["speaker"].get("World").is_some())
            .collect();
        assert!(
            world.len() > 1,
            "N registrations, one per receiver (Thm 6.2)"
        );
        for p in &world {
            assert!(p["parent"].is_null(), "roots, never a thread (Cor. 6.7)");
            assert!(p["act"].is_null(), "no gaps were measured");
            assert_eq!(p["speaker"]["World"], "open-meteo", "names the source");
        }
    }

    #[tokio::test]
    async fn weather_does_not_move_the_substrate() {
        // Guards the decision not to reweight the city graph. If a reading
        // could move a gap, a character could close its goal (Thm 7.4) and
        // an already-pruned agent's chi would shift with nothing recording
        // why.
        let st = AppState::with_weather(crate::weather::Source::Fixed(Some(obs(-5.0, 120.0))));
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        st.open(&token, "zuerich", 20260904);
        let before = st
            .with_session(&token, |s| {
                s.moderators.iter().map(|m| m.gap()).collect::<Vec<_>>()
            })
            .unwrap();
        for _ in 0..10 {
            let o = st.observation().await;
            st.advance(&token, o.as_ref());
        }
        let after = st
            .with_session(&token, |s| {
                s.moderators.iter().map(|m| m.gap()).collect::<Vec<_>>()
            })
            .unwrap();
        assert_eq!(before, after, "a reading is not a change to the city");
        assert!(
            st.with_session(&token, |s| s.moderators.iter().all(Moderator::is_open))
                .unwrap(),
            "every character still cannot close its goal (Thm 7.4)"
        );
    }

    #[tokio::test]
    async fn a_reached_region_registers_at_its_weakest_point() {
        // Not a position derived from the reading. The observation lands
        // where the region already has an open question, which is the one
        // place it can arrive without being put there by its content.
        let o = obs(26.7, 30.0);
        let st = AppState::with_weather(crate::weather::Source::Fixed(Some(o.clone())));
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        st.open(&token, "zuerich", 20260904);
        // `wants()` is read before the tick, since the round that follows it
        // may move the character's weakest point.
        let want: std::collections::BTreeSet<u32> = st
            .with_session(&token, |s| {
                crate::weather::reach(&o, &s.society)
                    .into_iter()
                    .filter_map(|r| {
                        let m = s.square.subgroups.iter().find(|g| g.name == r)?;
                        s.moderators
                            .iter()
                            .find(|x| x.region == m.members)
                            .and_then(crate::weather::terminus)
                    })
                    .collect()
            })
            .unwrap();
        assert!(!want.is_empty(), "the reach is non-empty");
        let ob = st.observation().await;
        st.advance(&token, ob.as_ref());
        let got: std::collections::BTreeSet<u32> = st
            .with_forum_ref(&token, |f| {
                f.posts()
                    .iter()
                    .filter(|p| matches!(p.speaker, Speaker::World(_)))
                    .map(|p| p.emitted.terminus)
                    .collect()
            })
            .unwrap();
        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn the_trajectory_is_the_reach_set() {
        let o = obs(26.7, 30.0);
        let st = AppState::with_weather(crate::weather::Source::Fixed(Some(o.clone())));
        let mut rng = rand::rng();
        let token = SessionToken::generate(&mut rng);
        st.open(&token, "zuerich", 20260904);
        let ob = st.observation().await;
        st.advance(&token, ob.as_ref());
        let want: std::collections::BTreeSet<String> = st
            .with_session(&token, |s| {
                std::iter::once(format!("weather/{}", o.source))
                    .chain(
                        crate::weather::reach(&o, &s.society)
                            .into_iter()
                            .map(|r| format!("weather/{r}")),
                    )
                    .collect()
            })
            .unwrap();
        let got = st
            .with_session(&token, |s| {
                s.runtime
                    .trajectory()
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap();
        // The trajectory names reach and nothing else: no reading, no time,
        // no region the weather did not touch.
        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn a_speaker_round_trips_on_the_wire() {
        use closure_runtime::voice::VoiceId;
        for (s, want) in [
            (Speaker::Player, serde_json::json!("Player")),
            (Speaker::Voice(VoiceId(3)), serde_json::json!({"Voice": 3})),
            (
                Speaker::World(String::from("open-meteo")),
                serde_json::json!({"World": "open-meteo"}),
            ),
        ] {
            assert_eq!(serde_json::to_value(&s).unwrap(), want);
        }
    }

    #[tokio::test]
    async fn an_unknown_session_is_not_found() {
        let (app, _) = app();
        let mut rng = rand::rng();
        let other = SessionToken::generate(&mut rng);
        let (status, _) = send(&app, get(&format!("/v1/session/{}/posts", other.as_str()))).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    #[tokio::test]
    async fn a_generated_society_hears_weather_wherever_it_is() {
        // The end-to-end claim behind generating the substrate: a reading
        // that is a consideration somewhere registers somewhere, in whatever
        // society the seed happened to draw. The old table named six fixed
        // regions and would have registered nowhere at all here.
        //
        // Storm conditions, chosen to trip every rule, so the only reason a
        // society could hear nothing is that it is entirely under a roof.
        let o = obs(30.0, 40.0);
        let mut heard = 0usize;
        let mut roofed = 0usize;
        let n = 24usize;
        for i in 0..n {
            let seed = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xC17A;
            let st = AppState::with_weather(crate::weather::Source::Fixed(Some(o.clone())));
            let mut rng = rand::rng();
            let token = SessionToken::generate(&mut rng);
            st.open(&token, "zuerich", seed);
            let all_indoors = st
                .with_session(&token, |s| {
                    s.society
                        .regions
                        .iter()
                        .all(|r| r.affords == crate::society::Affordance::Indoors)
                })
                .unwrap();
            let ob = st.observation().await;
            st.advance(&token, ob.as_ref());
            let world = st
                .with_forum_ref(&token, |f| {
                    f.posts()
                        .iter()
                        .filter(|p| matches!(p.speaker, Speaker::World(_)))
                        .count()
                })
                .unwrap();
            if all_indoors {
                roofed += 1;
                assert_eq!(world, 0, "seed {seed}: a roofed society heard weather");
            } else {
                assert!(world > 0, "seed {seed}: nowhere heard a storm");
                heard += 1;
            }
        }
        assert!(heard + roofed == n);
        assert!(
            heard > 0,
            "no society heard anything, which is not a substrate"
        );
    }
}
