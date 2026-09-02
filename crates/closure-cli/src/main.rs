//! `closure` — the command-line client.
//!
//! The CLI does three things: it authenticates you against a host, it mints a
//! session token for a city, and it opens the web surface. The world itself —
//! the agents, their graphs, their records — lives on the host, so that a
//! session survives you closing your laptop and so that several people can
//! inhabit one city.
//!
//! Run `closure --help` for the command list, or see `docs/CLI.md`.

mod commands;
mod config;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// A runtime for non-convergent social deliberation.
#[derive(Debug, Parser)]
#[command(
    name = "closure",
    version,
    about = "Talk to a city that is not waiting for you.",
    long_about = "closure connects you to a running deliberative population.\n\
                  \n\
                  There is no score and no win state. The instrument reports \
                  what propagated; it does not report whether you succeeded, \
                  and it cannot tell you which of your actions mattered."
)]
struct Cli {
    /// Host to talk to.
    #[arg(
        long,
        env = "CLOSURE_HOST",
        default_value = "https://api.closure.city",
        global = true
    )]
    host: String,

    /// Emit machine-readable JSON instead of prose.
    #[arg(long, global = true)]
    json: bool,

    /// Increase logging; repeat for more.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Authenticate this machine against the host.
    Login,

    /// Forget the stored credential.
    Logout,

    /// Start a session in a city and print the token to paste into the web app.
    Session(commands::session::Args),

    /// List the cities this host can instantiate.
    Cities,

    /// Report the six implementation invariants and whether the host honours them.
    Doctor,

    /// Print where configuration and credentials are stored.
    Where,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let ctx = commands::Context {
        host: cli.host,
        json: cli.json,
    };

    match cli.command {
        Command::Login => commands::auth::login(&ctx),
        Command::Logout => commands::auth::logout(&ctx),
        Command::Session(args) => commands::session::run(&ctx, &args),
        Command::Cities => commands::cities::run(&ctx),
        Command::Doctor => commands::doctor::run(&ctx),
        Command::Where => commands::where_cmd::run(&ctx),
    }
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => "closure=warn",
        1 => "closure=info",
        2 => "closure=debug",
        _ => "closure=trace",
    };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| level.to_owned());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_writer(std::io::stderr)
        .init();
}
