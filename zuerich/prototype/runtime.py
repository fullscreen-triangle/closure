"""
closure/zuerich -- prototype runtime.

Reference implementation of the runtime specified in
    docs/zuerich-common-closure/zuerich-common-closure.tex

Every public object here corresponds to a numbered definition in the paper,
and every enforced constraint corresponds to a numbered Invariant.  Section
references in docstrings point at the manuscript.

The module is deliberately dependency-light (numpy only) so that the
validation suite can be audited without reading a framework.

Run `python runtime.py` for a self-test of the six implementation invariants.
"""

from __future__ import annotations

import hashlib
import itertools
import json
import math
from dataclasses import dataclass, field
from typing import Callable, Dict, FrozenSet, Iterable, List, Optional, Sequence, Set, Tuple

import numpy as np

__all__ = [
    "ContactGraph", "min_cut_weight", "separation_cost", "character_invariant",
    "Consideration", "reach", "is_closed", "commits_to", "three_routes",
    "water_fill", "Agent", "Phase", "kuramoto_step", "order_parameter",
    "critical_coupling", "catalytic_power", "compose_powers", "saturates",
    "support_is_robust", "shortest_support_cycle",
    "Node", "Runtime", "Module", "prune",
    "SeekOutcome", "Resolved", "Declined", "seek",
    "type_averaged_prediction", "instance_specific_prediction", "eta_separation",
    "InvariantViolation",
]


# =====================================================================
#  Errors
# =====================================================================

class InvariantViolation(RuntimeError):
    """Raised when an implementation invariant of Sec. 12 would be broken."""


class TypeError_(Exception):
    """Static rejection by the type system of Sec. 10."""


# =====================================================================
#  Part II.1 -- Contact graphs and the floor  (Sec. 3)
# =====================================================================

@dataclass
class ContactGraph:
    """Definition 2.1 (contact graph).

    A finite, connected, positively weighted simple graph.  Vertices are
    'positions'; w(uv) is the cost of telling u and v apart.
    """
    n: int
    weights: Dict[FrozenSet[int], float] = field(default_factory=dict)

    # ---- construction -------------------------------------------------
    def add_edge(self, u: int, v: int, w: float) -> None:
        if u == v:
            raise ValueError("contact graphs are simple: no self-loops")
        if w <= 0:
            raise ValueError("Axiom 3 (costly individuation): weights must be > 0")
        self.weights[frozenset((u, v))] = float(w)

    @property
    def vertices(self) -> List[int]:
        return list(range(self.n))

    @property
    def beta(self) -> float:
        """The floor: the least edge weight (Axiom 3)."""
        if not self.weights:
            raise ValueError("empty graph has no floor")
        return min(self.weights.values())

    def neighbours(self, v: int) -> Set[int]:
        out = set()
        for e in self.weights:
            a, b = tuple(e)
            if a == v:
                out.add(b)
            elif b == v:
                out.add(a)
        return out

    def is_connected(self) -> bool:
        if self.n == 0:
            return False
        seen, stack = {0}, [0]
        while stack:
            for w in self.neighbours(stack.pop()):
                if w not in seen:
                    seen.add(w)
                    stack.append(w)
        return len(seen) == self.n

    def relabel(self, perm: Sequence[int]) -> "ContactGraph":
        """Apply a vertex relabelling; used to test Invariant 1."""
        g = ContactGraph(self.n)
        for e, w in self.weights.items():
            a, b = tuple(e)
            g.add_edge(perm[a], perm[b], w)
        return g

    def induced(self, subset: Iterable[int]) -> "ContactGraph":
        s = sorted(set(subset))
        idx = {v: i for i, v in enumerate(s)}
        g = ContactGraph(len(s))
        for e, w in self.weights.items():
            a, b = tuple(e)
            if a in idx and b in idx:
                g.add_edge(idx[a], idx[b], w)
        return g


def cut_weight(g: ContactGraph, A: Iterable[int]) -> float:
    """w(cut(A)) of Definition 2.2."""
    A = set(A)
    total = 0.0
    for e, w in g.weights.items():
        a, b = tuple(e)
        if (a in A) != (b in A):
            total += w
    return total


