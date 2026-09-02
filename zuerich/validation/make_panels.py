"""
Generate the eight publication panels.

Each panel is four charts in a row on a white background, at least one of
which is three-dimensional.  Every quantity plotted is COMPUTED -- from the
prototype runtime or from the Zurich record -- and no panel contains a
schematic, a table, or a text-only chart.

Figures are written to docs/zuerich-common-closure/figures/panel_N.png
and the underlying series to validation/results/panel_data.json.
"""

from __future__ import annotations

import itertools
import json
import math
import os
import sys

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib import cm
from mpl_toolkits.mplot3d import Axes3D  # noqa: F401

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..", "prototype")))

from runtime import (  # noqa: E402
    Agent, Consideration, ContactGraph, Phase,
    catalytic_power, character_invariant, compose_powers, coordination_cost,
    critical_coupling, cut_weight, eta_separation, instance_specific_prediction,
    is_closed, kuramoto_step, min_cut_weight, order_parameter, reach,
    separation_cost, shortest_support_cycle, support_is_robust,
    type_averaged_prediction, water_fill,
)

DATA = os.path.abspath(os.path.join(HERE, "..", "data"))
FIGS = os.path.abspath(os.path.join(HERE, "..", "docs",
                                    "zuerich-common-closure", "figures"))
RESULTS = os.path.join(HERE, "results")
os.makedirs(FIGS, exist_ok=True)

SEED = 20260902
SERIES: dict = {}

# ---- house style -----------------------------------------------------
plt.rcParams.update({
    "figure.facecolor": "white",
    "axes.facecolor": "white",
    "savefig.facecolor": "white",
    "font.size": 8.5,
    "axes.titlesize": 9,
    "axes.labelsize": 8.5,
    "xtick.labelsize": 7.5,
    "ytick.labelsize": 7.5,
    "legend.fontsize": 7.5,
    "axes.grid": True,
    "grid.alpha": 0.25,
    "grid.linewidth": 0.5,
    "axes.spines.top": False,
    "axes.spines.right": False,
    "lines.linewidth": 1.6,
    "figure.dpi": 200,
})

C0, C1, C2, C3 = "#1f4e79", "#c0392b", "#2e8b57", "#e08214"
CMAP = "viridis"


def new_panel(threed=(3,)):
    fig = plt.figure(figsize=(15.0, 3.5))
    axes = []
    for i in range(4):
        if i in threed:
            axes.append(fig.add_subplot(1, 4, i + 1, projection="3d"))
        else:
            axes.append(fig.add_subplot(1, 4, i + 1))
    return fig, axes


def tag(ax, letter, threed=False):
    if threed:
        ax.text2D(-0.06, 1.04, letter, transform=ax.transAxes,
                  fontsize=11, fontweight="bold", va="top")
    else:
        ax.text(-0.16, 1.06, letter, transform=ax.transAxes,
                fontsize=11, fontweight="bold", va="top")


def save(fig, n):
    fig.tight_layout(w_pad=2.0)
    p = os.path.join(FIGS, f"panel_{n}.png")
    fig.savefig(p, bbox_inches="tight")
    plt.close(fig)
    print(f"  panel_{n}.png")


def style3d(ax):
    ax.xaxis.pane.set_facecolor("white")
    ax.yaxis.pane.set_facecolor("white")
    ax.zaxis.pane.set_facecolor("white")
    ax.xaxis.pane.set_edgecolor("0.85")
    ax.yaxis.pane.set_edgecolor("0.85")
    ax.zaxis.pane.set_edgecolor("0.85")
    ax.grid(True, alpha=0.2)


def rand_graph(rng, n, lo=1.0, hi=9.0):
    g = ContactGraph(n)
    order = list(rng.permutation(n))
    for i in range(1, n):
        j = int(rng.integers(0, i))
        g.add_edge(int(order[i]), int(order[j]), float(rng.uniform(lo, hi)))
    for _ in range(int(rng.integers(0, n))):
        u, v = int(rng.integers(0, n)), int(rng.integers(0, n))
        if u != v:
            g.add_edge(u, v, float(rng.uniform(lo, hi)))
    return g


def zurich():
    b = os.path.join(DATA, "zurich_ballots.json")
    p = os.path.join(DATA, "zurich_population.json")
    return (json.load(open(b, encoding="utf-8")) if os.path.exists(b) else None,
            json.load(open(p, encoding="utf-8")) if os.path.exists(p) else None)


