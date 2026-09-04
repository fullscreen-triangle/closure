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

    let state = state::AppState::new(args.data_dir.clone());

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