def min_cut_weight(g: ContactGraph) -> Tuple[float, FrozenSet[int]]:
    """Res(G) of Definition 2.2, by exhaustive bipartition.

    Exponential by design: the validation suite uses small graphs and we
    prefer an obviously-correct brute force to a clever routine, so that
    agreement with the theorems is not an artefact of shared cleverness
    (Sec. 13.1, first convention).
    """
    best, best_A = math.inf, frozenset()
    verts = g.vertices
    for r in range(1, len(verts)):
        for A in itertools.combinations(verts, r):
            c = cut_weight(g, A)
            if c < best:
                best, best_A = c, frozenset(A)
    return best, best_A


def separation_cost(g: ContactGraph) -> float:
    return min_cut_weight(g)[0]


def character_invariant(g: ContactGraph) -> float:
    """chi(A) of Definition 9.2: least total cost of splitting into r >= 2 blocks.

    For r = 2 this coincides with the minimum cut; higher r never lowers the
    value on a connected graph, so we compute over bipartitions.
    """
    return separation_cost(g)


# =====================================================================
#  Part II.2 -- Considerations, reach, closure  (Sec. 4)
# =====================================================================

@dataclass(frozen=True)
class Consideration:
    """Definition 4.1 (consideration).

    `provenance` is an arbitrary label carrying no structure: no operation in
    this module reads it (Remark 4.2).  `resolution` maps a seed position to
    the sufficient region reached.  `veridical` exists only so that
    Theorem 4.11 (truth-blindness) can be *stated* and tested; nothing reads
    it either (Remark 4.9).
    """
    name: str
    provenance: str
    resolution: Dict[int, FrozenSet[int]]
    veridical: bool = True
    ctype: str = "generic"          # event type, for the corrected test (Def. 11.4)

    def resolve(self, seed: int) -> Optional[FrozenSet[int]]:
        return self.resolution.get(seed)


def reach(seed: int, gamma: Iterable[Consideration]) -> Set[FrozenSet[int]]:
    """Reach(v0, Gamma) of Definition 4.3."""
    out = set()
    for c in gamma:
        r = c.resolve(seed)
        if r is not None:
            out.add(r)
    return out


def is_closed(seed: int,
              invoked: Iterable[Consideration],
              available: Iterable[Consideration]) -> bool:
    """Cl(v0, Gamma', Gamma) of Definition 4.3."""
    return reach(seed, invoked) == reach(seed, available)


def commits_to(seed: int,
               cell: FrozenSet[int],
               invoked: Iterable[Consideration],
               available: Iterable[Consideration]) -> bool:
    """Definition 4.4 (commitment)."""
    invoked, available = list(invoked), list(available)
    return is_closed(seed, invoked, available) and cell in reach(seed, invoked)


def three_routes(seed: int,
                 cell: FrozenSet[int],
                 invoked: Iterable[Consideration],
                 available: Iterable[Consideration],
                 sufficient: Set[FrozenSet[int]]) -> str:
    """Proposition 4.14: classify a non-commitment into exactly one route.

    Returns 'committed', 'ignorance' (i), 'inertia' (ii), or 'confusion' (iii).
    """
    invoked, available = list(invoked), list(available)
    if cell not in sufficient:
        return "confusion"                      # (iii) not tau-sufficient
    if cell not in reach(seed, invoked):
        return "ignorance"                      # (i) no invoked consideration reaches it
    if not is_closed(seed, invoked, available):
        return "inertia"                        # (ii) closure fails
    return "committed"


# =====================================================================
#  Part II.3 -- Attention: water-filling and phase  (Sec. 5)
# =====================================================================

def water_fill(budget: float,
               marginal_inverse: Sequence[Callable[[float], float]],
               marginal_at_zero: Sequence[float],
               tol: float = 1e-12,
               max_iter: int = 200) -> Tuple[np.ndarray, float]:
    """Algorithm 1: water-filling allocation (Theorem 5.2).

    `marginal_inverse[i](p)` returns (gamma_i')^{-1}(p); `marginal_at_zero[i]`
    is gamma_i'(0).  Returns (allocation, price).  Bisection converges because
    sum_i (gamma_i')^{-1}(p) is continuous and non-increasing in p under
    Axiom 6 (concavity).
    """
    k = len(marginal_inverse)
    lo, hi = 0.0, max(marginal_at_zero) if k else 0.0

    def alloc_at(p: float) -> np.ndarray:
        a = np.zeros(k)
        for i in range(k):
            if marginal_at_zero[i] > p:
                a[i] = max(0.0, marginal_inverse[i](p))
        return a

    if alloc_at(0.0).sum() <= budget:
        return alloc_at(0.0), 0.0               # budget not binding => price 0

    for _ in range(max_iter):
        if hi - lo <= tol:
            break
        p = 0.5 * (lo + hi)
        if alloc_at(p).sum() > budget:
            lo = p
        else:
            hi = p
    p = 0.5 * (lo + hi)
    return alloc_at(p), p