# =====================================================================
#  PANEL 1 -- The floor
# =====================================================================
def panel1():
    rng = np.random.default_rng(SEED)
    fig, ax = new_panel(threed=(3,))

    # (A) min cut vs floor, 600 random graphs -- all on/above identity
    beta, res = [], []
    for _ in range(600):
        g = rand_graph(rng, int(rng.integers(3, 9)))
        beta.append(g.beta); res.append(separation_cost(g))
    beta, res = np.array(beta), np.array(res)
    ax[0].scatter(beta, res, s=7, c=C0, alpha=0.45, edgecolors="none")
    lim = [0, max(res.max(), beta.max()) * 1.05]
    ax[0].plot(lim, lim, color=C1, ls="--", lw=1.2)
    ax[0].set_xlabel(r"floor $\beta$"); ax[0].set_ylabel(r"Res($\mathcal{G}$)")
    ax[0].set_title("Res never falls below the floor")
    ax[0].set_xlim(0, lim[1]); ax[0].set_ylim(0, lim[1])
    tag(ax[0], "A")

    # (B) falsifier: admit a zero-weight edge -> bound breaks
    ratios_ok, ratios_bad = [], []
    for _ in range(400):
        g = rand_graph(rng, int(rng.integers(3, 9)))
        ratios_ok.append(separation_cost(g) / g.beta)
        b0 = g.beta
        e = list(g.weights)[int(rng.integers(0, len(g.weights)))]
        g.weights[e] = 0.0
        ratios_bad.append(separation_cost(g) / b0)
    bins = np.linspace(0, 4, 45)
    ax[1].hist(ratios_ok, bins=bins, color=C0, alpha=0.85, label="axiom holds")
    ax[1].hist(ratios_bad, bins=bins, color=C1, alpha=0.6, label="zero-weight edge")
    ax[1].axvline(1.0, color="0.25", ls="--", lw=1.2)
    ax[1].set_xlabel(r"Res($\mathcal{G}$) / $\beta$"); ax[1].set_ylabel("graphs")
    ax[1].set_title("Falsifier drives the ratio below 1")
    ax[1].legend(frameon=False, loc="upper right")
    tag(ax[1], "B")

    # (C) regional individuation: bridge weight sweep on the two-triangle graph
    ws = np.linspace(0.2, 34, 70)
    mc, best_single = [], []
    for w in ws:
        g = ContactGraph(6)
        for u, v in [(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5)]:
            g.add_edge(u, v, 10.0)
        g.add_edge(2, 3, float(w))
        mc.append(separation_cost(g))
        best_single.append(min(cut_weight(g, [v]) for v in g.vertices))
    mc, best_single = np.array(mc), np.array(best_single)
    ax[2].plot(ws, mc, color=C0, label="minimum cut")
    ax[2].plot(ws, best_single, color=C1, ls="--", label="best singleton")
    reg = mc < best_single - 1e-9
    ax[2].fill_between(ws, mc, best_single, where=reg, color=C2, alpha=0.18,
                       label="minimum is regional")
    xc = ws[reg].max() if reg.any() else None
    if xc is not None:
        ax[2].axvline(xc, color="0.35", ls=":", lw=1.1)
    ax[2].set_xlabel("bridge weight"); ax[2].set_ylabel("cut cost")
    ax[2].set_ylabel("cut cost")
    ax[2].set_title("Regional until the bridge exceeds a vertex")
    ax[2].legend(frameon=False, loc="upper left")
    tag(ax[2], "C")

    # (D) 3D: Res surface over (n, density)
    ns = np.arange(4, 11)
    dens = np.linspace(0.25, 1.0, 9)
    Z = np.zeros((len(ns), len(dens)))
    for i, n in enumerate(ns):
        for j, d in enumerate(dens):
            vals = []
            for _ in range(14):
                g = ContactGraph(int(n))
                order = list(rng.permutation(int(n)))
                for k in range(1, int(n)):
                    g.add_edge(int(order[k]), int(order[int(rng.integers(0, k))]),
                               float(rng.uniform(1, 9)))
                for u, v in itertools.combinations(range(int(n)), 2):
                    if rng.random() < d:
                        g.add_edge(u, v, float(rng.uniform(1, 9)))
                vals.append(separation_cost(g))
            Z[i, j] = np.mean(vals)
    X, Y = np.meshgrid(dens, ns)
    ax[3].plot_surface(X, Y, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=40, ccount=40)
    ax[3].set_xlabel("density"); ax[3].set_ylabel("positions $|V|$")
    ax[3].set_zlabel(r"Res($\mathcal{G}$)")
    ax[3].set_title("Separation cost surface")
    ax[3].view_init(elev=24, azim=-58)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel1"] = {"beta": beta.tolist(), "res": res.tolist(),
                        "bridge_sweep_w": ws.tolist(),
                        "bridge_mincut": mc.tolist(),
                        "bridge_best_singleton": best_single.tolist(),
                        "surface_ns": ns.tolist(), "surface_density": dens.tolist(),
                        "surface_Z": Z.tolist()}
    save(fig, 1)


