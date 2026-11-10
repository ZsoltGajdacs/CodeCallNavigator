# Direct-connection emphasis verification

TASK-27 keeps depth-based visibility and applies selection emphasis separately.
The selected anchor and direct incoming/outgoing neighbors stay bright; other
complete cards use 35% opacity. Edges not incident to an anchor, including their
arrowheads, use 8% of their previous alpha and paint before highlighted edges.
A visible selection overrides focus; missing selection falls back to visible
focus roots. With no roots, everything retains normal brightness.

`GraphEmphasis` is a pure, per-render presentation helper shared by cards and
edges. Geometry, relationship colors/dashes, traversal, and persistence are
unchanged. Dim cards and method rows remain interactive.

## Evidence

- `real-dark.png` and `real-light.png`: native Metal captures of the actual
  Recurring-billing graph at depth 2, centered on `fetchPrintTemplates`.
- `dense-dark.png` and `dense-light.png`: the same graph at depth 5.
- Captures use a copied current published SQLite generation; the original user
  cache was not changed and no analyzer/build was run against that project.
- Pure policy tests cover both directions, second hops, links between neighbors,
  self-loops, parallel edges, isolated selections, multiple roots, and fallback.
- Shape equality tests prove edge order, stroke and arrow alpha, and preservation
  of direct, callback, and uncertain styles in both themes.
- Production Rune interaction tests exercise depth expansion, selecting dimmed
  context, restoring normal brightness, class selection, inherited method-row
  opacity, and clicking a method inside a dimmed class.

## Validation

- `rust-tests.txt`: all-target Rust suite, 270 passed and 48 ignored. Ignored
  analyzer/toolchain and benchmark tests were not needed for rendering changes.
- `python-tests.txt`: 50 passed.
- `package.txt`: final macOS application package built successfully at
  `dist/CodeGraph.app` and `dist/CodeGraph.zip`.
- CodeScene reviewed all eight changed Rust files at 10.0 with no findings;
  its repository safeguard passed across eight eligible source files.
- Independent review found no actionable correctness or maintainability issues.
- Production Clippy passed with warnings denied. Strict all-target Clippy hit
  the existing constant runtime assertion in `src/ui_tests/canvas_performance.rs`;
  `clippy-strict.txt` preserves that failure. All-target Clippy passed with only
  `clippy::assertions_on_constants` suppressed on the command line (`clippy.txt`).
  That unrelated benchmark source was not changed.
- Sonar is inactive because this project has no `.sonar/config.json`.
- Physical trackpad and VoiceOver verification were not performed.

## Reproduction

With a copy of a current Recurring-billing cache containing `fetchPrintTemplates`:

```sh
cargo build --locked --bin codegraph
CODEGRAPH_DATA_DIR=/path/to/cache-copy ./target/debug/codegraph \
  --scene cached-focus --script evidence/ui/selection-emphasis/explore.json \
  --snap /tmp/emphasis.png --theme dark --size 1900x1100 --scale 1 --at 12
```

Use `dense.json` for depth 5 and `--theme light` for the light theme. Native Metal
access is required. These scripts use existing cached analysis; no refresh runs.

## Action log

1. Recorded approved plan and acceptance criteria in TASK-27. Backlog task creation
   required its normal `.git` lock outside sandbox access and succeeded on retry.
2. Reproduced the missing dimming in a production depth-expansion test: second-hop
   card was 1.0 rather than 0.35. Stabilized animation before measuring opacity.
3. Extracted shared emphasis policy and connected card/edge rendering. CodeScene
   findings prompted anchor-inclusive bright membership and a smaller edge helper.
4. Verified focused policy/edge tests and independent source review. Extended UI
   tests to fit the graph after camera changes and move the pointer away so source
   hover previews do not intercept subsequent gesture targets. Both UI tests pass.
5. Captured current cached data after an older fixture cache proved incompatible.
   Native GPU capture required sandbox escalation. An asynchronous normal-startup
   capture did not finish boot within virtual time and was discarded; synchronous
   cached tooling captures provide the visual evidence.
6. Ran complete default Rust and Python suites, CodeScene reviews and safeguard,
   Clippy, and packaging. Strict Clippy's unrelated existing lint is recorded above.
7. Left implementation changes uncommitted; no push or pull request was requested.
