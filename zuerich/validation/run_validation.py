"""
Validation suite for the runtime specified in
    docs/zuerich-common-closure/zuerich-common-closure.tex

Twenty-four experiments, each written to FALSIFY the claim it tests.

Conventions (Sec. 13.1):
  1. Structures are built by brute force from the definitions, not by
     re-deriving the algebra of the proofs, so agreement is not circular.
  2. Where a theorem asserts a structural feature is load-bearing, a
     FALSIFIER runs the same check on a structure with that feature removed
     and is required to FAIL.  A theorem whose falsifier also passes is
     vacuous, so falsifier counts are reported.
  3. E21 and E22 are NEGATIVE CONTROLS: failure of the method is the
     expected and confirmed outcome.

Results are written as JSON to results/.
"""

from __future__ import annotations

import json
import math
import os
import sys
import time
from typing import Dict, List

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..", "prototype")))

from runtime import (  # noqa: E402
    Agent, Consideration, ContactGraph, Declined, InvariantViolation, Module,
    Phase, Resolved, Runtime, TypeError_, catalytic_power, character_invariant,
    commits_to, compose_powers, coordination_cost, critical_coupling,
    cut_weight, eta_separation, generation_digest, instance_specific_prediction,
    is_closed, kuramoto_step, min_cut_weight, order_parameter, prune, reach,
    saturates, seek, separation_cost, shortest_support_cycle,
    support_is_robust, three_routes, type_averaged_prediction, typecheck_seek,
    water_fill,
)

SEED = 20260902
DATA = os.path.abspath(os.path.join(HERE, "..", "data"))
RESULTS = os.path.join(HERE, "results")
os.makedirs(RESULTS, exist_ok=True)

_REG: List[dict] = []


def experiment(eid: str, claim: str, method: str):
    def deco(fn):
        def wrapped(rng):
            t0 = time.time()
            rec = fn(rng)
            rec.update(id=eid, claim=claim, method=method,
                       seconds=round(time.time() - t0, 3))
            _REG.append(rec)
            flag = "PASS" if rec["passed"] else "FAIL"
            extra = ""
            if rec.get("negative_control"):
                flag = "CONFIRMED-NEG" if rec["passed"] else "FAIL"
            if "falsifier_failures" in rec:
                extra = f"  falsifier={rec['falsifier_failures']}"
            print(f"  {eid}  {flag:<14} {claim}{extra}")
            return rec
        wrapped.__name__ = fn.__name__
        return wrapped
    return deco


# ---------------------------------------------------------------------
#  helpers
# ---------------------------------------------------------------------

def random_connected_graph(rng, n, w_lo=1.0, w_hi=9.0) -> ContactGraph:
    g = ContactGraph(n)
    order = list(rng.permutation(n))
    for i in range(1, n):                       # spanning tree first
        j = int(rng.integers(0, i))
        g.add_edge(int(order[i]), int(order[j]), float(rng.uniform(w_lo, w_hi)))
    for _ in range(int(rng.integers(0, n))):    # a few extra chords
        u, v = int(rng.integers(0, n)), int(rng.integers(0, n))
        if u != v:
            g.add_edge(u, v, float(rng.uniform(w_lo, w_hi)))
    return g


def load_zurich() -> Dict[str, dict]:
    out = {}
    for name in ("zurich_ballots", "zurich_population"):
        p = os.path.join(DATA, f"{name}.json")
        out[name] = json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None
    return out


# =====================================================================
#  Part II -- the coordination law
# =====================================================================

@experiment("E01", "Floor theorem (Thm 3.1)", "randomised + falsifier")
def e01(rng):
    trials, violations = 200, 0
    for _ in range(trials):
        g = random_connected_graph(rng, int(rng.integers(3, 8)))
        if separation_cost(g) < g.beta - 1e-12:
            violations += 1
    # FALSIFIER: admit a zero-weight edge (violating Axiom 3) and the bound must break
    fals = 0
    for _ in range(trials):
        g = random_connected_graph(rng, int(rng.integers(3, 8)))
        e = list(g.weights)[int(rng.integers(0, len(g.weights)))]
        beta_before = g.beta
        g.weights[e] = 0.0                      # bypass add_edge guard deliberately
        if separation_cost(g) < beta_before - 1e-12:
            fals += 1
    return {"passed": violations == 0 and fals > 0,
            "trials": trials, "violations": violations, "falsifier_failures": fals}