class Phase:
    """Axiom 5 (phase exclusion): exactly one of two phases per instant."""
    CONSTRUCTION = "construction"
    COMMITMENT = "commitment"


# =====================================================================
#  Part III.1 -- The agent  (Sec. 9)
# =====================================================================

@dataclass
class Agent:
    """Definition 9.6 (agent): a contact graph paired with a renderer.

    Enforces Invariants 1-5.  The `render` callable stands in for the
    generative component; the prototype does not require a language model,
    and the validation suite passes a deterministic stub, because no theorem
    depends on any property of the renderer beyond its depositing (Thm 9.7).
    """
    ident: str
    graph: ContactGraph
    budget: float = 1.0
    omega: float = 1.0                            # intrinsic phase velocity
    phi: float = 0.0                              # outward phase
    record: int = 0                               # Invariant 2: monotone
    phase: str = Phase.COMMITMENT
    scenes: Dict[str, float] = field(default_factory=dict)   # scene -> richness k_i
    _char: Optional[float] = field(default=None, repr=False)

    def __post_init__(self) -> None:
        self._char = character_invariant(self.graph)

    # ---- Invariant 1 ---------------------------------------------------
    @property
    def chi(self) -> float:
        """Conserved character invariant (Theorem 9.3)."""
        return self._char

    # ---- Invariant 2 ---------------------------------------------------
    def _commit_act(self, cost: float = 1.0) -> None:
        if cost <= 0:
            raise InvariantViolation("Axiom 4: an act of zero cost is not an act")
        self.record += 1

    def undo(self) -> None:
        """Theorem 5.9(ii): 'undo' is a compensating commit, never a decrement."""
        self._commit_act()

    # ---- Invariant 5 ---------------------------------------------------
    def set_phase(self, phase: str) -> None:
        if phase not in (Phase.CONSTRUCTION, Phase.COMMITMENT):
            raise ValueError(phase)
        self.phase = phase

    # ---- Invariants 3 + 4 ----------------------------------------------
    def determine(self,
                  seed: int,
                  invoked: Sequence[Consideration],
                  available: Sequence[Consideration]) -> "SeekOutcome":
        """A determination: freshly computed, and depositing before returning.

        Invariant 3 (search not lookup): no cache is consulted.
        Invariant 4 (deposit): the record strictly increases before return.
        Invariant 5 (phases): forbidden during a construction phase.
        """
        if self.phase != Phase.COMMITMENT:
            raise InvariantViolation(
                "Invariant 5: no act may be emitted during a construction phase")
        before = self.record
        outcome = seek(seed, invoked, available)
        self._commit_act()
        if not self.record > before:
            raise InvariantViolation("Invariant 4: determination did not deposit")
        return outcome

    def water_fill_scenes(self) -> Tuple[Dict[str, float], float]:
        """Allocate capacity across concurrent scenes (Corollary 5.3).

        Uses gamma_i(a) = k_i * log(1 + a), so gamma_i'(a) = k_i / (1 + a),
        which is concave (Axiom 6) with inverse (k_i / p) - 1.
        """
        names = sorted(self.scenes)
        ks = [self.scenes[n] for n in names]
        inv = [(lambda p, k=k: (k / p) - 1.0 if p > 0 else math.inf) for k in ks]
        at0 = [k / 1.0 for k in ks]
        a, price = water_fill(self.budget, inv, at0)
        return {n: float(v) for n, v in zip(names, a)}, price


# =====================================================================
#  Part II.4 -- Coordination  (Sec. 6)
# =====================================================================

def order_parameter(phases: np.ndarray) -> Tuple[float, float]:
    """R e^{i psi} of Definition 6.4."""
    z = np.exp(1j * np.asarray(phases)).mean()
    return float(abs(z)), float(np.angle(z))


