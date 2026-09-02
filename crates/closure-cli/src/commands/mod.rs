//! CLI subcommands.

pub mod auth;
pub mod cities;
pub mod doctor;
pub mod session;
pub mod where_cmd;

/// Shared state for every subcommand.
#[derive(Debug, Clone)]
pub struct Context {
    /// Host to talk to.
    pub host: String,
    /// Whether to emit JSON rather than prose.
    pub json: bool,
}

impl Context {
    /// Build a URL against the host. Used once the CLI talks to a live
    /// host rather than the offline scaffold.
    #[allow(dead_code)]
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.host.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    /// Print a payload as JSON, or run `prose` for the human form.
    pub fn emit(&self, value: &serde_json::Value, prose: impl FnOnce()) -> anyhow::Result<()> {
        if self.json {
            println!("{}", serde_json::to_string_pretty(value)?);
        } else {
            prose();
        }
        Ok(())
    }
}