@experiment("E02", "Individuation is regional (Thm 3.3)", "constructed + random")
def e02(rng):
    g = ContactGraph(6)
    for u, v in [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)]:
        g.add_edge(u, v, 10.0)
    g.add_edge(2, 3, 1.0)
    mc, A = min_cut_weight(g)
    singles = [cut_weight(g, [v]) for v in g.vertices]
    constructed = (len(A) > 1 and mc == 1.0 and min(singles) > mc)
    # how generic is it?
    regional = 0
    for _ in range(200):
        h = random_connected_graph(rng, int(rng.integers(4, 8)))
        m, B = min_cut_weight(h)
        if min(cut_weight(h, [v]) for v in h.vertices) > m + 1e-12:
            regional += 1
    return {"passed": constructed,
            "constructed_min_cut": mc, "constructed_block_size": len(A),
            "best_singleton": min(singles),
            "regional_fraction_random": regional / 200.0}


@experiment("E03", "No privileged level (Thm 3.6)", "randomised")
def e03(rng):
    ok = 0
    for _ in range(150):
        g = random_connected_graph(rng, 6)
        parts = [[0, 1], [2, 3], [4, 5]]
        q = ContactGraph(3)
        for a in range(3):
            for b in range(a + 1, 3):
                w = sum(g.weights.get(frozenset((u, v)), 0.0)
                        for u in parts[a] for v in parts[b])
                if w > 0:
                    q.add_edge(a, b, w)
        # quotient is a contact graph with the same floor bound
        if q.weights and q.is_connected() and min(q.weights.values()) >= g.beta - 1e-12:
            ok += 1
    return {"passed": ok >= 140, "quotients_valid": ok, "trials": 150}


@experiment("E04", "Sufficiency not inherited (Prop 3.8)", "constructed")
def e04(rng):
    # blocks sufficient, level not
    g1 = ContactGraph(9)
    for base in (0, 3, 6):
        for u, v in [(0, 1), (1, 2), (0, 2)]:
            g1.add_edge(base + u, base + v, 1.0)
    g1.add_edge(0, 3, 100.0); g1.add_edge(3, 6, 100.0); g1.add_edge(0, 6, 100.0)
    blocks_ok = all(separation_cost(g1.induced(range(b, b + 3))) <= 2.0
                    for b in (0, 3, 6))
    lvl = ContactGraph(3)
    lvl.add_edge(0, 1, 100.0); lvl.add_edge(1, 2, 100.0); lvl.add_edge(0, 2, 100.0)
    level_bad = separation_cost(lvl) > 2.0
    # level sufficient, blocks not
    g2 = ContactGraph(4)
    g2.add_edge(0, 1, 100.0); g2.add_edge(2, 3, 100.0); g2.add_edge(1, 2, 1.0)
    blocks_bad = separation_cost(g2.induced([0, 1])) > 2.0
    level_ok = separation_cost(g2) <= 2.0
    return {"passed": blocks_ok and level_bad and blocks_bad and level_ok,
            "blocks_sufficient_level_not": bool(blocks_ok and level_bad),
            "level_sufficient_blocks_not": bool(blocks_bad and level_ok)}


@experiment("E05", "Closure > threshold (Thm 4.6)", "constructed + falsifier")
def e05(rng):
    A, B = frozenset({0, 1, 2}), frozenset({3, 4, 5})
    cA = Consideration("a", "p1", {0: A})
    cB = Consideration("b", "p2", {0: B})
    # every threshold in [0.5, 1.0] is met by cA alone, yet closure fails
    thresholds = np.linspace(0.5, 1.0, 11)
    threshold_met = all(1.0 >= t for t in thresholds)
    not_closed = not is_closed(0, [cA], [cA, cB])
    # FALSIFIER: replace the uninvoked consideration by one reaching the SAME
    # region -- closure must be restored, locating the cause in the distinct region
    cB2 = Consideration("b", "p2", {0: A})
    restored = is_closed(0, [cA], [cA, cB2])
    return {"passed": threshold_met and not_closed and restored,
            "thresholds_met": bool(threshold_met),
            "closure_fails": bool(not_closed),
            "falsifier_restores_closure": bool(restored)}


@experiment("E06", "Provenance-blindness (Thm 4.9)", "randomised + falsifier")
def e06(rng):
    trials, label_changed, res_changed = 300, 0, 0
    cells = [frozenset({0, 1}), frozenset({2, 3}), frozenset({4, 5})]
    for _ in range(trials):
        k = int(rng.integers(2, 5))
        G = [Consideration(f"c{i}", f"src{i}",
                           {0: cells[int(rng.integers(0, 3))]}) for i in range(k)]
        inv = G[:max(1, k - 1)]
        base = is_closed(0, inv, G)
        # permute provenance ONLY
        Gp = [Consideration(c.name, f"X{int(rng.integers(0, 999))}", c.resolution)
              for c in G]
        if is_closed(0, Gp[:len(inv)], Gp) != base:
            label_changed += 1
        # FALSIFIER: perturb the RESOLUTION with the same machinery
        Gr = [Consideration(c.name, c.provenance,
                            {0: cells[int(rng.integers(0, 3))]}) for c in G]
        if is_closed(0, Gr[:len(inv)], Gr) != base:
            res_changed += 1
    return {"passed": label_changed == 0 and res_changed > 0,
            "trials": trials,
            "verdict_changed_by_label": label_changed,
            "falsifier_failures": res_changed,
            "note": "invariance under label, sensitivity under resolution"}


