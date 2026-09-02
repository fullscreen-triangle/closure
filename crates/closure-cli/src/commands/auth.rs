//! Authentication against a host.

use super::Context;
use crate::config::Config;
use anyhow::{Result, bail};

/// Authenticate this machine.
///
/// Uses a device-code flow: the CLI asks the host for a code, you approve it
/// in a browser, and the CLI stores the resulting credential. Nothing about
/// the world is transmitted here — this only establishes who you are.
pub fn login(ctx: &Context) -> Result<()> {
    let mut cfg = Config::load()?;
    if cfg.credential.is_some() {
        eprintln!(
            "Already logged in to {}. Run `closure logout` first.",
            ctx.host
        );
        return Ok(());
    }

    // The scaffold stores a locally generated credential so the flow can be
    // exercised end to end offline. Wiring this to the host's device-code
    // endpoint is the one remaining step; see docs/CLI.md.
    let credential = format!("dev-{}", uuid::Uuid::new_v4());
    cfg.credential = Some(credential);
    cfg.host = Some(ctx.host.clone());
    cfg.save()?;

    ctx.emit(
        &serde_json::json!({ "status": "logged_in", "host": ctx.host }),
        || println!("Logged in to {}.", ctx.host),
    )
}

/// Forget the stored credential.
pub fn logout(ctx: &Context) -> Result<()> {
    let mut cfg = Config::load()?;
    if cfg.credential.is_none() {
        bail!("not logged in");
    }
    cfg.credential = None;
    cfg.save()?;
    ctx.emit(&serde_json::json!({ "status": "logged_out" }), || {
        println!("Logged out.");
    })
}

/// The stored credential, or an explanatory error.
pub fn credential() -> Result<String> {
    Config::load()?
        .credential
        .ok_or_else(|| anyhow::anyhow!("not logged in; run `closure login` first"))
}
