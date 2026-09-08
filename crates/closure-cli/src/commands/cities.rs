//! Report what a city is here, which is not a list of them.

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

/// What the scaffold can say about cities without contacting a host.
///
/// One entry, and it is not a city. The host draws a society from the seed
/// and the name, so every name opens a world and no name opens a prepared
/// one — there is nothing to enumerate. Listing a handful of real cities
/// would suggest those were bound and the rest refused, and neither is true.
///
/// No floor is published. A floor stated before the seed is known would
/// describe no world in particular. `floor` stays `Option` because a host
/// that *did* bind a fixed substrate could honestly publish one; this one
/// does not.
#[must_use]
pub fn builtin() -> Vec<City> {
    vec![City {
        id: "any".into(),
        name: "any city you name".into(),
        substrate: "generated per session from the seed and the name; no city is modelled".into(),
        floor: None,
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
        println!();
        println!("  Pass any name to `closure session new --city`. The name and");
        println!("  your token together draw the square, so a different name is a");
        println!("  different city -- and no name is a city anyone prepared.");
    })
}