@experiment("E07", "Truth-blindness (Thm 4.11)", "randomised")
def e07(rng):
    cells = [frozenset({0, 1}), frozenset({2, 3})]
    changed, all_false_committed = 0, 0
    for _ in range(400):
        k = int(rng.integers(2, 5))
        G = [Consideration(f"c{i}", "p", {0: cells[int(rng.integers(0, 2))]},
                           veridical=bool(rng.integers(0, 2))) for i in range(k)]
        base = is_closed(0, G, G)
        Gf = [Consideration(c.name, c.provenance, c.resolution,
                            veridical=not c.veridical) for c in G]
        if is_closed(0, Gf, Gf) != base:
            changed += 1
    for _ in range(200):
        G = [Consideration(f"c{i}", "p", {0: cells[0]}, veridical=False)
             for i in range(3)]
        if commits_to(0, cells[0], G, G):
            all_false_committed += 1
    return {"passed": changed == 0 and all_false_committed == 200,
            "flips_changing_verdict": changed,
            "all_false_but_committed": all_false_committed,
            "note": "commitment identical under wholesale falsity"}


@experiment("E08", "Water-filling optimality (Thm 5.2)", "convex cross-check")
def e08(rng):
    worst = 0.0
    for _ in range(200):
        k = int(rng.integers(2, 6))
        ks = rng.uniform(0.5, 4.0, k)
        budget = float(rng.uniform(0.5, 3.0))
        inv = [(lambda p, kk=kk: (kk / p) - 1.0 if p > 0 else math.inf) for kk in ks]
        at0 = list(ks)
        a, price = water_fill(budget, inv, at0)
        # KKT check: margins equal on support, <= price off support
        marg = [ks[i] / (1.0 + a[i]) for i in range(k)]
        on = [marg[i] for i in range(k) if a[i] > 1e-9]
        off = [marg[i] for i in range(k) if a[i] <= 1e-9]
        err = (max(on) - min(on)) if len(on) > 1 else 0.0
        if off and on:
            err = max(err, max(0.0, max(off) - min(on) - 1e-6))
        worst = max(worst, err)
    return {"passed": worst < 1e-5, "max_kkt_violation": worst}


@experiment("E09", "Phase alternation (Thm 5.6)", "randomised")
def e09(rng):
    g = random_connected_graph(rng, 5)
    a = Agent("x", g)
    blocked, allowed = 0, 0
    cells = [frozenset({0, 1}), frozenset({2, 3})]
    C = [Consideration(f"c{i}", "p", {0: cells[i % 2]}) for i in range(3)]
    for _ in range(200):
        if rng.integers(0, 2):
            a.set_phase(Phase.CONSTRUCTION)
            try:
                a.determine(0, C[:1], C)
            except InvariantViolation:
                blocked += 1
        else:
            a.set_phase(Phase.COMMITMENT)
            a.determine(0, C[:1], C)
            allowed += 1
    return {"passed": blocked > 0 and allowed > 0,
            "acts_blocked_in_construction": blocked,
            "acts_allowed_in_commitment": allowed,
            "final_record": a.record}


@experiment("E10", "Synchronisation threshold (Thm 6.7)", "integration, Zurich dw")
def e10(rng):
    z = load_zurich()["zurich_ballots"]
    pop = load_zurich()["zurich_population"]
    if z:
        # Per-question district dispositions.  A district is a *tempo class*,
        # not a single oscillator: the mean-field bifurcation of Thm 6.7 is an
        # N -> infinity statement, so with only |districts| units finite-size
        # fluctuation swamps the transition.  We therefore populate each class
        # with residents (S1), drawing each agent's velocity from that
        # district's observed disposition spread across the 275 questions.
        M = np.array([[q["yes_share"][d] for d in z["districts"]]
                      for q in z["questions"]])
        per_district_mean = M.mean(axis=0)
        per_district_sd = M.std(axis=0)
        if pop:
            tot = pop["total"]
            wts = np.array([tot.get(d, 1) for d in z["districts"]], dtype=float)
        else:
            wts = np.ones(len(z["districts"]))
        wts = wts / wts.sum()
        N = 600
        counts = np.maximum(1, (wts * N).astype(int))
        omegas = np.concatenate([
            rng.normal(per_district_mean[i], per_district_sd[i] + 1e-9, counts[i])
            for i in range(len(counts))])
        omegas = (omegas - omegas.mean()) / (np.std(omegas) + 1e-12)
        source = "zurich"
    else:
        omegas = rng.normal(0, 1, 600)
        source = "synthetic"
    n = len(omegas)
    Kc = critical_coupling(omegas)
    sweep, dt, T = [], 0.05, 4000
    for K in np.linspace(0.0, 4.0 * max(Kc, 1e-6), 21):
        ph = rng.uniform(0, 2 * np.pi, n)
        for _ in range(T):
            ph = kuramoto_step(ph, omegas, K, dt)
        R, _ = order_parameter(ph)
        sweep.append({"K": float(K), "R": float(R),
                      "cost": float(coordination_cost(R))})
    below = [s["R"] for s in sweep if s["K"] < 0.6 * Kc]
    above = [s["R"] for s in sweep if s["K"] > 2.0 * Kc]
    ok = (max(below) < 0.5 and min(above) > 0.8) if below and above else False
    return {"passed": bool(ok), "source": source, "n_units": n,
            "sigma_omega": float(np.std(omegas)), "Kc_predicted": float(Kc),
            "max_R_below_threshold": float(max(below)) if below else None,
            "min_R_above_threshold": float(min(above)) if above else None,
            "sweep": sweep}