# =====================================================================
#  PANEL 2 -- Closure and blindness
# =====================================================================
def panel2():
    rng = np.random.default_rng(SEED + 1)
    fig, ax = new_panel(threed=(3,))
    cells = [frozenset({0, 1}), frozenset({2, 3}), frozenset({4, 5})]

    # (A) threshold met while closure fails, across a confidence sweep
    th = np.linspace(0.0, 1.0, 41)
    cA = Consideration("a", "p1", {0: cells[0]})
    cB = Consideration("b", "p2", {0: cells[1]})
    thr_ok = (1.0 >= th).astype(float)
    clo = np.full_like(th, float(is_closed(0, [cA], [cA, cB])))
    clo_restored = np.full_like(th, float(is_closed(0, [cA], [cA, cA])))
    ax[0].plot(th, thr_ok, color=C1, label="threshold met")
    ax[0].plot(th, clo, color=C0, lw=2.4, label="closed (distinct region)")
    ax[0].plot(th, clo_restored, color=C2, ls="--", lw=2.0,
               label="closed (same region)")
    ax[0].set_ylim(-0.08, 1.12)
    ax[0].set_xlabel(r"threshold $\theta$"); ax[0].set_ylabel("criterion satisfied")
    ax[0].set_title("Closure is strictly stronger")
    ax[0].legend(frameon=False, loc="center right")
    tag(ax[0], "A")

    # (B) blindness asymmetry: label vs resolution perturbation
    sizes = [50, 100, 200, 400, 800]
    lab, res_ = [], []
    for N in sizes:
        cl = cr = 0
        for _ in range(N):
            k = int(rng.integers(2, 5))
            G = [Consideration(f"c{i}", f"s{i}",
                               {0: cells[int(rng.integers(0, 3))]}) for i in range(k)]
            inv = G[:max(1, k - 1)]
            base = is_closed(0, inv, G)
            Gp = [Consideration(c.name, f"X{int(rng.integers(0, 999))}", c.resolution)
                  for c in G]
            if is_closed(0, Gp[:len(inv)], Gp) != base:
                cl += 1
            Gr = [Consideration(c.name, c.provenance,
                                {0: cells[int(rng.integers(0, 3))]}) for c in G]
            if is_closed(0, Gr[:len(inv)], Gr) != base:
                cr += 1
        lab.append(100 * cl / N); res_.append(100 * cr / N)
    x = np.arange(len(sizes))
    b1 = ax[1].bar(x - 0.19, lab, 0.38, color=C0, label="permute provenance")
    ax[1].bar(x + 0.19, res_, 0.38, color=C1, label="perturb resolution")
    for xi, v in zip(x - 0.19, lab):
        ax[1].plot([xi - 0.19, xi + 0.19], [0, 0], color=C0, lw=3.0,
                   solid_capstyle="butt")
        ax[1].annotate("0", (xi, 1.4), ha="center", fontsize=7.5, color=C0)
    ax[1].set_xticks(x); ax[1].set_xticklabels(sizes)
    ax[1].set_ylim(0, max(res_) * 1.30)
    ax[1].set_xlabel("trials"); ax[1].set_ylabel("verdict changed (%)")
    ax[1].set_title("Blind to label, sensitive to resolution")
    ax[1].legend(frameon=False, loc="upper left")
    tag(ax[1], "B")

    # (C) availability non-monotonicity vs number of distinct new regions
    # A newcomer un-closes only if it reaches a region NOT already reached, so
    # the risk falls as the invoked reach already covers more of the pool.
    pool_sz = 8
    pool = [frozenset({10 + 2 * i, 11 + 2 * i}) for i in range(pool_sz)]
    ks = np.arange(1, pool_sz + 1)
    fracs = []
    for kk in ks:
        d = 0
        for _ in range(800):
            idx = rng.choice(pool_sz, int(kk), replace=False)
            G = [Consideration(f"c{i}", "p", {0: pool[i]}) for i in idx]
            new = Consideration("n", "p", {0: pool[int(rng.integers(0, pool_sz))]})
            if not is_closed(0, G, G + [new]):
                d += 1
        fracs.append(100 * d / 800)
    ax[2].plot(ks, fracs, "o-", color=C0, ms=4)
    ax[2].fill_between(ks, 0, fracs, color=C0, alpha=0.12)
    ax[2].plot(ks, 100 * (1 - ks / pool_sz), color=C1, ls="--", lw=1.2,
               label=r"$1-k/|\Sigma_\tau|$")
    ax[2].set_xlabel("regions already reached")
    ax[2].set_ylabel("closures destroyed (%)")
    ax[2].set_title("Un-closing risk falls as reach grows")
    ax[2].set_ylim(0, 105)
    ax[2].legend(frameon=False, loc="upper right")
    tag(ax[2], "C")

    # (D) 3D: closure probability over (invoked, available)
    inv_n = np.arange(1, 9)
    av_n = np.arange(1, 9)
    Z = np.zeros((len(inv_n), len(av_n)))
    pool = [frozenset({2 * i, 2 * i + 1}) for i in range(6)]
    for i, a in enumerate(inv_n):
        for j, b in enumerate(av_n):
            tot = int(a) + int(b)
            hits = 0
            for _ in range(160):
                G = [Consideration(f"c{t}", "p",
                                   {0: pool[int(rng.integers(0, len(pool)))]})
                     for t in range(tot)]
                if is_closed(0, G[:int(a)], G):
                    hits += 1
            Z[i, j] = hits / 160.0
    X, Y = np.meshgrid(av_n, inv_n)
    ax[3].plot_surface(X, Y, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=40, ccount=40)
    ax[3].set_xlabel("uninvoked available"); ax[3].set_ylabel("invoked")
    ax[3].set_zlabel("P(closed)")
    ax[3].set_title("Closure probability")
    ax[3].view_init(elev=26, azim=-52)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel2"] = {"blind_sizes": sizes, "pct_label": lab, "pct_resolution": res_,
                        "unclose_k": ks.tolist(), "unclose_pct": fracs,
                        "closure_surface": Z.tolist()}
    save(fig, 2)


