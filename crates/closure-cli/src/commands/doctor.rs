//! Report the six implementation invariants.
//!
//! This is the command a reviewer runs. It prints what the runtime commits to
//! and what certifies each commitment, so that the guarantees are auditable
//! without reading the manuscript.
//!
//! Note what `doctor` does *not* do: it reports no pass/fail verdict on a
//! session, because Invariant 6 forbids the system from comparing an achieved
//! determination to an expected one. It reports the commitments themselves.

use super::Context;
use anyhow::Result;
use closure_kernel::INVARIANTS;

pub fn run(ctx: &Context) -> Result<()> {
    let payload = serde_json::json!({
        "invariants": INVARIANTS.iter().map(|i| serde_json::json!({
            "index": i.index,
            "name": i.name,
            "predicate": i.predicate,
            "certified_by": i.certified_by,
        })).collect::<Vec<_>>(),
        "refusals": [
            {
                "capability": "success or failure verdict",
                "reason": "no quantity computable from the runtime state compares \
                           an achieved state to an expectation",
                "certified_by": "Theorem 11.5",
            },
            {
                "capability": "attribution of an outcome to a consideration",
                "reason": "admissibility is a global minimum expressed in local terms, \
                           and the terminus does not determine the trajectory",
                "certified_by": "Theorem 12.9, Theorem 11.15",
            },
            {
                "capability": "instance-fitted effect sizes",
                "reason": "prediction and measurement are the same algebraic expression, \
                           so the statistic is degenerate on every data set",
                "certified_by": "Theorem 12.2",
            },
        ],
    });

    ctx.emit(&payload, || {
        println!();
        println!("  Implementation invariants");
        println!();
        for i in &INVARIANTS {
            println!("  {}. {}", i.index, i.name);
            println!("     {}", i.predicate);
            println!("     certified by {}", i.certified_by);
            println!();
        }
        println!("  Refusals");
        println!();
        println!("  This instrument does not report success or failure, does not");
        println!("  attribute an outcome to any action you took, and does not");
        println!("  expose instance-fitted effect sizes. Each refusal is a");
        println!("  consequence of the theory, not a missing feature: a version");
        println!("  reporting them would be evidence the model is wrong.");
        println!();
    })
}
