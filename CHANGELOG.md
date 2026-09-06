# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `closure-kernel`: contact graphs and the floor theorem, considerations and
  closure, identity as the pair `(chi, m)`, outcomes with declination as a
  first-class value, and session tokens over an unambiguous alphabet.
- `closure-runtime`: the four-operation node runtime, water-filling attention
  allocation, and pruning a voice into a character by capping and amalgamating
  the moderators it was heard by — never by matching it against a catalogue.
- `closure-cli`: `login`, `session new`, `cities`, `doctor`, `where`.
- `closure-server`: session exchange and the read-only session view.
- `web/`: token entry and the session panel.
- `closure-server`: a **generated society per session**. The graph, its
  overlapping regions, their affordances and the characters cut from them are
  drawn from the session seed; no city is modelled. Replaces a hand-written
  thirty-one position Zürich, which was invented without being generated —
  the worst of both, since the goal is unreachable in any society and a
  carefully-built one bought nothing a drawn one does not.
- `closure-server`: the weather keys off a region's **affordance** (water,
  open, transit, indoors) rather than its name, so the same four rules hold in
  every society the generator can produce. What is under a roof is never
  reached, in any of them.
- The manuscript, its eight panels, and a 24-experiment validation suite.

### Changed

- `closure-server` binds `127.0.0.1:8080` by default rather than `0.0.0.0`.
  The API has no authentication beyond a locally minted session token, so
  serving every interface was not a safe default to ship.
- `GET /v1/cities` no longer publishes a measured floor or a data provenance
  for a city. Both described a substrate the host does not use: the society is
  generated per session, so a floor stated before the seed is known measures
  nothing.

### Removed

- `--data-dir` / `CLOSURE_DATA_DIR`. It pointed at a directory of "city
  substrates" that never existed and was read by nothing. A flag promising a
  data source is worse than no flag, because it tells a reader the substrate
  came from somewhere.

### Notes

Three capabilities are absent by construction rather than pending: success or
failure reporting, attribution of an outcome to an action, and instance-fitted
effect sizes. See `closure doctor` or Section 12 of the manuscript.