def kuramoto_step(phases: np.ndarray,
                  omegas: np.ndarray,
                  K: float,
                  dt: float) -> np.ndarray:
    """One Euler step of the mean-field flow (Definition 6.6)."""
    R, psi = order_parameter(phases)
    return phases + dt * (omegas + K * R * np.sin(psi - phases))


def critical_coupling(omegas: np.ndarray) -> float:
    """Kc = 2 / (pi g(0)) for a unimodal symmetric velocity law (Thm 6.7).

    Estimated from a Gaussian fit, for which g(0) = 1/(sqrt(2 pi) sigma) and
    hence Kc = 2 sigma sqrt(2 pi) / pi.
    """
    sigma = float(np.std(omegas))
    if sigma == 0:
        return 0.0
    return 2.0 * sigma * math.sqrt(2.0 * math.pi) / math.pi


def coordination_cost(R: float, lam: float = 1.0) -> float:
    """C = lambda (1 - R) of Definition 6.4; vanishes iff R = 1 (Thm 6.5)."""
    return lam * (1.0 - R)


# =====================================================================
#  Part II.5 -- Composition  (Sec. 7)
# =====================================================================

def catalytic_power(S_before: float, S_after: float, beta: float) -> float:
    """kappa of Definition 7.1, clamped to [0, 1] by Lemma 7.2."""
    denom = S_before - beta
    if denom <= 0:
        return 0.0
    k = (S_before - S_after) / denom
    return float(min(1.0, max(0.0, k)))


def compose_powers(kappas: Sequence[float]) -> float:
    """1 - prod(1 - k_i) of Theorem 7.3."""
    prod = 1.0
    for k in kappas:
        prod *= (1.0 - k)
    return 1.0 - prod


def saturates(kappas: Sequence[float], tol: float = 1e-9) -> bool:
    """Theorem 7.6: residual -> 0 iff sum kappa_i diverges.

    On a finite prefix we cannot observe a limit, so we report the
    operational criterion: the partial sum is large enough that the residual
    product has fallen below `tol`.  Callers testing the dichotomy should
    supply prefixes long enough for the divergent case to clear it.
    """
    resid = 1.0
    for k in kappas:
        resid *= (1.0 - k)
        if resid < tol:
            return True
    return False


def shortest_support_cycle(edges: Set[Tuple[int, int]], n: int) -> Optional[int]:
    """Length of the shortest directed cycle in a support graph, or None."""
    best = None
    adj = {i: set() for i in range(n)}
    for a, b in edges:
        adj[a].add(b)
    for start in range(n):
        # BFS for shortest path start -> start
        dist = {start: 0}
        queue = [start]
        while queue:
            u = queue.pop(0)
            for v in adj[u]:
                if v == start:
                    L = dist[u] + 1
                    best = L if best is None else min(best, L)
                elif v not in dist:
                    dist[v] = dist[u] + 1
                    queue.append(v)
    return best


def support_is_robust(edges: Set[Tuple[int, int]], n: int) -> bool:
    """Theorem 7.9: robust iff a directed cycle of length >= 3 exists."""
    L = shortest_support_cycle(edges, n)
    return L is not None and L >= 3


# =====================================================================
#  Part III.3 -- The language: seek / Resolved / Declined  (Sec. 10)
# =====================================================================

class SeekOutcome:
    """Definition 10.3: Outcome = Resolved(Cell) | Declined({Cell})."""


@dataclass(frozen=True)
class Resolved(SeekOutcome):
    cell: FrozenSet[int]

    def as_json(self):
        return {"outcome": "resolved", "cell": sorted(self.cell)}


@dataclass(frozen=True)
class Declined(SeekOutcome):
    cells: FrozenSet[FrozenSet[int]]

    def __post_init__(self):
        if len(self.cells) < 2:
            raise ValueError("Declined carries at least two incompatible cells")

    def as_json(self):
        return {"outcome": "declined",
                "cells": sorted([sorted(c) for c in self.cells])}