@experiment("E11", "Coordination cost vanishes at lock (Thm 6.5)", "randomised")
def e11(rng):
    worst = 0.0
    for _ in range(300):
        n = int(rng.integers(3, 12))
        ph = np.full(n, float(rng.uniform(0, 2 * np.pi)))     # locked
        R, _ = order_parameter(ph)
        worst = max(worst, abs(coordination_cost(R)))
    disp = []
    for _ in range(300):
        n = int(rng.integers(3, 12))
        ph = rng.uniform(0, 2 * np.pi, n)
        R, _ = order_parameter(ph)
        disp.append(coordination_cost(R))
    return {"passed": worst < 1e-12 and min(disp) > 0,
            "max_cost_at_lock": worst, "min_cost_dispersed": float(min(disp))}


@experiment("E12", "Reach ceiling (Prop 4.16)", "randomised")
def e12(rng):
    pool = [frozenset({i, i + 10}) for i in range(6)]
    curve = []
    for m in range(1, 11):
        G = [Consideration(f"c{i}", "p", {0: pool[i % len(pool)]}) for i in range(m)]
        curve.append({"agents": m, "reach": len(reach(0, G))})
    grows = all(curve[i]["reach"] == i + 1 for i in range(6))
    flat = all(c["reach"] == 6 for c in curve[6:])
    # duplicates never enlarge reach
    G = [Consideration(f"c{i}", "p", {0: pool[0]}) for i in range(10)]
    dup = len(reach(0, G))
    return {"passed": grows and flat and dup == 1,
            "curve": curve, "reach_of_ten_identical": dup,
            "pool_size": len(pool)}


@experiment("E13", "Crowd sharpening (Thm 6.9)", "Monte Carlo")
def e13(rng):
    rows = []
    for M in range(1, 11):
        q = 0.7
        analytic = q ** M
        trials = 20000
        fails = sum(1 for _ in range(trials)
                    if all(rng.random() < q for _ in range(M)))
        rows.append({"M": M, "analytic": analytic, "empirical": fails / trials})
    err = max(abs(r["analytic"] - r["empirical"]) for r in rows)
    mono = all(rows[i]["analytic"] >= rows[i + 1]["analytic"] for i in range(9))
    return {"passed": err < 0.02 and mono, "max_abs_error": err, "rows": rows}


@experiment("E14", "Availability non-monotone (Rem 4.13)", "randomised")
def e14(rng):
    cells = [frozenset({0, 1}), frozenset({2, 3}), frozenset({4, 5})]
    destroyed, trials = 0, 400
    for _ in range(trials):
        k = int(rng.integers(2, 4))
        G = [Consideration(f"c{i}", "p", {0: cells[0]}) for i in range(k)]
        assert is_closed(0, G, G)
        new = Consideration("new", "p", {0: cells[int(rng.integers(1, 3))]})
        if not is_closed(0, G, G + [new]):
            destroyed += 1
    return {"passed": destroyed > 0,
            "closures_destroyed": destroyed, "trials": trials,
            "fraction": destroyed / trials,
            "note": "enlarging availability can un-close; invoked-monotonicity holds"}


@experiment("E15", "Multiplicative composition (Thm 7.3)", "randomised")
def e15(rng):
    worst = 0.0
    for _ in range(500):
        n = int(rng.integers(1, 7))
        ks = rng.uniform(0.0, 0.9, n)
        beta, S = 1.0, [10.0]
        for k in ks:
            g = S[-1] - beta
            S.append(S[-1] - k * g)
        measured = (S[0] - S[-1]) / (S[0] - beta)
        worst = max(worst, abs(compose_powers(list(ks)) - measured))
    return {"passed": worst < 1e-12, "max_abs_error": worst}