# =====================================================================
#  PANEL 3 -- Truth-blindness and the three routes
# =====================================================================
def panel3():
    rng = np.random.default_rng(SEED + 2)
    fig, ax = new_panel(threed=(3,))
    cells = [frozenset({0, 1}), frozenset({2, 3})]

    # (A) commitment rate vs fraction of false considerations
    fr = np.linspace(0, 1, 21)
    rates = []
    for f in fr:
        c = 0
        for _ in range(500):
            G = [Consideration(f"c{i}", "p", {0: cells[0]},
                               veridical=(rng.random() > f)) for i in range(3)]
            if is_closed(0, G, G) and cells[0] in reach(0, G):
                c += 1
        rates.append(100 * c / 500)
    ax[0].plot(fr * 100, rates, "o-", color=C0, ms=3.5)
    ax[0].axhline(100, color=C1, ls="--", lw=1.2)
    ax[0].set_ylim(0, 108)
    ax[0].set_xlabel("false considerations (%)"); ax[0].set_ylabel("commitment rate (%)")
    ax[0].set_title("Commitment is flat in falsity")
    tag(ax[0], "A")

    # (B) three routes: incidence as the available set grows
    ns = np.arange(1, 11)
    counts = {"committed": [], "ignorance": [], "inertia": []}
    pool = [frozenset({2 * i, 2 * i + 1}) for i in range(5)]
    for n in ns:
        c = {"committed": 0, "ignorance": 0, "inertia": 0}
        for _ in range(500):
            G = [Consideration(f"c{i}", "p",
                               {0: pool[int(rng.integers(0, len(pool)))]})
                 for i in range(int(n))]
            inv = G[:1]
            target = pool[0]
            if target not in reach(0, inv):
                c["ignorance"] += 1
            elif not is_closed(0, inv, G):
                c["inertia"] += 1
            else:
                c["committed"] += 1
        for k in counts:
            counts[k].append(100 * c[k] / 500)
    ax[1].stackplot(ns, counts["committed"], counts["ignorance"], counts["inertia"],
                    colors=[C2, C3, C1], alpha=0.85,
                    labels=["committed", "ignorance (i)", "inertia (ii)"])
    ax[1].set_xlim(1, 10); ax[1].set_ylim(0, 100)
    ax[1].set_xlabel("available considerations"); ax[1].set_ylabel("share (%)")
    ax[1].set_title("Routes to non-commitment")
    ax[1].legend(frameon=False, loc="lower right")
    tag(ax[1], "B")

    # (C) reach ceiling: reach vs agents for several pool sizes
    for p, col in zip((2, 4, 6, 8), (C0, C2, C3, C1)):
        pool_p = [frozenset({3 * i, 3 * i + 1}) for i in range(p)]
        y = []
        for m in range(1, 13):
            G = [Consideration(f"c{i}", "p", {0: pool_p[i % p]}) for i in range(m)]
            y.append(len(reach(0, G)))
        ax[2].plot(range(1, 13), y, "o-", ms=3, color=col, label=f"pool {p}")
    ax[2].set_xlabel("agents"); ax[2].set_ylabel("reach")
    ax[2].set_title("Reach saturates at the pool size")
    ax[2].legend(frameon=False, loc="lower right", ncol=2)
    tag(ax[2], "C")

    # (D) 3D: commitment rate over (falsity, availability)
    fs = np.linspace(0, 1, 11)
    avs = np.arange(1, 9)
    Z = np.zeros((len(fs), len(avs)))
    for i, f in enumerate(fs):
        for j, a in enumerate(avs):
            c = 0
            for _ in range(120):
                G = [Consideration(f"c{t}", "p", {0: cells[0]},
                                   veridical=(rng.random() > f)) for t in range(3)]
                extra = [Consideration(f"x{t}", "p",
                                       {0: cells[int(rng.integers(0, 2))]})
                         for t in range(int(a))]
                if is_closed(0, G, G + extra) and cells[0] in reach(0, G):
                    c += 1
            Z[i, j] = c / 120.0
    X, Y = np.meshgrid(avs, fs)
    ax[3].plot_surface(X, Y, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=40, ccount=40)
    ax[3].set_xlabel("extra available"); ax[3].set_ylabel("falsity fraction")
    ax[3].set_zlabel("P(commit)")
    ax[3].set_title("Falsity flat, availability decisive")
    ax[3].view_init(elev=24, azim=-56)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel3"] = {"falsity": fr.tolist(), "commit_rate": rates,
                        "routes": counts, "surface": Z.tolist()}
    save(fig, 3)


# =====================================================================
#  PANEL 4 -- Attention: water-filling and phase
# =====================================================================
def panel4():
    rng = np.random.default_rng(SEED + 3)
    fig, ax = new_panel(threed=(3,))

    # (A) allocation vs budget for five scenes of differing richness
    ks = np.array([3.2, 2.4, 1.7, 1.1, 0.6])
    budgets = np.linspace(0.05, 6.0, 70)
    A = np.zeros((len(budgets), len(ks))); prices = []
    for i, b in enumerate(budgets):
        inv = [(lambda p, k=k: (k / p) - 1.0 if p > 0 else math.inf) for k in ks]
        a, pr = water_fill(float(b), inv, list(ks))
        A[i] = a; prices.append(pr)
    for j, k in enumerate(ks):
        ax[0].plot(budgets, A[:, j], color=cm.viridis(j / (len(ks) - 1)),
                   label=f"$k_i$={k}")
    ax[0].set_xlabel(r"budget $\alpha$"); ax[0].set_ylabel("allocation $a_i^\\star$")
    ax[0].set_title("Scenes enter as the price falls")
    ax[0].legend(frameon=False, loc="upper left", ncol=2)
    tag(ax[0], "A")

    # (B) price vs budget and vs competing scenes
    ax[1].plot(budgets, prices, color=C0, label="vs budget")
    ax[1].set_xlabel(r"budget $\alpha$"); ax[1].set_ylabel(r"price $p^\star$", color=C0)
    ax[1].tick_params(axis="y", labelcolor=C0)
    ax2 = ax[1].twiny()
    ns = np.arange(1, 13)
    pr_n = []
    for n in ns:
        kk = np.linspace(3.0, 1.0, int(n))
        inv = [(lambda p, k=k: (k / p) - 1.0 if p > 0 else math.inf) for k in kk]
        _, pr = water_fill(1.5, inv, list(kk))
        pr_n.append(pr)
    ax2.plot(ns, pr_n, color=C1, ls="--")
    ax2.set_xlabel("competing scenes", color=C1)
    ax2.tick_params(axis="x", labelcolor=C1)
    ax[1].set_title("Price falls in budget, rises in demand")
    tag(ax[1], "B")

    # (C) marginal gains equalise on the support (KKT residual)
    # active-set size and the fraction of budget each scene receives, as the
    # budget grows: scenes enter one at a time at their entry thresholds
    ks2 = np.array([3.2, 2.4, 1.7, 1.1, 0.6])
    bud = np.linspace(0.02, 6.0, 200)
    active, share_top = [], []
    for b in bud:
        inv = [(lambda p, k=k: (k / p) - 1.0 if p > 0 else math.inf) for k in ks2]
        a, pr = water_fill(float(b), inv, list(ks2))
        active.append(int((a > 1e-9).sum()))
        share_top.append(a[0] / max(a.sum(), 1e-12))
    ax[2].step(bud, active, color=C0, where="post", label="scenes attended")
    ax[2].set_xlabel(r"budget $\alpha$"); ax[2].set_ylabel("active scenes",
                                                            color=C0)
    ax[2].tick_params(axis="y", labelcolor=C0)
    ax[2].set_ylim(0, len(ks2) + 0.4)
    axb = ax[2].twinx()
    axb.plot(bud, share_top, color=C1, ls="--")
    axb.set_ylabel("share to richest scene", color=C1)
    axb.tick_params(axis="y", labelcolor=C1)
    axb.grid(False)
    ax[2].set_title("Scenes enter one at a time")
    tag(ax[2], "C")

    # (D) 3D: allocation surface over (richness, price)
    kk = np.linspace(0.4, 4.0, 40)
    pp = np.linspace(0.15, 4.0, 40)
    K, P = np.meshgrid(kk, pp)
    Z = np.maximum(0.0, (K / P) - 1.0)
    Z[K <= P] = 0.0
    ax[3].plot_surface(K, P, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=45, ccount=45)
    ax[3].set_xlabel("scene richness $k_i$"); ax[3].set_ylabel(r"price $p^\star$")
    ax[3].set_zlabel("$a_i^\\star$")
    ax[3].set_title("Allocation surface with entry threshold")
    ax[3].view_init(elev=26, azim=-56)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel4"] = {"budgets": budgets.tolist(), "alloc": A.tolist(),
                        "prices": prices, "price_vs_scenes": pr_n}
    save(fig, 4)


