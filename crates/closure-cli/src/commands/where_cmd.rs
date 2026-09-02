//! Report where configuration lives.

use super::Context;
use anyhow::Result;

/// Run the command.
pub fn run(ctx: &Context) -> Result<()> {
    let dir = crate::config::dir()?;
    let file = crate::config::path()?;
    let payload = serde_json::json!({
        "config_dir": dir.display().to_string(),
        "config_file": file.display().to_string(),
        "exists": file.exists(),
    });
    ctx.emit(&payload, || {
        println!("config directory : {}", dir.display());
        println!("config file      : {}", file.display());
        println!("exists           : {}", file.exists());
    })
}
