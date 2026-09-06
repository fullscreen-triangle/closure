//! The world reporting itself into the square.
//!
//! A weather observation is real-world input, and it is the first thing in
//! this system that comes from outside it. That makes it the sharpest test
//! of the constraint [`crate::substrate`] states: text that carried meaning
//! the mechanism then acted on would be a fifth operation (§11), and there
//! is no fifth.
//!
//! So the observation's **text is display-only**, and its entire mechanical
//! content is the set of regions it reaches. That is the truth-blindness
//! theorem made operational: an observation with arbitrary veridicality but
//! identical reach changes nothing about the run. A player reading the
//! square learns that it is windy; the mechanism learns only that something
//! registered on the water.
//!
//! ## Why it cannot name a region
//!
//! The society is generated per session ([`crate::society`]), so its region
//! names are drawn and a rule naming one would match nothing. The rules key
//! off [`Affordance`] instead, which is the only structured fact a region
//! carries. That is a stronger position than the one it replaced: a rule
//! about wind and water is a claim about weather, where a rule about
//! `watersports` was a claim about one particular city.
//!
//! ## What is deliberately absent
//!
//! * **No reweighting of the city graph.** A storm plausibly raises the cost
//!   of telling two shores apart, and [`ContactGraph::add_edge`] would
//!   overwrite happily. It is still wrong here: [`Moderator`]'s goal is
//!   frozen at construction and every moderator holds its own induced copy,
//!   so a reweight leaves every character stale; a rebuild could let a
//!   character *close* its goal, which Theorem 7.4 forbids; and it would
//!   silently move the `chi` of an already-pruned agent with nothing
//!   recording why. The condition is recorded as an emission instead. This
//!   omission is a decision, not an unfinished edge.
//! * **No terminus derived from a reading.** The temperature chooses *which*
//!   regions are reached, never *where within* one. Deriving a position from
//!   a value would let a client invert the mapping and read the weather off
//!   the terminus, which is content entering the mechanism by the back door.

use crate::society::{Affordance, Society};
use closure_runtime::moderator::Moderator;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Zürich.
const LAT: f64 = 47.3769;
const LON: f64 = 8.5417;

/// How long a reading is served before another fetch is attempted.
///
/// Open-Meteo recomputes every 900 s, so a shorter TTL buys nothing but
/// traffic. A stale reading is not a degraded one: if it trips the same
/// rules it has the same reach, and reach is the whole of what propagates.
const TTL: Duration = Duration::from_secs(600);

/// How long to wait on the network before giving up and serving what we have.
const TIMEOUT: Duration = Duration::from_secs(3);

/// One reading of the city.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    /// Degrees Celsius.
    pub temperature_c: f64,
    /// Kilometres per hour.
    pub wind_kph: f64,
    /// Millimetres.
    pub precipitation_mm: f64,
    /// WMO weather code.
    pub code: u32,
    /// When the source says it was taken. Display only.
    pub taken: String,
    /// Who reported it.
    pub source: String,
}

/// What an observation is a consideration *for*.
///
/// **This table is the entire mechanical content of the weather.** Everything
/// else about an observation is display. Stated here as a table rather than
/// buried inside a formatter, because it is the part that has to be argued
/// for.
///
/// | condition | is a consideration for |
/// |---|---|
/// | wind ≥ 15 km/h | [`Affordance::Water`] |
/// | precipitation > 0 mm | [`Affordance::Transit`] |
/// | temperature ≤ 0 °C or ≥ 25 °C | [`Affordance::Open`], [`Affordance::Transit`] |
/// | code ≥ 45 (fog and worse) | [`Affordance::Water`], [`Affordance::Transit`] |
///
/// ## Why this keys off affordance and not off a region name
///
/// It used to name six regions, because there were six and they were fixed.
/// A society is generated now, and its region names are drawn — so a table
/// of names would match nothing and the weather would silently register
/// nowhere. The deeper reason is the better one: a rule that named
/// `watersports` was not saying anything about weather, it was saying
/// something about that one city. Keyed on affordance, the same four lines
/// are a claim about wind and water that holds in every society the
/// generator can produce.
///
/// [`Affordance::Indoors`] appears in no rule, because a roof is a roof.
/// That absence is the check on the whole scheme: a mapping that touched
/// every region would be one that had stopped distinguishing, and a
/// consideration that reaches everything is not a consideration.
///
/// An observation matching no rule is a consideration for nothing and
/// registers nowhere. That is a day the weather was not a consideration, and
/// nothing is invented to cover it.
#[must_use]
pub fn considers(o: &Observation) -> Vec<Affordance> {
    let mut out: Vec<Affordance> = Vec::new();
    let mut add = |a: Affordance| {
        if !out.contains(&a) {
            out.push(a);
        }
    };
    if o.wind_kph >= 15.0 {
        add(Affordance::Water);
    }
    if o.precipitation_mm > 0.0 {
        add(Affordance::Transit);
    }
    if o.temperature_c <= 0.0 || o.temperature_c >= 25.0 {
        add(Affordance::Open);
        add(Affordance::Transit);
    }
    if o.code >= 45 {
        add(Affordance::Water);
        add(Affordance::Transit);
    }
    out
}