# =====================================================================
#  PANEL 5 -- Synchronisation on the Zurich record
# =====================================================================
def panel5():
    rng = np.random.default_rng(SEED + 4)
    fig, ax = new_panel(threed=(3,))
    zb, zp = zurich()

    M = np.array([[q["yes_share"][d] for d in zb["districts"]]
                  for q in zb["questions"]])
    D = zb["districts"]
    mu, sd = M.mean(axis=0), M.std(axis=0)
    w = np.array([zp["total"].get(d, 1) for d in D], float); w /= w.sum()
    counts = np.maximum(1, (w * 600).astype(int))
    om = np.concatenate([rng.normal(mu[i], sd[i] + 1e-9, counts[i])
                         for i in range(len(counts))])
    om = (om - om.mean()) / np.std(om)
    Kc = critical_coupling(om)

    # (A) observed district dispositions across 275 questions
    parts = ax[0].violinplot([M[:, i] for i in range(len(D))],
                             showmeans=True, widths=0.8)
    for pc in parts["bodies"]:
        pc.set_facecolor(C0); pc.set_alpha(0.55)
    for key in ("cmeans", "cbars", "cmins", "cmaxes"):
        if key in parts:
            parts[key].set_color("0.3"); parts[key].set_linewidth(1.0)
    ax[0].axhline(0.5, color=C1, ls="--", lw=1.1)
    ax[0].set_xticks(range(1, len(D) + 1))
    ax[0].set_xticklabels([d.replace("Kreis ", "K") for d in D])
    ax[0].set_ylabel("approving share")
    ax[0].set_title(f"Dispositions, {M.shape[0]} municipal questions")
    tag(ax[0], "A")

    # (B) bifurcation: R vs K, with Kc marked
    Ks = np.linspace(0, 4.0 * Kc, 33)
    Rs, Cs = [], []
    for K in Ks:
        ph = rng.uniform(0, 2 * np.pi, len(om))
        for _ in range(3000):
            ph = kuramoto_step(ph, om, K, 0.05)
        R, _ = order_parameter(ph)
        Rs.append(R); Cs.append(coordination_cost(R))
    ax[1].plot(Ks, Rs, "o-", color=C0, ms=3.2)
    ax[1].axvline(Kc, color=C1, ls="--", lw=1.3)
    ax[1].set_xlabel("coupling $K$"); ax[1].set_ylabel("order parameter $R$")
    ax[1].set_title(f"Bifurcation, $K_c$={Kc:.2f} (N={len(om)})")
    ax[1].set_ylim(-0.03, 1.03)
    tag(ax[1], "B")

    # (C) coordination cost collapses as R -> 1
    ax[2].semilogy(Rs, np.maximum(Cs, 1e-6), "o", color=C0, ms=3.5)
    ax[2].set_xlabel("order parameter $R$")
    ax[2].set_ylabel(r"coordination cost $\lambda(1-R)$")
    ax[2].set_title("Cost vanishes only at lock")
    tag(ax[2], "C")

    # (D) 3D: R over (K, heterogeneity)
    # One fixed standardised velocity sample, rescaled per row: every row then
    # differs only in spread, not in sampling noise, so the surface shows the
    # threshold's dependence on heterogeneity rather than draw-to-draw scatter.
    spreads = np.linspace(0.3, 2.5, 16)
    Kgrid = np.linspace(0, 7, 26)
    base = rng.normal(0.0, 1.0, 400)
    base = (base - base.mean()) / base.std()
    ph0 = rng.uniform(0, 2 * np.pi, len(base))
    Z = np.zeros((len(spreads), len(Kgrid)))
    for i, s in enumerate(spreads):
        o = base * s
        for j, K in enumerate(Kgrid):
            ph = ph0.copy()
            for _ in range(2500):
                ph = kuramoto_step(ph, o, K, 0.05)
            Z[i, j] = order_parameter(ph)[0]
    # overlay the predicted threshold Kc = 2 sigma sqrt(2 pi)/pi, on the surface
    kc_line = [critical_coupling(base * s) for s in spreads]
    kc_z = [float(np.interp(kc_line[i], Kgrid, Z[i])) for i in range(len(spreads))]
    X, Y = np.meshgrid(Kgrid, spreads)
    ax[3].plot_surface(X, Y, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=40, ccount=40)
    ax[3].plot(kc_line, spreads, np.array(kc_z) + 0.03,
               color=C1, lw=2.6, zorder=12)
    ax[3].set_xlabel("coupling $K$"); ax[3].set_ylabel(r"spread $\Delta\omega$")
    ax[3].set_zlabel("$R$")
    ax[3].set_title(r"Locked region retreats as $\Delta\omega$ grows")
    ax[3].set_zlim(0, 1)
    ax[3].view_init(elev=38, azim=-70)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel5"] = {"Kc": float(Kc), "K": Ks.tolist(), "R": Rs,
                        "cost": Cs, "districts": D,
                        "surface_spreads": spreads.tolist(),
                        "surface_K": Kgrid.tolist(), "surface_R": Z.tolist()}
    save(fig, 5)


