# Rune Kit Graph Studio validation

The approved migration uses the pinned Rune checkout and adds its `rune-kit` crate.
No vendor source or analyzer contracts changed. The custom graph renderer consumes
shared theme colors; standard controls and the shell use Kit directly.

## Evidence

- `fixture-{dark,light}.png`: selected fixture method and Search drawer at 1420 × 900.
- `fixture-{dark,light}-minimum.png`: same state at 1050 × 680.
- `real-{dark,light}.png` and `real-{dark,light}-minimum.png`: actual cached PacificFate
  graph (20,494 symbols; 75,881 relationships), searched and focused on
  `ResolveNoDemandTerrainMode`, depth 2, Method grouping. The bounded neighborhood
  contains five displayed symbols; all indexed symbols remain searchable.
- Matching `.tree.json` files record native rendered geometry/accessibility metadata.
- `reference-graph-studio.png` is the third displayed generated concept selected by
  the user. `comparison*.png` support the project-root `design-qa.md` review.
- `inspect-fixture.json` and `explore.json` are reproducible native input scripts.

The source graph was copied into `/private/tmp/codegraph-graph-studio-cache` before
capture. The user's original cache was not modified by capture scripts. No analysis
refresh was run against the real project for this UI validation.

## Reproduction

Build with `cargo build --locked --bin codegraph`, then run:

```sh
./target/debug/codegraph --scene fixture \
  --script evidence/ui/graph-studio/inspect-fixture.json \
  --snap /tmp/graph-studio-fixture.png --theme dark --size 1420x900 --scale 1 --at 3

CODEGRAPH_DATA_DIR=/path/to/copy-of-cache ./target/debug/codegraph --scene cached-focus \
  --script evidence/ui/graph-studio/explore.json \
  --snap /tmp/graph-studio-real.png --theme dark --size 1420x900 --scale 1 --at 12
```

Use `--theme light` and/or `--size 1050x680` for the other matrix cells. Native Metal
access is required. Real-project scripts include time for debounced SQLite search
and Graphviz workers; they expect a cached graph containing the named method.

## Checks

- Full Rust suite, including the compiler-backed ignored regression: see `rust-tests.txt`.
- Clippy across all targets with warnings denied: `clippy.txt`.
- Python tests: `python-tests.txt`.
- CodeScene reviewed changed Rust components and tests at 10.0 with no findings.
  The unchanged upstream icon data has no executable functions to score.
  Repository safeguard passed over 16 eligible modified files.
- Reviewer rechecked pane budgeting, identity header and component boundaries;
  no material findings remain. Sonar is inactive: no `.sonar/config.json` exists.

Runtime tests additionally cover actual build discovery failure, appearance changes,
Reduce Motion, disabled controls, long identifiers, legacy session restoration,
independent UI preference persistence and camera preservation during live pane drags.
Physical trackpad/window-shell and VoiceOver interaction remain unverified.
