//! Where the CLI keeps its configuration and credential.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Stored credential and preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Bearer credential obtained by `closure login`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<String>,
    /// Last host used, so `--host` need not be repeated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Web surface to open after a session is minted.
    #[serde(default = "default_web")]
    pub web: String,
}

fn default_web() -> String {
    String::from("https://play.closure.city")
}

/// Directory holding config and credential.
pub fn dir() -> Result<PathBuf> {
    if let Ok(explicit) = std::env::var("CLOSURE_CONFIG_DIR") {
        return Ok(PathBuf::from(explicit));
    }
    let d = directories::ProjectDirs::from("city", "closure", "closure")
        .context("could not determine a configuration directory for this platform")?;
    Ok(d.config_dir().to_path_buf())
}

/// Path to the config file.
pub fn path() -> Result<PathBuf> {
    Ok(dir()?.join("config.toml"))
}

impl Config {
    /// Load, returning defaults if nothing is stored yet.
    pub fn load() -> Result<Self> {
        let p = path()?;
        if !p.exists() {
            return Ok(Self {
                web: default_web(),
                ..Self::default()
            });
        }
        let raw =
            std::fs::read_to_string(&p).with_context(|| format!("reading {}", p.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing {}", p.display()))
    }

    /// Persist, creating the directory if needed.
    pub fn save(&self) -> Result<()> {
        let d = dir()?;
        std::fs::create_dir_all(&d).with_context(|| format!("creating {}", d.display()))?;
        let p = path()?;
        let raw = toml::to_string_pretty(self)?;
        std::fs::write(&p, raw).with_context(|| format!("writing {}", p.display()))?;
        restrict(&p);
        Ok(())
    }
}

/// Tighten permissions on the credential file where the platform supports it.
#[cfg(unix)]
fn restrict(p: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_p: &std::path::Path) {}