# =====================================================================
#  PANEL 6 -- Composition, saturation, support
# =====================================================================
def panel6():
    rng = np.random.default_rng(SEED + 5)
    fig, ax = new_panel(threed=(3,))

    # (A) repetition saturates; diversification does not
    n = np.arange(0, 31)
    for k, col in zip((0.15, 0.3, 0.5), (C0, C2, C3)):
        ax[0].plot(n, [compose_powers([k] * int(i)) for i in n], color=col,
                   label=f"repeat $\\kappa$={k}")
    div = [compose_powers([0.5 / (i + 1) for i in range(int(m))]) for m in n]
    ax[0].plot(n, div, color=C1, ls="--", label="fresh routes (1/i)")
    ax[0].axhline(1.0, color="0.3", ls=":", lw=1.0)
    ax[0].set_ylim(0, 1.05)
    ax[0].set_xlabel("considerations applied"); ax[0].set_ylabel("composite power")
    ax[0].set_title("Repetition never reaches completion")
    ax[0].legend(frameon=False, loc="lower right")
    tag(ax[0], "A")

    # (B) residual: divergent vs convergent power sequences
    m = np.arange(1, 400)
    seqs = {"constant 0.1": [0.1] * 400, "harmonic 1/i": [1.0 / i for i in range(1, 401)],
            "geometric $2^{-i}$": [2.0 ** -i for i in range(1, 401)],
            "$1/i^2$": [1.0 / i ** 2 for i in range(1, 401)]}
    for (nm, s), col in zip(seqs.items(), (C0, C2, C3, C1)):
        r, out = 1.0, []
        for k in s[:399]:
            r *= (1 - k); out.append(max(r, 1e-18))
        ax[1].semilogy(m, out, color=col, label=nm)
    ax[1].set_xlabel("considerations"); ax[1].set_ylabel("residual fraction")
    ax[1].set_title(r"Saturation iff $\sum\kappa_i=\infty$")
    ax[1].legend(frameon=False, loc="lower left")
    tag(ax[1], "B")

    # (C) robustness vs shortest support cycle, exhaustive over 4-node digraphs
    pairs = [(i, j) for i in range(4) for j in range(4) if i != j]
    by_len = {}
    for mask in range(1 << len(pairs)):
        if bin(mask).count("1") > 6:
            continue
        E = {pairs[i] for i in range(len(pairs)) if mask >> i & 1}
        L = shortest_support_cycle(E, 4)
        key = 0 if L is None else L
        by_len.setdefault(key, [0, 0])
        by_len[key][0] += 1
        if support_is_robust(E, 4):
            by_len[key][1] += 1
    keys = [0, 1, 2, 3, 4]
    frac, counts_n = [], []
    for k in keys:
        tot, rob = by_len.get(k, [0, 0])
        frac.append(100 * rob / tot if tot else 0.0)
        counts_n.append(tot)
    xs = np.arange(len(keys))
    ax[2].bar(xs, frac, color=[C1 if k < 3 else C2 for k in keys], alpha=0.9)
    for xi, f, n_ in zip(xs, frac, counts_n):
        if f < 1:
            ax[2].plot([xi - 0.4, xi + 0.4], [0, 0], color=C1, lw=3.5,
                       solid_capstyle="butt")
            ax[2].annotate("0", (xi, 3.5), ha="center", fontsize=7.5, color=C1)
        ax[2].annotate(f"n={n_}", (xi, 100 if f > 50 else 12), ha="center",
                       fontsize=6.2, color="0.35")
    ax[2].set_xticks(xs)
    ax[2].set_xticklabels(["none", "1", "2", "3", "4"])
    ax[2].set_xlabel("shortest support cycle"); ax[2].set_ylabel("robust (%)")
    ax[2].set_title(r"Only cycles $\geq 3$ ground")
    ax[2].set_ylim(0, 112)
    tag(ax[2], "C")

    # (D) 3D: composite power over (kappa1, kappa2)
    k1 = np.linspace(0, 0.95, 45); k2 = np.linspace(0, 0.95, 45)
    K1, K2 = np.meshgrid(k1, k2)
    Z = 1 - (1 - K1) * (1 - K2)
    ax[3].plot_surface(K1, K2, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=45, ccount=45)
    ax[3].set_xlabel(r"$\kappa_1$"); ax[3].set_ylabel(r"$\kappa_2$")
    ax[3].set_zlabel("composite")
    ax[3].set_title("Multiplicative composition")
    ax[3].set_zlim(0, 1)
    ax[3].view_init(elev=26, azim=-54)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel6"] = {"cycle_lengths": [int(k) for k in keys],
                        "robust_pct": frac, "divergent_curve": div}
    save(fig, 6)


