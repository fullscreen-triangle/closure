//! `closure-server` — the Axum host.
//!
//! The server owns the world: sessions, populations, and the monotone records
//! that make agents individuals. The web surface is a thin client over the
//! API in [`routes`].
//!
//! Two API-shaped consequences of the theory are worth stating here, because
//! they will look like omissions to anyone who has not read the manuscript:
//!
//! * There is no `/score`, `/progress`, or `/outcome` endpoint. The runtime
//!   cannot compute a verdict (Theorem 11.5).
//! * There is no endpoint attributing a change to a player action. That
//!   quantity is not expressible from any store of local content
//!   (Theorem 12.9), and the terminal state does not determine the path that
//!   produced it (Theorem 11.15).
//!
//! What the API does expose is the record of what propagated, which is the
//! honest measurement surface for this class of phenomenon.

mod routes;
mod state;
mod substrate;
mod weather;

use anyhow::Result;
use clap::Parser;
use std::net::SocketAddr;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

#[derive(Debug, Parser)]
#[command(
    name = "closure-server",
    version,
    about = "Host for the closure runtime"
)]
struct Args {
    /// Address to bind.
    #[arg(long, env = "CLOSURE_BIND", default_value = "0.0.0.0:8080")]
    bind: SocketAddr,

    /// Directory holding city substrates.
    #[arg(long, env = "CLOSURE_DATA_DIR", default_value = "./data")]
    data_dir: std::path::PathBuf,

    /// Open-Meteo forecast endpoint, **bare** — no query string. The
    /// coordinates and fields are appended, so a URL that already carries a
    /// `?` would produce a malformed request and a world that never reports.
    /// Unset means the world does not report, which keeps a run hermetic and
    /// its post bodies deterministic.
    #[arg(long, env = "CLOSURE_WEATHER_URL")]
    weather_url: Option<String>,

    /// Comma-separated origins permitted to call the API.
    #[arg(
        long,
        env = "CLOSURE_ALLOWED_ORIGINS",
        default_value = "http://localhost:5173"
    )]
    allowed_origins: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    init_tracing();

    let source = match args.weather_url.as_deref() {
        // Refuse at startup rather than serve `weather: null` forever. The
        // query string is appended, so a `?` here silently malforms every
        // request, and a source that is down is indistinguishable from one
        // that was never configured.
        Some(u) if u.contains('?') => {
            anyhow::bail!(
                "--weather-url must be the bare endpoint, without a query string; the coordinates and fields are appended"
            )
        }
        Some(u) => {
            tracing::info!(url = %u, "the world will report itself");
            weather::Source::Live(weather::Live::new(u))
        }
        None => weather::Source::Fixed(None),
    };
    let state = state::AppState::with_weather(args.data_dir.clone(), source);

    let cors = build_cors(&args.allowed_origins)?;
    let app = routes::router(state)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(args.bind).await?;
    tracing::info!(addr = %args.bind, "closure-server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn init_tracing() {
    let filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| String::from("closure_server=info,tower_http=warn"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn build_cors(origins: &str) -> Result<CorsLayer> {
    let list: Vec<_> = origins
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<axum::http::HeaderValue>())
        .collect::<std::result::Result<_, _>>()?;
    Ok(CorsLayer::new()
        .allow_origin(list)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
