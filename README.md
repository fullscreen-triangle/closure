# closure

A runtime for non-convergent social deliberation, and a game played inside it.

You arrive in a city. You can talk to anyone in it. You are trying to get
something built — a river transport system, a school, a course of study, a
bridge. The people you talk to are not waiting for you: they have their own
purposes, their own finite attention divided across contexts you never see,
their own questions, and their own capacity to decline. They do not know they
are in a game, and there is nothing to win.

The name is the technical term and the ordinary one, and the argument is that
they are the same word.

---

## Why there is nothing to win

Zürich is not poor, not technically backward, and not opposed to river
transport. It simply has none, and no one there experiences that as a
decision. That is the ordinary condition of most feasible proposals in most
functioning polities, and neither belief-based nor preference-based accounts
predict it.

The explanation the runtime implements is that **commitment is closure** — an
agent commits when every further consideration available to it resolves into
a region it has already reached — and that **closure is blind**: the condition
quantifies over resolutions and mentions neither where a consideration came
from nor whether it is true. A proposal can therefore be believed, reachable,
and never committed to, because some available-but-uninvoked consideration
resolves elsewhere. A failure of closure is not an event. Nothing happened, so
there is nothing to revisit.

If a city could be moved by finding the right argument, someone would have
found it. That the argument does not exist is the premise, not an excuse for a
missing win screen.

The full development is in
[`zuerich/docs/zuerich-common-closure`](zuerich/docs/zuerich-common-closure/)
— 47 pages, self-contained, with 24 validation experiments that pass.

---

## What the instrument refuses to do

These are consequences of the theory, enforced in the code, not features that
are missing:

| It will not | Because |
|---|---|
| Report success or failure | No quantity computable from the runtime state compares an achieved state to an expectation (Thm 11.5) |
| Tell you which action mattered | Admissibility is a global minimum expressed in local terms, and the terminus does not determine the trajectory (Thm 12.9, 11.15) |
| Show instance-fitted effect sizes | Prediction and measurement are the same algebraic expression, so the statistic is degenerate on every data set (Thm 12.2) |

Run `closure doctor` to see all six implementation invariants and what
certifies each.

A version of this instrument that reported an effect attribution would be
evidence that the theory it implements is wrong.

---

## Getting started

```bash
# Install the CLI (or download a release binary)
cargo install --path crates/closure-cli

closure login
closure session new            # prints a token and opens the web surface
```

The token looks like `CLOSURE-K4M2-9XPQ-7RTF`. Paste it into the web app and
you are in the city.

### Running the whole thing locally

```bash
make setup      # fetch Rust and Node dependencies
make dev        # API on :8080, web on :5173
```

Or with containers:

```bash
docker compose up --build     # web on :8081, API proxied at /v1
```

---

## Layout

```
crates/
  closure-kernel/    contact graphs, closure, the six invariants
  closure-runtime/   nodes, agents, scenes, the induced trajectory
  closure-cli/       the binary a player downloads
  closure-server/    Axum host; owns the world
web/                 React interaction surface
zuerich/
  docs/              the manuscript and its eight panels
  prototype/         Python reference implementation
  validation/        24 experiments and the Zürich substrate binding
  data/              normalised open government data
docker/              images for server and web
```

Three crates and one web app, but only one of them is load-bearing:
`closure-kernel` holds the guarantees, and the rest are ways of reaching them.

---

## Development

```bash
make check      # everything CI runs: fmt, clippy, eslint, tests
make validate   # the 24-experiment suite from the manuscript
make paper      # build the manuscript
make panels     # regenerate the eight figures
```

`make help` lists every target.

### Where the invariants live

The three that matter most are enforced by types rather than by convention,
so they cannot be forgotten:

- `Record` has no `decrement`, no `reset`, and no `set`. An undo is a
  compensating commit that raises the count.
- `Agent::determine` refuses to run outside a commitment phase, and errors if
  a determination returned without depositing.
- Nothing in the workspace compares an achieved determination to an expected
  one. The absence is the enforcement.

---

## The Zürich binding

The substrate is the public record of the City of Zürich, used as a source of
**vocabulary and heterogeneity**, never to reconstruct persons:

- 11,406 ballot rows since 1933; 275 municipal questions on six consistently
  reported units, 2002–2026
- 408,639 population records by district, origin, sex and age, 1993–2025

Measured floor on this binding: **β = 0.0197** — the two most similar
districts differ by about two percentage points on average across the record.

Modules are *patterns*, never persons. Pruning them against a demographic
record yields fictional agents consistent with aggregate statistics. See
[`zuerich/README.md`](zuerich/README.md) for the ethical commitments.

Other cities are new substrate bindings, not new engines: the language depends
on a substrate only through four obligations.

---

## Status

Early. The kernel, the runtime, the CLI token flow and the API skeleton are
real and tested; the conversation surface is the next piece. The manuscript
and its validation suite are complete.

## License

AGPL-3.0-or-later. See [LICENSE](LICENSE).
