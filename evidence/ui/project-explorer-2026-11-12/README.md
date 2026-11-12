# Synchronized project explorer

TASK-45 adds the filesystem tree below repository/build controls. The captures
use the production Rune view, layout and renderer with real temporary directories
and an explicit synthetic graph; no analyzer or saved user workspace is used.
`Shared.cs` contains classes from two analyzed build identities. Hidden and
non-source files remain visible alongside source files.

## Visual evidence

- [Dark, 1420×900](dark-1420.png)
- [Light, supported minimum 1050×680](light-1050.png)

Both images were inspected after the file-framing correction: class cards stay
below the filter panel, and explorer controls and selected paths remain visible.

The optional capture branch requires native Metal access. The sandbox could not
see a Metal device; the same isolated test succeeded outside the sandbox.

## Regression coverage

`src/ui_tests/project_explorer.rs` exercises the production event loop with a
real filesystem. It covers single-click isolation, graph keyboard focus, a
300-file virtualized listing, lazy ancestor reveal, multi-file graph selection,
re-clicking the same graph node after selecting an unrelated explorer row,
double-click class navigation across builds, camera framing, Back/Forward,
unanalyzed and filtered files, closed-sidebar synchronization, root replacement,
canonical analyzer paths with logical selected-root aliases, selected symlink
roots without following child directory links, supported window sizes, and resizing between 1420×900 and 1050×680.

The runtime tests first caught explorer rows taking focus despite their
focus-preserving press flag. Rune's explicit `focusable(true)` wins over that
flag; select-only explorer rows now preserve graph focus. Inspection of the
initial minimum-size image also caught file-class cards underneath graph filters;
the layout regression checks the card tops against the filter panel bounds.

## Reproduction

```sh
cargo test --locked -p codecallnavigator --bin codecallnavigator ui_tests::project_explorer -- --nocapture
CODECALLNAVIGATOR_EXPLORER_CAPTURES=evidence/ui/project-explorer-2026-11-12 \
  cargo test --locked -p codecallnavigator --bin codecallnavigator \
  ui_tests::project_explorer::explorer_layouts_and_optional_native_captures -- --nocapture
```

The capture environment variable is optional; the ordinary test suite does not
require GPU access or write images. Source paths and the workspace title vary
with the temporary directory name.

## Results

- [Seven real-runtime regressions passed](runtime-tests.txt), including resize and filter occlusion checks.
- [Two native Metal captures completed](native-captures.txt); both were visually inspected.
- CodeScene review of the new runtime test module returned **10.0**, with no findings.

## Repository validation

- `cargo check --locked -p codecallnavigator --all-targets`: passed.
- `cargo test --locked -p codecallnavigator --all-targets`: 364 passed, 70
  toolchain/benchmark tests ignored by default. These include eight explorer
  model tests and seven real-runtime explorer tests.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`: passed.
- Compiler acceptance with ignored tests enabled passed across all desktop
  integration tests and core/engine targets. The initial run stopped at missing
  private-fixture configuration; remaining targets were then completed in
  batches with the available private checkout. The compiled-output test passed
  separately using freshly exported successful Maven test-compile metadata.
- All three CPU/GPU canvas benchmarks passed in release mode using a
  transactional copy of the saved workspace, including native Metal at scales
  1 and 2. These are throughput checks, not measurements of displayed FPS.
- `python3.13 -m unittest discover -s tests -v`: 106 passed.
- `python3.13 tools/package.py`: passed after the final root-alias correction;
  local ad hoc signed app and ZIP rebuilt in `dist/`.
- Final CodeScene reviews: 10.0 for changed source files; pre-commit safeguard
  passed all 22 eligible files with no regressions. SonarMCP is inactive because
  this project has no `.sonar/config.json`.

The exact debug `--include-ignored` invocation initially failed because its
three ignored canvas benchmarks explicitly require release mode; their proper
release run passed. A process-cleanup test also failed under sandbox process
restrictions and passed outside the sandbox. Compiler acceptance and native
Metal captures were run outside those restrictions. No tests were modified or
weakened to bypass these requirements.

Private source, build metadata and copied saved-workspace contents remain outside
this evidence directory. Detailed command logs are under
`/tmp/codecallnavigator-explorer-*.log`; this record captures their results.

The filesystem fixture publishes canonical compiler source paths while the tree
retains the selected root spelling. The symlink-root regression verifies both
directions of that mapping, including file double-click navigation and preserved
graph focus. The images predate this mapping correction; it does not change their
rendered layout, file labels, or highlighted class cards.
