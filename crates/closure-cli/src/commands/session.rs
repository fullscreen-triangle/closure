//! Start a session and mint the token the web surface asks for.

use super::Context;
use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use closure_kernel::SessionToken;

/// `closure session ...`
#[derive(Debug, ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Mint a token for a new session.
    New {
        /// Which city to inhabit.
        #[arg(long, default_value = "zuerich")]
        city: String,

        /// Print the token only, with no surrounding prose. Useful in scripts.
        #[arg(long)]
        quiet: bool,

        /// Do not attempt to open a browser.
        #[arg(long)]
        no_open: bool,
    },

    /// Show the token for the current session.
    Show,
}

pub fn run(ctx: &Context, args: &Args) -> Result<()> {
    match &args.command {
        Cmd::New {
            city,
            quiet,
            no_open,
        } => new(ctx, city, *quiet, *no_open),
        Cmd::Show => show(ctx),
    }
}

fn new(ctx: &Context, city: &str, quiet: bool, no_open: bool) -> Result<()> {
    let _credential = super::auth::credential()?;

    let mut rng = rand::rng();
    let token = SessionToken::generate(&mut rng);

    let cfg = crate::config::Config::load()?;
    let url = format!("{}/join?token={}", cfg.web.trim_end_matches('/'), token);

    if quiet {
        println!("{token}");
        return Ok(());
    }

    let payload = serde_json::json!({
        "token": token.as_str(),
        "city": city,
        "host": ctx.host,
        "url": url,
    });

    ctx.emit(&payload, || {
        println!();
        println!("  Session opened in {city}.");
        println!();
        println!("      {token}");
        println!();
        println!("  Paste that at {}", cfg.web);
        println!("  or open {url}");
        println!();
        // Said once, plainly, at the only moment a player is guaranteed to
        // read it. The instrument cannot report success and cannot attribute
        // an outcome to an action; a player who expects either will misread
        // everything that follows.
        println!("  There is no score. The city is not waiting for you, and");
        println!("  nothing here will tell you whether you are getting");
        println!("  anywhere -- only what happened.");
        println!();
    })?;

    if !no_open {
        let _ = open_browser(&url);
    }
    Ok(())
}

fn show(ctx: &Context) -> Result<()> {
    // A token names a live session on the host; the scaffold has no local
    // store of one yet, so this reports honestly rather than inventing it.
    ctx.emit(
        &serde_json::json!({ "session": serde_json::Value::Null }),
        || println!("No local session recorded. Run `closure session new`."),
    )
}

/// Best-effort browser open. Failure is not an error: the URL was printed.
fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    };
    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
}
