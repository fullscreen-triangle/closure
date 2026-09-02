# Contributing

Thank you for looking. A few things about this repository are unusual, and
knowing them in advance will save you time.

## The constraints are the product

Several things this system does not do look like gaps. They are not. Before
proposing a feature that reports progress, scores a session, or attributes an
outcome to a player action, read Section 12 of the manuscript
(`zuerich/docs/zuerich-common-closure`). Each refusal is a theorem, and a
version of the instrument that lifted one would be evidence that the theory it
implements is wrong.

If you believe a refusal is mistaken, that is a genuinely interesting claim —
open an issue arguing against the theorem, not a pull request routing around
it.

## Before you open a pull request

```bash
make check      # fmt, clippy, eslint, typecheck, tests
make validate   # the 24-experiment suite
```

Both must pass. CI runs the same commands.

## Where to make a change

| Change | Crate |
|---|---|
| A guarantee, an invariant, closure itself | `closure-kernel` |
| Agent behaviour, scenes, the node runtime | `closure-runtime` |
| Anything a player types in a terminal | `closure-cli` |
| API surface, sessions | `closure-server` |
| The interaction surface | `web/` |
| Theory, validation, figures | `zuerich/` |

`closure-kernel` is the load-bearing crate. A change there should come with a
test that would have failed before it, and should say which numbered result in
the manuscript it corresponds to.

## Style

- Rust: `cargo fmt`, and clippy clean at `-D warnings`.
- TypeScript: Prettier and ESLint, both enforced.
- Comments explain *why*, and cite the theorem where one applies. Comments
  that restate the code get deleted in review.
- Tests are named after the property they establish, not the function they
  call: `closure_is_stronger_than_any_threshold`, not `test_is_closed`.

## Tests that encode theory

Some tests exist to catch drift in the guarantees rather than in the code:

- `no_route_reports_a_verdict_or_an_attribution` is a tripwire on the API
  surface. If you add a route, add it to `DECLARED_ROUTES` too — a companion
  test fails if the list goes stale.
- `provenance_is_not_read` and `truth_is_not_read` will fail if any code path
  starts consulting a consideration's label or veridicality. That is the
  point; they are the executable form of Theorems 4.9 and 4.11.

## The validation suite

`zuerich/validation/run_validation.py` contains two experiments (E21, E22)
whose *expected result is that a method fails*. They are negative controls. If
you make them pass in the ordinary sense, you have broken them.

Instance-fitted effect sizes must never be exposed through the API or the UI.
The only composition test with a null is the type-averaged one, and it ships
with the `eta` diagnostic that says when even that has no discriminating
power.

## Data

`zuerich/data/raw/` is gitignored: the raw CSVs are large and refetchable with
`make data`. The normalised JSON they produce *is* committed, so the
validation suite runs without network access.

Never commit anything that identifies a real resident. Modules are patterns;
that is a commitment of the design, not a policy layered on top of it.

## Commits

Present tense, imperative, and say what changed rather than what file you
touched. If a change is motivated by a result in the manuscript, name it.

## License

Contributions are accepted under AGPL-3.0-or-later.