# =====================================================================
#  PANEL 7 -- The two negative results
# =====================================================================
def panel7():
    rng = np.random.default_rng(SEED + 6)
    fig, ax = new_panel(threed=(3,))
    beta = 1.0

    # (A) degenerate test: prediction vs measurement on adversarial data
    P, Mq = [], []
    for _ in range(1500):
        nn = int(rng.integers(2, 7))
        S = [float(rng.uniform(5, 20))]
        for _ in range(nn):
            step = float(rng.uniform(0.01, 0.6)) * (S[-1] - beta)
            step *= float(rng.uniform(0.2, 1.8))
            S.append(max(beta + 1e-9, S[-1] - step))
        p, m = instance_specific_prediction(S, beta)
        P.append(p); Mq.append(m)
    P, Mq = np.array(P), np.array(Mq)
    ax[0].scatter(Mq, P, s=6, c=C1, alpha=0.35, edgecolors="none")
    ax[0].plot([0, 1], [0, 1], color="0.3", ls="--", lw=1.1)
    r = np.corrcoef(P, Mq)[0, 1]
    ax[0].set_xlabel("measured net power"); ax[0].set_ylabel("predicted")
    ax[0].set_title(f"Instance-fitted: $r$={r:.6f} on violating data")
    tag(ax[0], "A")

    # (B) corrected test does have a null
    tl = ["fiscal", "siting", "precedent"]
    tm = {"fiscal": 0.20, "siting": 0.45, "precedent": 0.65}

    def run(conform):
        p_, m_ = [], []
        for _ in range(1200):
            nn = int(rng.integers(2, 5))
            ts = [tl[int(rng.integers(0, 3))] for _ in range(nn)]
            S = [float(rng.uniform(5, 20))]
            for t in ts:
                k = tm[t] if conform else float(rng.uniform(0, 0.9))
                k = min(0.95, max(0.0, k + float(rng.normal(0, 0.02))))
                S.append(S[-1] - k * (S[-1] - beta))
            p_.append(type_averaged_prediction(S, ts, tm))
            m_.append((S[0] - S[-1]) / (S[0] - beta))
        return np.array(p_), np.array(m_)

    pc, mc_ = run(True); pv, mv = run(False)
    ax[1].scatter(mc_, pc, s=6, c=C2, alpha=0.4, edgecolors="none",
                  label=f"conforming $r$={np.corrcoef(pc, mc_)[0,1]:.3f}")
    ax[1].scatter(mv, pv, s=6, c=C1, alpha=0.4, edgecolors="none",
                  label=f"violating $r$={np.corrcoef(pv, mv)[0,1]:.3f}")
    ax[1].plot([0, 1], [0, 1], color="0.3", ls="--", lw=1.1)
    ax[1].set_xlabel("measured net power"); ax[1].set_ylabel("type-averaged prediction")
    ax[1].set_title("Corrected test discriminates")
    ax[1].legend(frameon=False, loc="upper left")
    tag(ax[1], "B")

    # (C) eta diagnostic: discriminating power vs between-type separation
    seps = np.linspace(0.0, 0.35, 22)
    etas, gaps = [], []
    types = np.array(["t1", "t2", "t3"] * 400)
    for s in seps:
        base = {"t1": 0.45 - s, "t2": 0.45, "t3": 0.45 + s}
        pw = np.array([base[t] + rng.normal(0, 0.06) for t in types])
        etas.append(eta_separation(pw, types))
        means = {t: float(pw[types == t].mean()) for t in ("t1", "t2", "t3")}
        gaps.append(max(means.values()) - min(means.values()))
    ax[2].plot(seps, etas, "o-", color=C0, ms=3.5)
    ax[2].axhline(0.25, color=C1, ls="--", lw=1.2)
    ax[2].fill_between(seps, 0, 0.25, color=C1, alpha=0.09)
    ax[2].set_xlabel("between-type separation")
    ax[2].set_ylabel(r"$\eta$")
    ax[2].set_title(r"$\eta\approx0$: binding is uninformative")
    ax[2].set_ylim(0, 1.02)
    tag(ax[2], "C")

    # (D) 3D: |prediction - measurement| over (n, distortion) for the two tests
    nn = np.arange(2, 9)
    dist = np.linspace(0.0, 1.2, 14)
    Zi = np.zeros((len(nn), len(dist)))     # instance-fitted (degenerate)
    Zt = np.zeros((len(nn), len(dist)))     # type-averaged (corrected)
    for i, n_ in enumerate(nn):
        for j, d in enumerate(dist):
            ei, et = [], []
            for _ in range(60):
                ts = [tl[int(rng.integers(0, 3))] for _ in range(int(n_))]
                S = [float(rng.uniform(5, 20))]
                for tname in ts:
                    k = tm[tname] * (1.0 + d * rng.normal())
                    k = min(0.95, max(0.005, k))
                    S.append(S[-1] - k * (S[-1] - beta))
                p, m = instance_specific_prediction(S, beta)
                ei.append(abs(p - m))
                et.append(abs(type_averaged_prediction(S, ts, tm) - m))
            Zi[i, j] = np.mean(ei); Zt[i, j] = np.mean(et)
    X, Y = np.meshgrid(dist, nn)
    ax[3].plot_surface(X, Y, Zt, cmap="magma", edgecolor="none", alpha=0.92,
                       rcount=40, ccount=40)
    ax[3].plot_wireframe(X, Y, Zi, color=C0, lw=0.7, rcount=8, ccount=8)
    ax[3].set_xlabel("process distortion"); ax[3].set_ylabel("cascade length")
    ax[3].set_zlabel("|pred - meas|")
    ax[3].set_title("Corrected error grows; fitted stays at 0")
    ax[3].set_zlim(0, max(Zt.max() * 1.15, 1e-3))
    ax[3].view_init(elev=24, azim=-58)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel7"] = {"degenerate_r": float(r),
                        "conforming_r": float(np.corrcoef(pc, mc_)[0, 1]),
                        "violating_r": float(np.corrcoef(pv, mv)[0, 1]),
                        "eta_sweep_sep": seps.tolist(), "eta_sweep": etas,
                        "err_instance_max": float(Zi.max()),
                        "err_typeavg_max": float(Zt.max())}
    save(fig, 7)