/// Which of `society`'s regions an observation reaches.
///
/// The composition of [`considers`] with the society's own affordances. Two
/// societies fed the same observation reach different regions, and the same
/// society fed two observations that trip the same rules reaches the same
/// ones — which is truth-blindness surviving the move to a generated
/// substrate rather than being a property of the old fixed table.
#[must_use]
pub fn reach(o: &Observation, society: &Society) -> Vec<String> {
    let considered = considers(o);
    society
        .regions
        .iter()
        .filter(|r| considered.contains(&r.affords))
        .map(|r| r.name.clone())
        .collect()
}

/// Where in a region an observation registers.
///
/// The moderator's own weakest point — "which position this character is
/// least able to resolve, and therefore what it will ask about next". The
/// observation lands exactly where the region already has an open question,
/// which is the one place it can arrive without being put there by its
/// content.
#[must_use]
pub fn terminus(m: &Moderator) -> Option<u32> {
    m.wants()
}

/// What a world post says. Display only — nothing reads this.
#[must_use]
pub fn body(o: &Observation, region: &str) -> String {
    format!(
        "[{region}] {:.1} °C, wind {:.0} km/h, {:.1} mm — reported {} by {}",
        o.temperature_c, o.wind_kph, o.precipitation_mm, o.taken, o.source
    )
}

/// Where an observation comes from.
///
/// An enum rather than a trait object: there are only ever two of these, so
/// dynamic dispatch would buy nothing and cost a dependency.
#[derive(Debug)]
pub enum Source {
    /// The live feed.
    Live(Live),
    /// A fixed reading, or none at all. Tests and offline runs.
    Fixed(Option<Observation>),
}

impl Source {
    /// The current observation, or `None` if there is not one to be had.
    ///
    /// Never fails. A source that is down is not an error condition here —
    /// it is a day the world did not report, and the square keeps moving.
    /// The runtime's own convention is the same: an error is an ordinary
    /// emitted value and halts nothing.
    pub async fn current(&self) -> Option<Observation> {
        match self {
            Self::Live(l) => l.current().await,
            Self::Fixed(o) => o.clone(),
        }
    }
}

/// The live Open-Meteo feed. No API key.
#[derive(Debug)]
pub struct Live {
    url: String,
    client: reqwest::Client,
    cache: tokio::sync::Mutex<Option<(Instant, Observation)>>,
}

impl Live {
    /// A live source against `base` (the Open-Meteo forecast endpoint).
    #[must_use]
    pub fn new(base: &str) -> Self {
        let url = format!(
            "{base}?latitude={LAT}&longitude={LON}\
             &current=temperature_2m,wind_speed_10m,precipitation,weather_code\
             &timezone=Europe%2FZurich"
        );
        Self {
            url,
            client: reqwest::Client::builder()
                .timeout(TIMEOUT)
                .build()
                .unwrap_or_default(),
            cache: tokio::sync::Mutex::new(None),
        }
    }

    async fn current(&self) -> Option<Observation> {
        let mut cache = self.cache.lock().await;
        if let Some((at, o)) = cache.as_ref()
            && at.elapsed() < TTL
        {
            return Some(o.clone());
        }
        let fresh = match self.client.get(&self.url).send().await {
            Ok(r) => match r.text().await {
                Ok(b) => {
                    let o = parse(&b);
                    if o.is_none() {
                        tracing::warn!("the world replied with something unreadable");
                    }
                    o
                }
                Err(e) => {
                    tracing::warn!(error = %e, "the world's reply did not arrive");
                    None
                }
            },
            Err(e) => {
                // Not an error path: a day the world did not report is an
                // ordinary day. Logged so a misconfigured source is
                // distinguishable from one that was never asked for.
                tracing::warn!(error = %e, "the world did not report");
                None
            }
        };
        match fresh {
            Some(o) => {
                *cache = Some((Instant::now(), o.clone()));
                Some(o)
            }
            // Serve what we have rather than nothing. A stale reading with
            // the same reach is mechanically identical to a fresh one.
            None => cache.as_ref().map(|(_, o)| o.clone()),
        }
    }
}

