//! List the cities a host can instantiate.

use super::Context;
use anyhow::Result;

/// A city binding offered by the host.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct City {
    /// Handle used with `closure session new --city`.
    pub id: String,
    /// Human name.
    pub name: String,
    /// Where the substrate comes from.
    pub substrate: String,
    /// Measured floor of the binding, if the host publishes it.
    #[serde(default)]
    pub floor: Option<f64>,
}

/// Cities the scaffold knows about without contacting a host.
#[must_use]
pub fn builtin() -> Vec<City> {
    vec![City {
        id: "zuerich".into(),
        name: "Zürich".into(),
        substrate: "Statistik Stadt Zürich, open government data".into(),
        floor: Some(0.0197),
    }]
}

/// Run the command.
pub fn run(ctx: &Context) -> Result<()> {
    let cities = builtin();
    let payload = serde_json::to_value(&cities)?;
    ctx.emit(&payload, || {
        for c in &cities {
            println!("{:<10} {}", c.id, c.name);
            println!("           substrate: {}", c.substrate);
            if let Some(f) = c.floor {
                println!("           measured floor: {f:.4}");
            }
        }
    })
}