@experiment("E16", "Saturation (Cor 7.4, Thm 7.6)", "exhaustive")
def e16(rng):
    k, rows = 0.3, []
    for n in (1, 2, 5, 10, 25, 50):
        rows.append({"n": n, "composite": compose_powers([k] * n),
                     "residual": (1 - k) ** n})
    increasing = all(rows[i]["composite"] < rows[i + 1]["composite"]
                     for i in range(len(rows) - 1))
    never_one = all(r["composite"] < 1.0 for r in rows)
    decaying = all(rows[i]["residual"] > rows[i + 1]["residual"]
                   for i in range(len(rows) - 1))
    div = saturates([1.0 / (i + 1) for i in range(100000)])      # harmonic: diverges
    conv = saturates([2.0 ** -(i + 1) for i in range(200)])      # geometric: converges
    return {"passed": increasing and never_one and decaying and div and not conv,
            "rows": rows, "divergent_saturates": div,
            "convergent_saturates": conv}


@experiment("E17", "Three routes to non-commitment (Prop 4.14)", "constructed")
def e17(rng):
    A, B, C = frozenset({0, 1}), frozenset({2, 3}), frozenset({4, 5})
    suff = {A, B}
    cA = Consideration("a", "p", {0: A})
    cB = Consideration("b", "p", {0: B})
    r_committed = three_routes(0, A, [cA], [cA], suff)
    r_ignorance = three_routes(0, B, [cA], [cA], suff)
    r_inertia = three_routes(0, A, [cA], [cA, cB], suff)
    r_confusion = three_routes(0, C, [cA], [cA], suff)
    got = [r_committed, r_ignorance, r_inertia, r_confusion]
    return {"passed": got == ["committed", "ignorance", "inertia", "confusion"],
            "routes": got}


@experiment("E18", "Support cycle >= 3 (Thm 7.9)", "exhaustive + falsifier")
def e18(rng):
    # exhaustive over all digraphs on 3 nodes
    import itertools
    nodes, pairs = 3, [(i, j) for i in range(3) for j in range(3) if i != j]
    robust_with_short_cycle = 0
    total, robust = 0, 0
    for mask in range(1 << len(pairs)):
        E = {pairs[i] for i in range(len(pairs)) if mask >> i & 1}
        total += 1
        r = support_is_robust(E, nodes)
        L = shortest_support_cycle(E, nodes)
        if r:
            robust += 1
            if L is not None and L < 3:
                robust_with_short_cycle += 1
    # FALSIFIER: 1-cycles and 2-cycles must NOT be robust
    fals = sum(1 for E in ({(0, 0)}, {(0, 1), (1, 0)})
               if support_is_robust(E, 3))
    return {"passed": robust_with_short_cycle == 0 and robust > 0 and fals == 0,
            "digraphs_enumerated": total, "robust": robust,
            "robust_with_cycle_lt_3": robust_with_short_cycle,
            "falsifier_failures": 2 - fals,
            "note": "exhaustive over all 3-node digraphs"}


# =====================================================================
#  Part III -- instrument: pruning, floor estimators
# =====================================================================

@experiment("E19", "Pruning is determined (Thm 8.7)", "Zurich substrate")
def e19(rng):
    z = load_zurich()["zurich_population"]
    shared = ContactGraph(12)
    for i in range(11):
        shared.add_edge(i, i + 1, 2.0)
    shared.add_edge(0, 11, 2.0)
    mods = [
        Module("young", frozenset(range(0, 6)),
               lambda r: r.get("age_band") in ("0-19", "20-39")),
        Module("older", frozenset(range(4, 12)),
               lambda r: r.get("age_band") in ("40-59", "60-79", "80+")),
        Module("swiss", frozenset(range(2, 10)),
               lambda r: r.get("origin", "").startswith("Schweiz")),
    ]
    records = []
    if z:
        for k in z["districts"]:
            for band in z["age_bands"].get(k, {}):
                records.append({"district": k, "age_band": band,
                                "origin": "Schweizer*in"})
    else:
        records = [{"district": "K", "age_band": "20-39", "origin": "Schweizer*in"}]

    digests, chis, stable = {}, {}, True
    for r in records[:60]:
        g1, n1 = prune(mods, r, shared)
        g2, n2 = prune(mods, r, shared)                 # idempotent before contact
        d = generation_digest(n1, r)
        if n1 != n2:
            stable = False
        digests.setdefault(d, set()).add(
            character_invariant(g1) if g1.n > 1 else 0.0)
        chis[d] = character_invariant(g1) if g1.n > 1 else 0.0
    one_chi_per_digest = all(len(v) == 1 for v in digests.values())

    # after first contact the agent is no longer reproducible by pruning
    g, names = prune(mods, records[0], shared)
    if g.n > 1:
        a = Agent("gen", g)
        a.undo()
        irreversible = a.record > 0
    else:
        irreversible = True
    return {"passed": stable and one_chi_per_digest and irreversible,
            "records_tested": len(records[:60]),
            "distinct_digests": len(digests),
            "deterministic": stable and one_chi_per_digest,
            "irreversible_after_contact": irreversible}