/// Parse an Open-Meteo `current` block.
///
/// Returns `None` on anything unexpected rather than panicking or erroring:
/// a malformed reply is a day the world did not report.
#[must_use]
pub fn parse(body: &str) -> Option<Observation> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let c = v.get("current")?;
    Some(Observation {
        temperature_c: c.get("temperature_2m")?.as_f64()?,
        wind_kph: c.get("wind_speed_10m")?.as_f64()?,
        precipitation_mm: c.get("precipitation")?.as_f64()?,
        code: u32::try_from(c.get("weather_code")?.as_u64()?).ok()?,
        taken: c.get("time")?.as_str()?.to_owned(),
        source: String::from("open-meteo"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::society::Society;

    /// A body captured from the live endpoint. Inline, so no test touches
    /// the network.
    const CAPTURED: &str = r#"{"latitude":47.375,"longitude":8.5,
        "current_units":{"time":"iso8601","temperature_2m":"°C"},
        "current":{"time":"2026-09-06T18:30","interval":900,
        "temperature_2m":26.7,"wind_speed_10m":7.5,
        "precipitation":0.00,"weather_code":0}}"#;

    fn obs(t: f64, w: f64, p: f64, code: u32) -> Observation {
        Observation {
            temperature_c: t,
            wind_kph: w,
            precipitation_mm: p,
            code,
            taken: String::from("2026-09-06T18:30"),
            source: String::from("open-meteo"),
        }
    }

    fn seeds() -> impl Iterator<Item = u64> {
        (0..32u64).map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15))
    }

    #[test]
    fn a_captured_reply_parses() {
        let o = parse(CAPTURED).unwrap();
        assert!((o.temperature_c - 26.7).abs() < 1e-9);
        assert!((o.wind_kph - 7.5).abs() < 1e-9);
        assert_eq!(o.code, 0);
        assert_eq!(o.source, "open-meteo");
    }

    #[test]
    fn a_malformed_reply_is_a_day_the_world_did_not_report() {
        assert!(parse("").is_none());
        assert!(parse("{").is_none());
        assert!(parse(r#"{"current":{}}"#).is_none());
        assert!(parse(r#"{"current":{"temperature_2m":"warm"}}"#).is_none());
    }

    #[test]
    fn what_is_under_a_roof_is_never_reached() {
        // Indoors is indoors, in every society the generator can produce.
        // If this ever fails the mapping has stopped distinguishing and the
        // weather has become a global modifier.
        //
        // Note this is now a claim about the *generator*, not about two
        // region names that happened to be spelled a certain way. The test
        // it replaced went vacuously green the moment the names changed.
        for t in [-20.0, -5.0, 0.0, 12.0, 18.0, 25.0, 26.7, 40.0] {
            for w in [0.0, 5.0, 14.9, 15.0, 30.0, 120.0] {
                for p in [0.0, 0.1, 20.0] {
                    for code in [0, 3, 45, 61, 95] {
                        let o = obs(t, w, p, code);
                        assert!(
                            !considers(&o).contains(&Affordance::Indoors),
                            "{t} {w} {p} {code}"
                        );
                        for s in seeds().take(8) {
                            let soc = Society::generate(s);
                            let reached = reach(&o, &soc);
                            for r in &soc.regions {
                                if r.affords == Affordance::Indoors {
                                    assert!(!reached.contains(&r.name), "seed {s}: {}", r.name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn an_ordinary_day_is_a_consideration_for_nothing() {
        // 18 °C, light air, dry, clear. Nothing is invented for a day the
        // weather was not a consideration.
        let o = obs(18.0, 5.0, 0.0, 0);
        assert!(considers(&o).is_empty());
        for s in seeds() {
            assert!(reach(&o, &Society::generate(s)).is_empty());
        }
    }

    #[test]
    fn a_region_is_reached_once_however_many_rules_name_it() {
        // Hot and raining and foggy all consider transit. It is reached
        // once: a broadcast is N registrations, one per receiver, not one
        // per reason.
        let o = obs(30.0, 2.0, 5.0, 45);
        let n = considers(&o)
            .iter()
            .filter(|a| **a == Affordance::Transit)
            .count();
        assert_eq!(n, 1, "got {:?}", considers(&o));
        for s in seeds() {
            let r = reach(&o, &Society::generate(s));
            let mut sorted = r.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), r.len(), "seed {s}: {r:?}");
        }
    }

    #[test]
    fn wind_is_a_consideration_for_water_and_nothing_else() {
        assert_eq!(considers(&obs(18.0, 20.0, 0.0, 0)), vec![Affordance::Water]);
    }

    #[test]
    fn the_reach_is_blind_to_how_extreme_the_reading_is() {
        // Truth-blindness at the level of the table: two readings that trip
        // the same rules are the same input to the mechanism, however
        // different they look to a reader.
        let hot = obs(26.7, 30.0, 0.0, 0);
        let cold = obs(-5.0, 30.0, 0.0, 0);
        assert_eq!(considers(&hot), considers(&cold));
        for s in seeds() {
            let soc = Society::generate(s);
            assert_eq!(reach(&hot, &soc), reach(&cold, &soc));
        }
    }

    #[test]
    fn the_same_weather_reaches_different_places_in_different_societies() {
        // The point of generating: a society is not a relabelling of one
        // fixed city, so the same reading is a consideration in different
        // places depending on where you are.
        let o = obs(30.0, 30.0, 5.0, 45);
        let shapes: std::collections::BTreeSet<Vec<String>> =
            seeds().map(|s| reach(&o, &Society::generate(s))).collect();
        assert!(shapes.len() > 4, "got {} distinct reaches", shapes.len());
    }

    #[test]
    fn a_body_names_the_region_and_carries_no_verdict() {
        let b = body(&obs(26.7, 30.0, 0.0, 0), "harbour-rowing");
        assert!(b.contains("harbour-rowing"));
        assert!(b.contains("26.7"));
        for banned in ["score", "rank", "good", "bad", "warning", "alert"] {
            assert!(!b.to_lowercase().contains(banned), "{b}");
        }
    }
}
