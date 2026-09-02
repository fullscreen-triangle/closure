# Architecture

Four crates, one web app, and a research directory. Only one crate is
load-bearing.

```
        player
          |
     closure-cli ----- mints a token ----+
          |                              |
      (browser)                          v
          |                        closure-server
       web/  <---- /v1 ---------->  sessions, worlds
                                         |
                                  closure-runtime
                                  nodes, agents, scenes
                                         |
                                   closure-kernel
                              graphs, closure, invariants
```

## closure-kernel

The guarantees. Contact graphs and the floor, considerations and closure,
identity as the pair `(chi, m)`, outcomes, and session tokens.

Three invariants are enforced by the type system rather than by discipline:

- `Record` exposes `commit` and `undo` and nothing else. There is no
  `decrement`, no `reset`, no `set`. Invariant 2 holds because the operation
  that would break it does not exist.
- `Agent::determine` checks the phase before running and checks the deposit
  after, returning `InvariantViolation` for either. Invariants 4 and 5 cannot
  be forgotten at a call site.
- No item in the crate compares an achieved state to an expected one. That
  absence is Invariant 6.

`Consideration` carries `provenance` and `veridical` fields that **nothing
reads**. They exist so that provenance- and truth-blindness can be stated and
tested: one cannot permute what is not there.

## closure-runtime

The semantically inert runtime. Its vocabulary is four operations — identify,
read, transform, emit — and there is no fifth. `transform` is deliberately
absent from the `Runtime` type itself, because it is internal to a module and
invisible to the runtime.

Consequences worth knowing before reading the code:

- `execute` runs *every* chunk on a node. A failing chunk yields an error
  value, which is emitted like any other; the run does not halt and no
  emission suppresses another. This costs real cycles on inputs a human would
  already know are corrupt, and that is the right trade for an instrument.
- Edges are produced by reading, not declared. The trajectory is a fixed point
  of the run and is not schedulable in advance, so reproducibility attaches to
  the protocol fingerprint rather than to any reading.
- `converge` is a join-semilattice operation, so the node set does not depend
  on contribution order. Two agents arriving at the same subtask land on one
  node with no merge protocol.

`attention::water_fill` is the one place whose *optimality* is conditional on
the environment rather than on the agent: it needs concave gain profiles. Give
it a non-concave scene and you get a rational obsessive, which is a modelling
choice rather than a defect.

## closure-cli

What a player downloads. Authenticates, mints a token, opens the browser.
Holds no world state — that is deliberate, so that a session survives the
laptop closing and several people can share a city.

`closure doctor` is the auditable surface: it prints the invariants and the
refusals without issuing any verdict of its own.

## closure-server

Owns the world. Sessions map a token to a `Runtime` plus its opening time.

The route table is short and deliberately incomplete in one direction. There
is no `/score`, `/progress`, or `/attribution`, and a test enforces this
against a declared route list, with a companion test that fails if the list
goes stale.

## web/

React over Vite. Thin: it renders what the session has accumulated and lets a
player talk. Types in `lib/api.ts` are hand-mirrored from the Rust handlers,
which means a shape change needs editing in two places and CI will not catch
the drift.

## zuerich/

The research half: the manuscript, the Python reference implementation, the
24-experiment validation suite, and the normalised substrate.

The Python prototype and the Rust kernel implement the same theory twice, on
purpose. Agreement between them is evidence; a divergence is a bug in one of
them and worth chasing.

## Adding a city

A city is a substrate binding, not a new engine. It discharges four
obligations — receivers, observable, events, floor — and the language depends
on nothing else about it. The floor estimator is the one with empirical
content: discharge it with a sample minimum and it is positive whatever the
data, which tells you nothing.