def typecheck_seek(target: Optional[FrozenSet[int]],
                   excluding: Optional[FrozenSet[int]],
                   via: Sequence[Consideration],
                   independence: Dict[str, Set[str]],
                   beta: float) -> None:
    """The two non-standard typing rules of Sec. 10.2, plus exclusion.

    T-Seek-Pos  (Def. 10.4): rejects a non-positive floor.
    T-Seek-Coh  (Def. 10.5): rejects fewer than three mutually independent
                             considerations.
    Thm 10.6                : rejects a missing exclusion clause.
    """
    if excluding is None:
        raise TypeError_("Theorem 10.6: a seek without `excluding` has no denotation")
    if not beta > 0:
        raise TypeError_("T-Seek-Pos: floor must be strictly positive")
    if via:
        if len(via) < 3:
            raise TypeError_(
                "T-Seek-Coh: fewer than three considerations cannot ground "
                "(Theorem 7.9)")
        names = [c.name for c in via]
        for a, b in itertools.combinations(names, 2):
            if b not in independence.get(a, set()) or a not in independence.get(b, set()):
                raise TypeError_(
                    f"T-Seek-Coh: {a} and {b} are not declared mutually independent")


def seek(seed: int,
         invoked: Sequence[Consideration],
         available: Sequence[Consideration]) -> SeekOutcome:
    """Operational semantics of Sec. 10.3.

    E-Invoke consumes available considerations until closure; E-Close-Res and
    E-Close-Dec are selected by |C| (Theorem 10.11, dichotomy).  Terminates in
    at most |available| invocations (Theorem 10.10).
    """
    C: Set[FrozenSet[int]] = set(reach(seed, invoked))
    pending = [c for c in available if c not in invoked]
    steps = 0
    while True:
        progressed = False
        for c in list(pending):
            r = c.resolve(seed)
            pending.remove(c)
            steps += 1
            if r is not None and r not in C:
                C.add(r)
                progressed = True
                break
        if not progressed and not pending:
            break
        if steps > len(available) + len(invoked) + 1:
            break                                     # Theorem 10.10 bound
    if not C:
        raise InvariantViolation("closure reached with empty reach set")
    if len(C) == 1:
        return Resolved(next(iter(C)))
    return Declined(frozenset(C))


# =====================================================================
#  Part III.2 -- Modules, substrate, pruning  (Sec. 8 + Sec. 11)
# =====================================================================

@dataclass(frozen=True)
class Module:
    """Definition 8.1: a tau-sufficient region -- a pattern, never a person."""
    name: str
    members: FrozenSet[int]
    predicate: Optional[Callable[[dict], bool]] = None

    def matches(self, record: dict) -> bool:
        return bool(self.predicate(record)) if self.predicate else False


def prune(modules: Sequence[Module],
          record: dict,
          shared: ContactGraph) -> Tuple[ContactGraph, List[str]]:
    """Definition 8.3 (pruning) + Theorem 8.7 (generation is determined).

    Returns the induced graph on the intersection of matching modules, and the
    names of those modules.  Deterministic in (modules, record): the same
    record pruned against the same modules always yields the same graph, hence
    the same chi (Theorem 8.7).
    """
    hits = [m for m in modules if m.matches(record)]
    if not hits:
        return ContactGraph(0), []
    members = set.intersection(*[set(m.members) for m in hits])
    if not members:
        members = set(hits[0].members)
    return shared.induced(sorted(members)), [m.name for m in hits]


def generation_digest(module_names: Sequence[str], record: dict) -> str:
    """Stable digest witnessing determinism of generation (Theorem 8.7)."""
    payload = json.dumps({"modules": sorted(module_names),
                          "record": {k: record[k] for k in sorted(record)}},
                         sort_keys=True, default=str)
    return hashlib.sha256(payload.encode()).hexdigest()[:16]


# =====================================================================
#  Part III.4 -- The global runtime  (Sec. 11)
# =====================================================================

@dataclass
class Node:
    """Definition 11.1: (subtask identity, chunk bag, value collection)."""
    theta: str
    chunks: List[Callable[[], object]] = field(default_factory=list)
    values: List[object] = field(default_factory=list)