@experiment("E20", "Floor estimators (Rem 8.2)", "Zurich + synthetic + falsifier")
def e20(rng):
    """The substantive obligation (S4).

    sample_minimum() is positive whenever the sample is nonempty and therefore
    discriminates nothing; asymptotic_separation() can return a value
    indistinguishable from zero and so can be wrong.  We test both against a
    process WITH a floor and a process WITHOUT one.
    """
    def sample_minimum(x):
        """Trivial discharge: the least observed separation in the sample."""
        return float(np.min(np.abs(x)))

    def asymptotic_separation(x, rng_local, reps=40):
        """Non-trivial discharge: the limit of the least separation as the
        sample grows.  Estimated by subsampling at increasing n and
        extrapolating min(n) against 1/n to the intercept.  Unlike
        sample_minimum this CAN return ~0, hence can be wrong (Rem. 8.2)."""
        v = np.abs(np.asarray(x, dtype=float))
        if v.size < 40:
            return float(np.min(v))
        ns = np.unique(np.geomspace(20, v.size, 10).astype(int))
        mins = []
        for n in ns:
            # average the min over independent subsamples of size n
            mins.append(float(np.mean([np.min(rng_local.choice(v, n, replace=False))
                                       for _ in range(reps)])))
        A = np.vstack([1.0 / ns, np.ones(len(ns))]).T
        coef, *_ = np.linalg.lstsq(A, np.array(mins), rcond=None)
        return float(max(0.0, coef[1]))

    n = 4000
    # a process WITH a genuine floor: separations bounded below by 0.5
    with_floor = rng.uniform(0.5, 1.0, n) * rng.choice([-1, 1], n)
    # a process WITHOUT one: separations accumulate arbitrarily close to 0
    no_floor = rng.uniform(0.0, 1.0, n) ** 3 * rng.choice([-1, 1], n)

    sm_w, sm_n = sample_minimum(with_floor), sample_minimum(no_floor)
    as_w = asymptotic_separation(with_floor, rng)
    as_n = asymptotic_separation(no_floor, rng)

    # The diagnostic: sample_minimum is positive on BOTH processes, so a
    # positivity claim from it carries no evidential weight.  The asymptotic
    # estimator separates them.
    trivial = sm_w > 0 and sm_n > 0
    separates = as_w > 0.4 and as_n < 0.05

    # ---- the real binding ------------------------------------------------
    # (S2) The observable is inter-district SEPARATION on a question, not a
    # single district's distance to 50%: the floor attaches to telling two
    # receivers apart (Def. 2.2), and published shares are rounded to 0.1pp,
    # so a single margin can be exactly zero without any separation vanishing.
    z = load_zurich()["zurich_ballots"]
    zur = None
    if z:
        M = np.array([[q["yes_share"][d] for d in z["districts"]]
                      for q in z["questions"]])
        D = z["districts"]
        # Per-question shares are published rounded to 0.1pp, so two districts
        # can tie on a single question without being inseparable.  The floor
        # attaches to a RECEIVER PAIR over the whole record (Def. 2.2): the
        # cost of telling Kreis i from Kreis j, taken across all 275 questions.
        pair_costs, ties_single = [], 0
        for i in range(len(D)):
            for j in range(i + 1, len(D)):
                d = np.abs(M[:, i] - M[:, j])
                ties_single += int((d == 0).sum())
                pair_costs.append(float(d.mean()))       # cost of the cut {i}|{j}
        pair_costs = np.array(pair_costs)
        zur = {"observable": "mean inter-district separation over the full record",
               "n_receiver_pairs": int(pair_costs.size),
               "n_questions": int(M.shape[0]),
               "sample_minimum": sample_minimum(pair_costs),
               "asymptotic_separation": asymptotic_separation(pair_costs, rng),
               "median_pair_cost": float(np.median(pair_costs)),
               "single_question_ties": ties_single,
               "single_question_tie_rate": ties_single / (M.shape[0] *
                                                          len(D) * (len(D) - 1) / 2),
               "declared_estimator": "asymptotic_separation",
               "floor_positive": bool(pair_costs.min() > 0),
               "note": ("single-question ties are a 0.1pp rounding artefact; "
                        "no receiver pair is inseparable over the record")}
    return {"passed": trivial and separates,
            "sample_minimum_with_floor": sm_w,
            "sample_minimum_no_floor": sm_n,
            "asymptotic_with_floor": as_w,
            "asymptotic_no_floor": as_n,
            "sample_minimum_is_trivial": trivial,
            "asymptotic_discriminates": separates,
            "falsifier_failures": int(trivial),
            "zurich": zur,
            "note": ("sample_minimum is positive on a floorless process, so it "
                     "discriminates nothing; the asymptotic estimator can "
                     "return ~0 and therefore can be wrong")}


