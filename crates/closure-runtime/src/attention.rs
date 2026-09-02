//! Water-filling division of a bounded capacity (Section 5, Algorithm 1).
//!
//! An agent facing several scenes at once cannot attend to all of them. The
//! value-maximising division is a water-filling rule governed by a single
//! shadow price, and that rule *is* presence-everywhere-preoccupied-
//! everywhere: the appearance of split attention is what optimal attention
//! looks like from any one scene (Remark 5.5).
//!
//! This module is the one place in the workspace whose optimality is
//! conditional on the environment rather than on the agent. Theorem 5.2
//! requires concave gain profiles (Axiom 6), a claim about the world. Where
//! a scene offers increasing returns the optimum is concentration, not
//! division (Remark 5.7), and the routine below is then a heuristic rather
//! than an optimum. Callers wanting an obsessive agent get one by supplying
//! a non-concave scene; that is a modelling choice, not a defect.

use serde::{Deserialize, Serialize};

/// A concurrent demand on an agent's capacity.
///
/// The gain profile is `gamma(a) = k * ln(1 + a)`, so `gamma'(a) = k/(1+a)`,
/// which is concave with inverse `k/p - 1`. `richness` is `k`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Stable handle for the scene.
    pub name: String,
    /// Richness `k`: the marginal gain at zero attention.
    pub richness: f64,
}

impl Scene {
    /// A scene of the given richness.
    ///
    /// # Errors
    /// If `richness` is not strictly positive and finite.
    pub fn new(name: impl Into<String>, richness: f64) -> crate::Result<Self> {
        let name = name.into();
        if richness <= 0.0 || !richness.is_finite() {
            return Err(crate::Error::Scene(name));
        }
        Ok(Self { name, richness })
    }

    /// Inverse marginal gain: the attention at which this scene's margin
    /// equals `price`.
    #[must_use]
    fn inverse_marginal(&self, price: f64) -> f64 {
        if price <= 0.0 {
            f64::INFINITY
        } else {
            (self.richness / price - 1.0).max(0.0)
        }
    }
}

/// The result of dividing a budget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Allocation {
    /// Attention given to each scene, in the order supplied.
    pub shares: Vec<f64>,
    /// The shadow price of the budget constraint.
    ///
    /// A single scalar recording how reachable the agent is at this moment.
    /// High price: thinly present in its high-value scenes and absent from
    /// the rest. Low price: forthcoming everywhere at once (Cor. 5.3).
    pub price: f64,
}

impl Allocation {
    /// Scenes receiving positive attention, paired with their share.
    pub fn attended<'a>(&'a self, scenes: &'a [Scene]) -> impl Iterator<Item = (&'a str, f64)> {
        scenes
            .iter()
            .zip(&self.shares)
            .filter(|(_, s)| **s > 1e-12)
            .map(|(sc, s)| (sc.name.as_str(), *s))
    }
}

/// Divide `budget` across `scenes` by water-filling (Algorithm 1).
///
/// Bisection converges because the total demand at a price is continuous and
/// non-increasing in that price, so a unique price meets the budget.
#[must_use]
pub fn water_fill(budget: f64, scenes: &[Scene]) -> Allocation {
    let alloc_at = |p: f64| -> Vec<f64> {
        scenes
            .iter()
            .map(|s| {
                if s.richness > p {
                    s.inverse_marginal(p)
                } else {
                    0.0
                }
            })
            .collect()
    };

    if scenes.is_empty() || budget <= 0.0 {
        return Allocation {
            shares: vec![0.0; scenes.len()],
            price: 0.0,
        };
    }

    // If the budget is not binding, its multiplier is zero (Thm 5.2).
    let unconstrained = alloc_at(0.0);
    if unconstrained.iter().sum::<f64>() <= budget {
        return Allocation {
            shares: unconstrained,
            price: 0.0,
        };
    }

    let mut lo = 0.0_f64;
    let mut hi = scenes.iter().map(|s| s.richness).fold(0.0_f64, f64::max);
    for _ in 0..200 {
        if hi - lo <= 1e-12 {
            break;
        }
        let mid = 0.5 * (lo + hi);
        if alloc_at(mid).iter().sum::<f64>() > budget {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let price = 0.5 * (lo + hi);
    Allocation {
        shares: alloc_at(price),
        price,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenes() -> Vec<Scene> {
        [
            ("work", 3.2),
            ("family", 2.4),
            ("player", 1.7),
            ("verein", 1.1),
            ("post", 0.6),
        ]
        .into_iter()
        .map(|(n, k)| Scene::new(n, k).unwrap())
        .collect()
    }

    #[test]
    fn margins_equalise_on_the_support() {
        let sc = scenes();
        let a = water_fill(1.5, &sc);
        let margins: Vec<f64> = sc
            .iter()
            .zip(&a.shares)
            .filter(|(_, s)| **s > 1e-9)
            .map(|(s, share)| s.richness / (1.0 + share))
            .collect();
        assert!(margins.len() > 1, "several scenes attended");
        let (lo, hi) = (
            margins.iter().copied().fold(f64::INFINITY, f64::min),
            margins.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        );
        assert!(hi - lo < 1e-6, "Theorem 5.2: one price on the support");
    }

    #[test]
    fn price_falls_as_capacity_rises() {
        let sc = scenes();
        let busy = water_fill(0.4, &sc).price;
        let idle = water_fill(4.0, &sc).price;
        assert!(
            idle < busy,
            "Theorem 5.2: price is non-increasing in budget"
        );
    }

    #[test]
    fn poor_scenes_are_dropped_below_the_price() {
        let sc = scenes();
        let a = water_fill(0.3, &sc);
        assert!(a.shares.last().copied().unwrap() <= 1e-12, "priced out");
        assert!(a.shares[0] > 0.0, "the richest scene is attended");
    }

    #[test]
    fn a_scene_just_above_the_price_gets_a_sliver() {
        // Corollary 5.4: the formal minimal acknowledgement -- positive but
        // arbitrarily small. This is the agent that answers while manifestly
        // being somewhere else.
        //
        // Rather than assert a sliver at some hand-picked budget, drive the
        // budget to the exact point where a scene enters: as the price falls
        // to a scene's richness its allocation approaches zero from above.
        let sc = scenes();
        let entering = sc[2].richness; // the third scene, k = 1.7

        // Approach the entry threshold from above: shrinking the budget
        // raises the price back toward the scene's richness, so its share
        // falls continuously to zero rather than dropping off a step.
        let mut prev = 0.0_f64;
        let mut observed = false;
        for step in 0..8 {
            let budget = 1.60 - 0.03 * f64::from(step);
            let a = water_fill(budget, &sc);
            let share = a.shares[2];
            if a.price < entering {
                assert!(
                    share > 0.0,
                    "attended once the price drops below its richness"
                );
                assert!(share < 0.2, "and the allocation is a sliver: {share}");
                if observed {
                    assert!(share < prev, "share falls as the budget tightens");
                }
                prev = share;
                observed = true;
            }
        }
        assert!(
            observed,
            "the third scene was attended somewhere in the sweep"
        );
    }

    #[test]
    fn non_positive_richness_is_refused() {
        assert!(Scene::new("x", 0.0).is_err());
    }
}