class Runtime:
    """The semantically inert runtime of Sec. 11.

    Vocabulary is exactly four operations (Definition 11.2): identify, read,
    transform (external to the runtime), emit.  There is no fifth, and in
    particular no operation comparing a value to an expectation -- which is
    why Theorem 11.5 (no exit code) holds of this class.
    """

    def __init__(self) -> None:
        self.nodes: Dict[str, Node] = {}
        self.edges: Set[Tuple[str, str]] = set()      # induced, per run
        self.record: int = 0

    # ---- the four operations ------------------------------------------
    def identify(self, theta: str) -> Node:
        if theta not in self.nodes:
            self.nodes[theta] = Node(theta)
        return self.nodes[theta]

    def read(self, theta: str) -> List[object]:
        return list(self.identify(theta).values)

    def emit(self, theta: str, value: object, cause: Optional[str] = None) -> None:
        self.identify(theta).values.append(value)     # adjoins, never replaces
        self.record += 1
        if cause is not None and cause != theta:
            self.edges.add((cause, theta))            # Prop 11.7: produced, not scheduled

    # ---- convergence (Theorem 11.4) ------------------------------------
    def converge(self, theta: str, chunks: Iterable[Callable], values: Iterable) -> Node:
        n = self.identify(theta)
        n.chunks.extend(chunks)
        for v in values:
            if v not in n.values:
                n.values.append(v)
        return n

    # ---- execution (Definition 11.3) -----------------------------------
    def execute(self, theta: str) -> List[object]:
        """Run *every* chunk and emit each result.  Errors are values.

        Corollary 11.6 (run to completion): an exception is emitted as an
        error value and execution of the remaining chunks proceeds.
        """
        n = self.identify(theta)
        out = []
        for c in n.chunks:
            try:
                r = c()
            except Exception as exc:                  # noqa: BLE001 -- by design
                r = {"error": type(exc).__name__, "msg": str(exc)}
            out.append(r)
            self.emit(theta, r)
        return out

    def trajectory(self) -> Set[str]:
        """Definition 11.6: vertex set of the induced causal relation."""
        return {a for a, _ in self.edges} | {b for _, b in self.edges}

    def protocol_fingerprint(self) -> str:
        """Corollary 11.9: the node set is durable and hashable; the
        trajectory is not."""
        payload = json.dumps(sorted(self.nodes), sort_keys=True)
        return hashlib.sha256(payload.encode()).hexdigest()[:16]


# =====================================================================
#  Part IV -- Measurement surface  (Sec. 11 of the paper: negative results)
# =====================================================================

def instance_specific_prediction(S: Sequence[float], beta: float) -> Tuple[float, float]:
    """Definition 11.1 + Theorem 11.2 (telescoping obstruction).

    Returns (multiplicative prediction, measured net power).  These are the
    SAME algebraic expression, so the pair is degenerate on every data set --
    which is precisely what E21 confirms.  Exposed only so the suite can
    demonstrate the failure; never use it to evaluate a hypothesis.
    """
    kappas = [catalytic_power(S[i], S[i + 1], beta) for i in range(len(S) - 1)]
    predicted = compose_powers(kappas)
    denom = S[0] - beta
    measured = (S[0] - S[-1]) / denom if denom > 0 else 0.0
    return float(predicted), float(measured)


def type_averaged_prediction(S: Sequence[float],
                             types: Sequence[str],
                             type_means: Dict[str, float]) -> float:
    """Definition 11.4 + Theorem 11.5: the corrected, non-degenerate test.

    The prediction uses type means estimated *excluding* this cascade, so it
    is not a function of this cascade's own uncertainties and the telescoping
    identity is broken.
    """
    return compose_powers([type_means[t] for t in types])


def eta_separation(powers: Sequence[float], types: Sequence[str]) -> float:
    """Definition 11.6: eta = Var_between / (Var_between + Var_within).

    Proposition 11.7: if eta ~ 0 the binding is uninformative and a negative
    result is evidence about the observable, not about the process.
    """
    powers = np.asarray(powers, dtype=float)
    types = np.asarray(types)
    labels = np.unique(types)
    if len(labels) < 2:
        return 0.0
    means, within = [], []
    for t in labels:
        v = powers[types == t]
        if len(v) == 0:
            continue
        means.append(v.mean())
        within.append(v.var())
    vb = float(np.var(means))
    vw = float(np.mean(within))
    return 0.0 if (vb + vw) == 0 else vb / (vb + vw)


# =====================================================================
#  Self-test of the six implementation invariants (Sec. 12)
# =====================================================================