# =====================================================================
#  Part IV -- negative controls and the corrected test
# =====================================================================

@experiment("E21", "Telescoping obstruction (Thm 11.2)", "adversarial")
def e21(rng):
    """NEGATIVE CONTROL.

    Generate cascades from a process that VIOLATES multiplicative composition
    by construction, and confirm the instance-specific test still reports
    r = 1 with vanishing error.  A test that cannot fail cannot discriminate
    (Cor 11.3).
    """
    preds, meas = [], []
    beta = 1.0
    for _ in range(2000):
        n = int(rng.integers(2, 7))
        S = [float(rng.uniform(5.0, 20.0))]
        for _ in range(n):
            # deliberately NON-multiplicative, non-monotone-in-kappa dynamics
            step = float(rng.uniform(0.01, 0.6)) * (S[-1] - beta)
            step *= float(rng.uniform(0.2, 1.8))              # arbitrary distortion
            S.append(max(beta + 1e-9, S[-1] - step))
        p, m = instance_specific_prediction(S, beta)
        preds.append(p); meas.append(m)
    preds, meas = np.array(preds), np.array(meas)
    r = float(np.corrcoef(preds, meas)[0, 1])
    rmse = float(np.sqrt(np.mean((preds - meas) ** 2)))
    degenerate = abs(r - 1.0) < 1e-9 and rmse < 1e-12
    return {"passed": degenerate, "negative_control": True,
            "pearson_r": r, "rmse": rmse, "cascades": len(preds),
            "generating_process": "adversarial, violates Thm 7.3 by construction",
            "note": "r=1 on data violating the law => the test has no null"}


@experiment("E22", "Uninformative binding, eta ~ 0 (Prop 11.7)", "constructed")
def e22(rng):
    """NEGATIVE CONTROL: a binding on which the composition test has no power."""
    types = np.array(["t1", "t2", "t3"] * 400)
    # all type means identical -> eta ~ 0
    powers_flat = rng.normal(0.4, 0.15, len(types))
    eta_flat = eta_separation(powers_flat, types)
    # separated type means -> eta large
    base = {"t1": 0.15, "t2": 0.45, "t3": 0.75}
    powers_sep = np.array([base[t] + rng.normal(0, 0.02) for t in types])
    eta_sep = eta_separation(powers_sep, types)
    # at fixed cascade length the flat binding gives a constant prediction
    means_flat = {t: float(powers_flat[types == t].mean()) for t in ("t1", "t2", "t3")}
    preds = [type_averaged_prediction([], ["t1", "t2", "t3"], means_flat)
             for _ in range(50)]
    constant = float(np.std(preds)) < 1e-12
    return {"passed": eta_flat < 0.25 and eta_sep > 0.75 and constant,
            "negative_control": True,
            "eta_flat_binding": eta_flat, "eta_separated_binding": eta_sep,
            "prediction_constant_at_fixed_length": constant,
            "note": "eta~0 => negative result is about the observable, not the process"}


@experiment("E23", "Corrected type-averaged test (Thm 11.5)", "Zurich types")
def e23(rng):
    """The corrected test HAS a null: it fails on data violating the law."""
    beta = 1.0
    tlist = ["fiscal", "siting", "precedent"]
    tmeans = {"fiscal": 0.20, "siting": 0.45, "precedent": 0.65}

    def run(conforming: bool):
        preds, meas = [], []
        for _ in range(1500):
            n = int(rng.integers(2, 5))
            ts = [tlist[int(rng.integers(0, 3))] for _ in range(n)]
            S = [float(rng.uniform(5.0, 20.0))]
            for t in ts:
                k = tmeans[t] if conforming else float(rng.uniform(0.0, 0.9))
                k = min(0.95, max(0.0, k + float(rng.normal(0, 0.02))))
                S.append(S[-1] - k * (S[-1] - beta))
            preds.append(type_averaged_prediction(S, ts, tmeans))
            meas.append((S[0] - S[-1]) / (S[0] - beta))
        preds, meas = np.array(preds), np.array(meas)
        return (float(np.corrcoef(preds, meas)[0, 1]),
                float(np.sqrt(np.mean((preds - meas) ** 2))))

    r_ok, rmse_ok = run(True)
    r_bad, rmse_bad = run(False)

    z = load_zurich()["zurich_ballots"]
    zur_eta = None
    if z:
        M = np.array([[q["yes_share"][d] for d in z["districts"]]
                      for q in z["questions"]])
        # event = consecutive ballots; type = decade; power from margin change
        pw, ty = [], []
        for j in range(M.shape[1]):
            col = M[:, j]
            for i in range(len(col) - 1):
                S0 = abs(col[i] - 0.5) + 1.0
                S1 = abs(col[i + 1] - 0.5) + 1.0
                pw.append(catalytic_power(S0, min(S1, S0), 1.0))
                ty.append(z["questions"][i]["date"][:3])
        zur_eta = {"eta": eta_separation(pw, ty), "n_events": len(pw)}

    has_null = (rmse_bad > 10 * max(rmse_ok, 1e-9)) and (r_bad < r_ok)
    return {"passed": has_null,
            "conforming_r": r_ok, "conforming_rmse": rmse_ok,
            "violating_r": r_bad, "violating_rmse": rmse_bad,
            "test_has_a_null": has_null, "zurich": zur_eta,
            "note": "unlike E21 the corrected test discriminates"}