# =====================================================================
#  PANEL 8 -- Runtime, pruning, and the Zurich floor
# =====================================================================
def panel8():
    rng = np.random.default_rng(SEED + 7)
    fig, ax = new_panel(threed=(3,))
    zb, zp = zurich()

    # (A) measured inter-district separation, all 15 receiver pairs
    M = np.array([[q["yes_share"][d] for d in zb["districts"]]
                  for q in zb["questions"]])
    D = zb["districts"]
    labels, costs = [], []
    for i in range(len(D)):
        for j in range(i + 1, len(D)):
            labels.append(f"{D[i].replace('Kreis ','K')}|{D[j].replace('Kreis ','K')}")
            costs.append(float(np.abs(M[:, i] - M[:, j]).mean()))
    o = np.argsort(costs)
    ax[0].bar(range(len(costs)), np.array(costs)[o], color=C0, alpha=0.9)
    ax[0].axhline(min(costs), color=C1, ls="--", lw=1.3)
    ax[0].set_xticks(range(len(costs)))
    ax[0].set_xticklabels([labels[k] for k in o], rotation=90, fontsize=5.5)
    ax[0].set_ylabel("mean separation")
    ax[0].set_title(f"Zurich floor $\\beta$={min(costs):.4f} over 15 pairs")
    tag(ax[0], "A")

    # (B) floor estimators on processes with and without a floor
    def smin(x): return float(np.min(np.abs(x)))

    def asym(x, r, reps=40):
        v = np.abs(np.asarray(x, float))
        ns = np.unique(np.geomspace(20, v.size, 10).astype(int))
        mins = [float(np.mean([np.min(r.choice(v, n, replace=False))
                               for _ in range(reps)])) for n in ns]
        A = np.vstack([1.0 / ns, np.ones(len(ns))]).T
        c, *_ = np.linalg.lstsq(A, np.array(mins), rcond=None)
        return float(max(0.0, c[1]))

    N = 4000
    wf = rng.uniform(0.5, 1.0, N) * rng.choice([-1, 1], N)
    nf = rng.uniform(0, 1, N) ** 3 * rng.choice([-1, 1], N)
    vals = [[smin(wf), smin(nf)], [asym(wf, rng), asym(nf, rng)]]
    x = np.arange(2)
    FLOORV = 1e-14
    pres = [max(vals[0][0], FLOORV), max(vals[1][0], FLOORV)]
    absn = [max(vals[0][1], FLOORV), max(vals[1][1], FLOORV)]
    ax[1].bar(x - 0.19, pres, 0.38, color=C0, label="floor present")
    ax[1].bar(x + 0.19, absn, 0.38, color=C1, label="floor absent")
    ax[1].set_yscale("log")
    ax[1].set_ylim(FLOORV, 3.0)
    ax[1].axhline(0.5, color="0.35", ls=":", lw=1.1)
    for xi, v in zip(x + 0.19, [vals[0][1], vals[1][1]]):
        ax[1].annotate(f"{v:.0e}" if v > 0 else "0",
                       (xi, max(v, FLOORV) * 2.2), ha="center", fontsize=6.6,
                       color=C1)
    ax[1].set_xticks(x); ax[1].set_xticklabels(["sample-min", "asymptotic"])
    ax[1].set_ylabel("estimate (log)")
    ax[1].set_title("Only the asymptotic estimator discriminates")
    ax[1].legend(frameon=False, loc="lower left")
    tag(ax[1], "B")

    # (C) trajectory divergence: distinct edge sets over one fixed node set
    from runtime import Runtime
    seen, curve = set(), []
    for t in range(1, 121):
        rt = Runtime()
        for i in range(6):
            rt.identify(f"n{i}")
        rt.emit(f"n{int(rng.integers(0,6))}", 1.0)
        for _ in range(6):
            s = f"n{int(rng.integers(0,6))}"; d = f"n{int(rng.integers(0,6))}"
            if rt.read(s):
                rt.emit(d, float(rng.random()), cause=s)
        seen.add(frozenset(rt.edges)); curve.append(len(seen))
    ax[2].plot(range(1, 121), curve, color=C0)
    ax[2].fill_between(range(1, 121), 0, curve, color=C0, alpha=0.12)
    ax[2].set_xlabel("runs over one fixed node set")
    ax[2].set_ylabel("distinct trajectories")
    ax[2].set_title("Node set durable, trajectory is not")
    tag(ax[2], "C")

    # (D) 3D: pruning blast radius b^(D-k)
    bs = np.array([2, 3, 4, 5])
    depth = 5
    ks = np.arange(1, depth + 1)
    B, Kk = np.meshgrid(bs, ks)
    Z = np.log10(B.astype(float) ** (depth - Kk))
    ax[3].plot_surface(B, Kk, Z, cmap=CMAP, edgecolor="none", alpha=0.95,
                       rcount=30, ccount=30)
    ax[3].set_xlabel("branching $b$"); ax[3].set_ylabel("edit depth $k$")
    ax[3].set_zlabel(r"$\log_{10}$ leaves touched")
    ax[3].set_title("Module to individual: prefix containment")
    ax[3].view_init(elev=24, azim=-56)
    style3d(ax[3]); tag(ax[3], "D", threed=True)

    SERIES["panel8"] = {"pair_labels": labels, "pair_costs": costs,
                        "beta_zurich": float(min(costs)),
                        "estimators": {"sample_min": vals[0],
                                       "asymptotic": vals[1]},
                        "trajectory_curve": curve}
    save(fig, 8)


def main():
    print("generating panels ->", FIGS)
    for fn in (panel1, panel2, panel3, panel4, panel5, panel6, panel7, panel8):
        fn()
    with open(os.path.join(RESULTS, "panel_data.json"), "w", encoding="utf-8") as fh:
        json.dump(SERIES, fh, indent=1, default=float)
    print("series ->", os.path.join(RESULTS, "panel_data.json"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