def _selftest() -> Dict[str, bool]:
    rng = np.random.default_rng(20260902)
    results: Dict[str, bool] = {}

    # --- build a small connected contact graph -------------------------
    g = ContactGraph(6)
    for (u, v) in [(0, 1), (1, 2), (0, 2)]:
        g.add_edge(u, v, 10.0)
    for (u, v) in [(3, 4), (4, 5), (3, 5)]:
        g.add_edge(u, v, 10.0)
    g.add_edge(2, 3, 1.0)                     # the thin bridge
    assert g.is_connected()

    # Invariant 1: chi conserved under relabelling
    chi0 = character_invariant(g)
    perm = list(rng.permutation(6))
    results["inv1_conserved_invariant"] = math.isclose(
        character_invariant(g.relabel(perm)), chi0, rel_tol=1e-12)

    # Theorem 3.1 + 3.3: floor holds, and the min cut is regional
    _, A = min_cut_weight(g)
    singleton_costs = [cut_weight(g, [v]) for v in g.vertices]
    results["thm_floor"] = chi0 >= g.beta > 0
    results["thm_regional"] = len(A) > 1 and min(singleton_costs) > chi0

    # Invariant 2: record never decreases, undo increments
    a = Agent("t", g, scenes={"work": 2.0, "player": 1.0})
    r0 = a.record
    a.undo()
    results["inv2_monotone_record"] = a.record == r0 + 1

    # Invariants 3+4: determination deposits before returning
    cells = [frozenset({0, 1, 2}), frozenset({3, 4, 5})]
    cA = Consideration("a", "verified", {0: cells[0]}, ctype="fiscal")
    cB = Consideration("b", "hearsay", {0: cells[1]}, ctype="siting")
    before = a.record
    out = a.determine(0, [cA], [cA, cB])
    results["inv4_deposit"] = a.record > before
    results["thm_dichotomy_declined"] = isinstance(out, Declined)

    # Invariant 5: acts forbidden during construction
    a.set_phase(Phase.CONSTRUCTION)
    try:
        a.determine(0, [cA], [cA])
        results["inv5_phase_exclusion"] = False
    except InvariantViolation:
        results["inv5_phase_exclusion"] = True
    a.set_phase(Phase.COMMITMENT)

    # Invariant 6 / Theorem 11.5: runtime has no exit-code operation
    rt = Runtime()
    results["inv6_no_verdict"] = not any(
        hasattr(rt, nm) for nm in ("succeeded", "failed", "exit_code", "compare"))

    # Theorem 4.6: closure strictly stronger than a threshold
    results["thm_closure_stronger"] = (
        not is_closed(0, [cA], [cA, cB]) and is_closed(0, [cA, cB], [cA, cB]))

    # Theorem 4.9/4.11: provenance- and truth-blindness
    cA2 = Consideration("a", "TOTALLY-DIFFERENT", cA.resolution, veridical=False)
    results["thm_blind"] = (reach(0, [cA, cB]) == reach(0, [cA2, cB]))

    # Theorem 5.2: water-filling equalises margins at one price
    alloc, price = a.water_fill_scenes()
    margins = [a.scenes[n] / (1.0 + alloc[n]) for n in alloc if alloc[n] > 1e-9]
    results["thm_waterfill"] = (len(margins) == 0 or
                                max(margins) - min(margins) < 1e-6)

    # Theorem 7.9: three mutually supporting considerations minimum
    results["thm_three"] = (
        support_is_robust({(0, 1), (1, 2), (2, 0)}, 3)
        and not support_is_robust({(0, 1), (1, 0)}, 2)
        and not support_is_robust({(0, 1), (1, 2)}, 3))

    # Theorem 10.6: seek without exclusion does not typecheck
    try:
        typecheck_seek(cells[0], None, [cA, cB], {}, g.beta)
        results["thm_exclusion_required"] = False
    except TypeError_:
        results["thm_exclusion_required"] = True

    # Theorem 11.2: the instance-specific test is degenerate
    S = [10.0, 7.0, 5.0, 4.0]
    pred, meas = instance_specific_prediction(S, beta=1.0)
    results["thm_telescoping"] = math.isclose(pred, meas, rel_tol=1e-12)

    return results


if __name__ == "__main__":
    res = _selftest()
    width = max(len(k) for k in res)
    for k in sorted(res):
        print(f"{k:<{width}}  {'PASS' if res[k] else 'FAIL'}")
    n_ok = sum(res.values())
    print(f"\n{n_ok}/{len(res)} checks passed")
    raise SystemExit(0 if n_ok == len(res) else 1)