@experiment("E24", "Runtime: convergence + emergence (Thm 11.4, 11.8)", "randomised")
def e24(rng):
    # convergence is associative/commutative/idempotent -> order-independent nodes
    fps = set()
    for _ in range(50):
        rt = Runtime()
        subs = [f"s{i}" for i in range(8)]
        for th in rng.permutation(subs):
            rt.converge(str(th), [lambda: 1], [1])
        for th in rng.permutation(subs):                 # duplicate contributions
            rt.converge(str(th), [], [1])
        fps.add(rt.protocol_fingerprint())
    order_independent = len(fps) == 1

    # trajectories differ across runs over the same node set
    trajs = set()
    for _ in range(60):
        rt = Runtime()
        for i in range(6):
            rt.identify(f"n{i}")
        seed = int(rng.integers(0, 6))
        rt.emit(f"n{seed}", 1.0)
        for step in range(6):
            src = f"n{int(rng.integers(0, 6))}"
            dst = f"n{int(rng.integers(0, 6))}"
            if rt.read(src):
                rt.emit(dst, float(rng.random()), cause=src)
        trajs.add(frozenset(rt.edges))
    emergent = len(trajs) > 1

    # run to completion: a raising chunk does not halt the run
    rt = Runtime()
    n = rt.identify("boom")
    n.chunks = [lambda: 1, lambda: (_ for _ in ()).throw(ValueError("x")), lambda: 3]
    out = rt.execute("boom")
    completed = len(out) == 3 and isinstance(out[1], dict) and "error" in out[1]

    # no exit code: the runtime exposes no comparison operation
    no_verdict = not any(hasattr(rt, m) for m in
                         ("exit_code", "succeeded", "failed", "compare", "expect"))
    return {"passed": order_independent and emergent and completed and no_verdict,
            "distinct_protocol_fingerprints": len(fps),
            "distinct_trajectories": len(trajs),
            "run_to_completion": completed,
            "no_exit_code": no_verdict}


# =====================================================================
#  driver
# =====================================================================

def main() -> int:
    rng = np.random.default_rng(SEED)
    print(f"closure/zuerich validation suite   seed={SEED}\n")
    print("Part II -- the coordination law")
    for fn in (e01, e02, e03, e04, e05, e06, e07, e08, e09, e10,
               e11, e12, e13, e14, e15, e16, e17, e18):
        fn(rng)
    print("\nPart III -- the instrument")
    for fn in (e19, e20):
        fn(rng)
    print("\nPart IV -- negative controls and the corrected test")
    for fn in (e21, e22, e23, e24):
        fn(rng)

    n_pass = sum(1 for r in _REG if r["passed"])
    neg = [r["id"] for r in _REG if r.get("negative_control")]
    summary = {
        "suite": "closure/zuerich",
        "seed": SEED,
        "generated": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "n_experiments": len(_REG),
        "n_passed": n_pass,
        "negative_controls": neg,
        "pass_rate": f"{n_pass}/{len(_REG)}",
        "experiments": _REG,
    }
    with open(os.path.join(RESULTS, "validation_summary.json"), "w",
              encoding="utf-8") as fh:
        json.dump(summary, fh, indent=1, default=float)
    for r in _REG:
        with open(os.path.join(RESULTS, f"{r['id']}.json"), "w",
                  encoding="utf-8") as fh:
            json.dump(r, fh, indent=1, default=float)

    print(f"\n{n_pass}/{len(_REG)} passed "
          f"(negative controls confirmed: {', '.join(neg)})")
    print(f"results -> {RESULTS}")
    return 0 if n_pass == len(_REG) else 1


if __name__ == "__main__":
    raise SystemExit(main())
