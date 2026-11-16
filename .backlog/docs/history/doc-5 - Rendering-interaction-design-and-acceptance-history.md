---
id: doc-5
title: Rendering interaction design and acceptance history
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:07:00Z'
---
# Rendering interaction design and acceptance history


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-architecture-readme-md"></a>

## evidence/architecture/README.md

Source: [evidence/architecture/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/architecture/README.md). Historical record; retained rationale, methods and limitations.

### Architectural layer verification

Verified on 2026-11-09 on Apple Silicon macOS.

#### Checks

- `cargo test --all-targets --offline --quiet`: 185 passed, 39 ignored. The ignored
  cases need real toolchains or private-project inputs and were exercised separately.
- Real-toolchain checks: 37 of the ignored cases passed across Roslyn, Java source
  inventory, Maven/Gradle, packaged analyzer resources and TypeScript workers.
- Two private-project cases could not run because `CODEGRAPH_COMPILED_METADATA`
  and/or `CODEGRAPH_RECURRING_BILLING` were unset: the archived successful-build
  metadata check and the recurring-billing generated-declaration check. Invoking
  them without those inputs fails at their explicit environment preconditions;
  this is not reported as a pass.
- `python3.13 -m unittest discover -s tests -v`: 48 passed.
- `node --test tools/typescript/*.test.js`: 31 passed using the installed compiler.
- `cargo clippy --all-targets --offline --no-deps -- -D warnings`: passed.
- `cargo fmt --package codegraph --check`: passed. Use the package scope to avoid
  formatting dependency workspaces.
- CodeScene source reviews prompted focused extraction of matching, grouping,
  geometry, controls and test fixtures. The final safeguard passed with no
  regressions across 56 eligible files (74 modified files at the check).
- SonarMCP is inactive because this repository has no `.sonar/config.json`.

The new configuration tests cover first-match semantics, conjunctive matching,
attribute suffix normalization, test precedence, inheritance/conversion failures,
native-file priority, nested package paths, dotted Java build directories and
canonical repository paths. Real analyzer acceptance covers type-only declarations,
SOURCE-retained Java annotations, C# linked sources compiled in separate projects,
TypeScript decorators and transitive inheritance, stable graph identities and cache
roundtrips. UI/lifecycle tests cover recovery after invalid edits, selection/tab
persistence, obsolete jobs, filtered relationship counts and control overlap.

#### Packaging

The normal package provenance check refused the existing local Rune checkout:
its 1,171-file formatting diff predates this task (sample modification timestamps
were 16:11–16:13 local time, before implementation began). The checkout was
preserved. A clean-copy formatter comparison confirmed the file set but not an
exact byte match, so no restore was performed. A backup of the local diff was
kept outside the repository.

Packaging was instead run from an isolated root under `/private/tmp` with the
current CodeGraph manifest/lock/source, the clean pinned Rune revision, the exact
checked-in CallAtlas patch and the current analyzer artifacts. `tools/package.py`
ran its unchanged provenance gate, release build, dependency/license bundling,
ad-hoc signing and strict signature verification successfully. The resulting app
and ZIP are under `dist/`. This does not claim the unmodified packaging command
can pass against the existing dirty vendor checkout.

The packaged binary then analyzed the Maven standalone fixture with `--layers
spring`, using bundled analyzer resources, and published a graph successfully.
Its classification summary is [packaged-cli.txt](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/architecture/packaged-cli.txt).

#### Visual evidence

[Four native Metal captures](<doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-architectural-layers-readme-md>) cover light/dark
appearance at 1420×900 and 1050×680. They use explicit synthetic graph data through
production Graphviz geometry. They do not substitute for the real compiler tests.
Physical trackpad and VoiceOver validation were not performed.


<a id="source-evidence-ui-aligned-levels-readme-md"></a>

## evidence/ui/aligned-levels/README.md

Source: [evidence/ui/aligned-levels/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/aligned-levels/README.md). Historical record; retained rationale, methods and limitations.

### Uniform focused graph levels

TASK-29 follows TASK-23. Focused projection resolves one semantic presentation
level before aggregating and traversing relationships. Class, namespace/package,
folder, project, layer and method selections therefore align connected boxes,
independently of manual overview expansion. Folder connections use the selected
build-relative directory depth, with shallower and project-root fallbacks.

The `aligned-classes` scene deliberately expands only the service folder and
marks a caller class as expanded while the other folders remain collapsed. Both
native Metal captures were inspected: OrderController, OrderService and
OrderRepository are class boxes, connected by two edges. The unrelated Metrics
class is absent. The selected service retains its scrollable method list. Dark
and light themes show readable labels, directed edges and the selected anchor.

Reproduce after `cargo build --offline`:

```sh
./target/debug/codegraph --scene aligned-classes --snap evidence/ui/aligned-levels/classes-dark.png --theme dark --size 1420x900 --scale 1 --at 2
./target/debug/codegraph --scene aligned-classes --snap evidence/ui/aligned-levels/classes-light.png --theme light --size 1420x900 --scale 1 --at 2
```

Native capture requires Metal device access; the restricted sandbox lacks that
device, so captures were run with the normal native-device permissions.

Verification:

- The initial class/folder regression failed before the fix and passes afterward.
- `cargo test --all-targets --offline`: 301 passed, 50 ignored, no failures.
  See `rust-tests.log`. The ignored tests require explicit toolchains, fixtures or
  benchmark configuration; they are not counted as passes.
- Projection tests cover complete package/global namespace identities, folder
  nesting and root fallbacks, connected projects, layer-qualified identities and
  active tabs, filters, method focus, explicit child focus, cycles and pagination.
- Native interaction tests cover folder/package/class drill-down, persisted child
  focus, Back/Forward, unchanged overview expansion, depth and camera behavior.
- Production Graphviz layout checks and `cargo build --offline` passed.
- Scoped Rust formatting and `git diff --check` passed.
- Strict Clippy reports existing `needless_range_loop` in `src/edges/viewport.rs`
  and `assertions_on_constants` in the canvas benchmark tests. These files match
  HEAD and were left unchanged. Clippy passes with only those two lint classes
  exempted; see `clippy-exemptions.log`. This is not a clean strict-Clippy result.
- CodeScene reviewed all 11 changed source files at 10.0 with no remaining
  findings. Test fixture extraction and a combined navigation assertion resolved
  the initial test maintainability findings. The safeguard passed all 11 eligible
  files with no regressions.
- Independent production review found no blockers; requested project and layer
  coverage was added. Independent final testing confirmed the full Rust suite.
- Sonar is inactive because this checkout has no `.sonar/config.json`.
- Dependency source provenance verification passed. No upstream sources changed.
- Both packaged Java resource smoke tests passed with real Maven/Gradle/JDK
  artifacts, including rejection of a missing packaged extension; see
  `packaged-java.log`. The first restricted attempt could not access Gradle's
  cache lock; rerunning with normal toolchain cache access passed.
- Python acceptance/packaging unit checks passed: 50 tests via
  `python3.13 -m unittest discover -s tests -p 'test_*.py'`.

The development binary was rebuilt. No release bundle was created or installed,
and no commit or push was performed.

Single-click keeps a group at its own level. Double-click now persists explicit
child-group focus rather than modifying overview expansion. Existing group focus
sessions consequently restore at the group's own level; existing method focus
and class-method tree variants remain supported. No analyzer graph migration is
required. Physical trackpad and VoiceOver checks were not performed.


<a id="source-evidence-ui-architectural-layers-readme-md"></a>

## evidence/ui/architectural-layers/README.md

Source: [evidence/ui/architectural-layers/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/architectural-layers/README.md). Historical record; retained rationale, methods and limitations.

### Architectural layer UI evidence

The `architectural-layers` scene contains six explicit synthetic methods. It uses
production projection and Graphviz geometry, with Controller, Service, Repository
and Tests bands, a Utilities side lane, and the original caller-to-callee arrows
(including a cross-layer cycle). It does not claim analyzer acceptance for a real
repository.

The four screenshots cover dark/light appearance at 1420×900 and the minimum
1050×680 window. The capture script closes the side panes and fits the graph.
Visual inspection confirmed visible controls, ordered bands, side-lane placement,
unchanged edge categories and cycle styling. Pan, zoom and method focus retain
existing behavior; physical trackpad and VoiceOver validation are not claimed.

Reproduce after `cargo build --locked --bin codegraph`:

```sh
./target/debug/codegraph --scene architectural-layers \
  --script evidence/ui/architectural-layers/explore.json \
  --snap /tmp/codegraph-layers.png --theme dark --size 1420x900 --scale 1 --at 3
```

Use `--theme light` and/or `--size 1050x680` for the other matrix entries. Native
Metal device access is required for screenshots.

Runtime interaction tests additionally verify that presets and tabs work, history
restores the tab independently of the saved preset, and controls remain unclipped
with both side panes open at the minimum window size. An immediate post-transition
check prevents the graph canvas/filter overlay from covering freshly inserted tab
controls. Projection regressions verify real Graphviz ordering, independent
parallel flows, support lanes, cycles/reverse arrows, tab omission accounting,
filtered-edge counts and dotted Java build-root source paths. Workspace regressions
exercise cached-content reclassification, invalid-configuration recovery, disabling
layers and rejecting obsolete graph epochs.


<a id="source-evidence-ui-build-picker-2026-11-12-readme-md"></a>

## evidence/ui/build-picker-2026-11-12/README.md

Source: [evidence/ui/build-picker-2026-11-12/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/build-picker-2026-11-12/README.md). Historical record; retained rationale, methods and limitations.

### Anchored build picker — 2026-11-12

Find builds now opens a modal beside its button and rescans in the background.
The checkbox list scrolls vertically, selection applies immediately, and the
button's hover help lists selected compact build names. The sidebar no longer
reserves inline build-list space.

Picker rendering, desktop discovery lifecycle and shared metadata naming have
separate modules. BuildTarget fields, identity and saved-session schemas remain
unchanged. Rescans retain both selected and deselected known identities; new
identities use discovery defaults. Changing folders invalidates old targets and
stale discovery results. Discovery error cleanup preserves unrelated warnings.

Project names come from literal Maven, Gradle or unconditional C# metadata, with
folder/file-stem fallbacks. XML uses roxmltree already present in Cargo.lock.
Gradle scripts containing triple-quoted strings conservatively use folder names;
no build scripts or computed names are evaluated for display.

#### Visual evidence

Native Rune GPU rendering, inspected in both appearances:

- [Dark, 1420×900](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/build-picker-2026-11-12/builds-dark-1420.png)
- [Light, 1050×680](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/build-picker-2026-11-12/builds-light-1050.png)

The captures show the list after scrolling. The header remains fixed, and the
panel stays aligned with the button. Runtime tests additionally resize to
600×360 and verify the panel and scroll viewport remain usable.

Reproduce captures on macOS with native Metal access:

```sh
CODECALLNAVIGATOR_BUILD_CAPTURES=evidence/ui/build-picker-2026-11-12 cargo test --locked -p codecallnavigator --bin codecallnavigator ui_tests::build_picker::scrollable_modal_fits_windows_and_leaves_explorer_space -- --nocapture
```

#### Verification

Passed:

- `cargo check --locked -p codecallnavigator --all-targets`
- `cargo check --locked -p codecallnavigator-core --all-targets`
- `cargo test --locked -p codecallnavigator --all-targets` — 373 passed before the final additional discovery-error reset regression; that regression and the complete workspace suite passed afterward.
- `cargo test --locked -p codecallnavigator-core --all-targets` — 60 passed.
- `cargo test --workspace --all-targets` — final run: 494 passed, 84 ignored.
- Focused discovery lifecycle tests: 4 passed; picker Runtime tests: 8 passed.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`
- `python3.13 -m unittest discover -s tests -v` — 106 passed.
- `python3.13 tools/package.py` — release desktop/engine built, local app/ZIP packaged and signature verified. Packaging required sandbox escalation for its temporary Git index; captures required native Metal access.
- CodeScene file reviews: all modified source files scored 10.0. Final pre-commit safeguard passed, checked 15 eligible files, no issues found.
- Independent planning, exploration, implementation, static review and Runtime testing roles completed; review findings fixed and rechecked.
- `git diff --check`

Required full ignored-test gate did not pass:

- `cargo test --workspace --all-targets -- --include-ignored` failed in the desktop test binary. Three ignored canvas benchmarks require release mode (two also require an explicit copied profiling workspace); the engine forced-cancellation test reported a surviving detached child under the sandbox. It stopped before later test binaries.
- An expanded `--no-fail-fast` attempt passed 214 desktop tests but retained those four failures. It then reported five ASP.NET acceptance failures and stalled in the remaining test's `dotnet restore fixtures/mediatr-dispatch/MediatRDispatch.sln`. The expanded run and its restore process were terminated; it is incomplete, and no full analyzer-acceptance success is claimed.
- Sonar is inactive because this workspace has no `.sonar/config.json`.

TASK-46 retains the full-validation limitation. Changes are local; no commit,
push or pull request was created.

#### Subsequent full verification

The earlier full-suite limitations above were resolved during TASK-47. The final
native full verification on 2026-11-12 passed all 578 release workspace tests,
including ignored compiler acceptance and all canvas benchmarks, plus packaging
and CodeScene. No waivers were needed. See
[the complete verification record](<doc-10 - Testing-migration-and-verification-operations.md#source-evidence-verification-2026-11-12-readme-md>).


<a id="source-evidence-ui-centered-traversal-readme-md"></a>

## evidence/ui/centered-traversal/README.md

Source: [evidence/ui/centered-traversal/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/centered-traversal/README.md). Historical record; retained rationale, methods and limitations.

### Centered traversal verification

TASK-23 replaces selection dimming with a centered caller/callee neighborhood and
progressive group → class → method exploration. Class boxes list filtered,
scrollable method signatures. Opening a class method graph preserves its internal
call hierarchy with external context above and below.

The six PNGs use explicit synthetic traversal scenes, production projection and
Graphviz, and native Metal rendering. Dark/light normal-size class and method
views were inspected. Minimum-size class views use the explicit Fit action; Fit
reserves space beneath the floating filters. Focus itself preserves zoom and
centers its anchor, so large trees can require panning or Fit.

Reproduce with the built development binary:

```sh
./target/debug/codegraph --scene class-tree --snap /tmp/class-tree.png --theme dark --size 1420x900 --scale 1 --at 2
./target/debug/codegraph --scene method-tree --snap /tmp/method-tree.png --theme light --size 1420x900 --scale 1 --at 2
```

Use the class scene at 1050x680 and activate Fit for minimum-window checks.
Screenshots require native Metal device access. Physical trackpad and VoiceOver
verification are not claimed.

Behavioral coverage lives in the native traversal and graph-fit suites and the
projection focus tests. It covers click timing, both directions, class box hops,
method rows and long-list scrolling, internal class ordering, overload identities,
cycles, pinned pagination at the canvas limit, layer tabs, history, legacy focus,
filter invalidation, camera input during layout, and wrapped filter overlays.

Validation logs accompany these captures. Python packaging provenance tests and
real packaged Java resource smoke tests pass. A fresh app bundle cannot be built:
the existing ignored vendor/rune checkout has local changes rejected by the
packager's source-provenance guard. Those dependency changes were preserved.


<a id="source-evidence-ui-cli-backend-2026-11-12-readme-md"></a>

## evidence/ui/cli-backend-2026-11-12/README.md

Source: [evidence/ui/cli-backend-2026-11-12/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/cli-backend-2026-11-12/README.md). Historical record; retained rationale, methods and limitations.

### Desktop CLI adoption acceptance

The production desktop obtains graph information through the public CLI. See
[ownership and lifecycle](<../architecture/doc-11 - Desktop-query-architecture-and-lifecycle.md>).

#### Native workflow

`ui_tests::cli_backend::actual_cli_keeps_graph_context_across_edited_root_and_drives_search_source_flow`
analyzes a real TypeScript fixture with the actual engine process, then drives
Double Shift search, method activation, relationships, explorer/source and
function-flow navigation. Editing the folder field preserves the loaded graph's
original query/source context. The test also returns from flow to the call graph.

The native Metal test passed. The [dark](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/cli-backend-2026-11-12/dark-flow.png) and
[light](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/cli-backend-2026-11-12/light-flow.png) captures were inspected after waiting for frame settling:
source highlighting, branch chart, toolbar and panels render without overlap.
Separate focused tests cover layer/filter selection parity, cancellation,
obsolete search activation, corrupt saved navigation and transport failures.

#### Query responsiveness

[Raw measurements](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/cli-backend-2026-11-12/cli-query-latency.json) use the existing 113,628,316-byte
Recurring-billing artifact, without analysis or changes to the project. Three
samples per operation include CLI execution, pipe transfer and Python JSON
decoding; filesystem caches are uncontrolled. The first session status includes
initial loading. These measurements precede the final analyzer-version metadata
preservation fix; they do not measure native frame time or input-to-paint latency.

| Operation | One-process-per-request median | Persistent-session median |
| --- | ---: | ---: |
| Search | 1818 ms | 239 ms |
| Bounded selection | 1814 ms | 243 ms |
| Complete render selection | 1965 ms | 297 ms |
| Initial full snapshot | 3360 ms | 1816 ms |

The desktop uses the cancellable persistent CLI session because loading this
artifact for each interactive request is excessive. The initial snapshot remains
large (about 102 MB), is loaded asynchronously and is retained for rendering.
[Earlier render-only samples](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/cli-backend-2026-11-12/cli-render-latency.json) retain the pre-bounded
diagnostic payload for comparison; the table uses the newer bounded responses.

Release canvas benchmarks on Apple M4 Pro / Metal passed in the final full run.
Dense 300-node/6,000-edge CPU frame preparation medians were 2.85–3.56 ms across
the tested zooms and scales. The isolated saved workspace (19 visible nodes,
23 edges) measured 4.14–4.19 ms median preparation. GPU-complete offscreen frame
medians were 8.88/12.54 ms for dense scale 1/2 and 7.92/9.66 ms for the saved
workspace. These synchronous offscreen measurements are not displayed FPS.

#### Saved-workspace compatibility

Successful CLI snapshots populate a write-only derived SQLite presentation cache
for isolated profiling and rollback. Production queries never read its graph.
Real-process regressions cover first-run retention, portable identical artifacts
in different physical checkouts, malformed session recovery, cancellation and
fresh analyzer provenance. Python SQLite/WAL tests require a coherent saved CLI
generation/cache marker before profiling and copy an immutable generation.

Full runtime verification evidence is recorded under
`target/verification/task49-full-final-20261112/`: 524 ordinary Rust tests,
611 release tests including ignored acceptance and benchmarks, 160 Python project
tests, 26 Python tool tests, 112 TypeScript tests, storage targets, packaging and
strict signature/packaged-engine checks passed. Its final CodeScene source review
found complexity 9 in the desktop argument parser; the regression safeguard
passed 79/79 eligible files. A behavior-preserving option-handler extraction
resolved that finding. Four parser tests and desktop all-target Clippy passed
afterward, and independent review confirmed equivalent argument behavior.

The exact package, signature, packaged engine and all-source CodeScene stages
were rerun under `target/verification/task49-post-review-20261112/` against the
final parser source. Original results remain intact; the final Backlog task
records the combined outcome. The earlier `task49-full-20261112` run was deliberately interrupted
when review identified a missing retained-cache-file recovery case, then cleanup
completed and a process inventory confirmed no remaining verification/test job.
Profiling uses a copied saved workspace, never the live application store.


<a id="source-evidence-ui-function-flowcharts-readme-md"></a>

## evidence/ui/function-flowcharts/README.md

Source: [evidence/ui/function-flowcharts/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/function-flowcharts/README.md). Historical record; retained rationale, methods and limitations.

### Function flowchart UI evidence

Four native Metal screenshots show the dedicated chart/source workspace in dark
and light appearance at 1420×900 and 1050×680. The selected call step highlights its
cached source and exposes a compiler-attributed target button.

The explicit `function-flow` scene uses a two-function synthetic snapshot with the
production builder and Graphviz layout. It does not substitute for real compiler
acceptance tests. Source paths intentionally do not exist on disk, proving that
this view reads the snapshot. The captures were visually inspected for source
alignment, readable branches, accessible controls and minimum-window bounds.

Reproduce after building the development binary:

```sh
./target/debug/codegraph --scene function-flow \
  --script evidence/ui/function-flowcharts/explore.json \
  --snap /tmp/function-flow.png --theme dark --size 1420x900 --scale 1 --at 2
```

Use `--theme light` and `--size 1050x680` for the remaining matrix entries.
Native Metal access is required. Physical trackpad and VoiceOver validation are
not claimed.

Runtime interaction tests cover the inspector action, actual target-button
navigation, source-row highlight movement, flow history/camera restoration,
unchanged call-graph navigation, per-visit source scrolling (including A → B → A),
generation invalidation, old-cache messaging and
minimum-window geometry. Tests found and fixed the initial fast-load fit race and
newly inserted footer controls being clipped by animated sizing.


<a id="source-evidence-ui-graph-studio-readme-md"></a>

## evidence/ui/graph-studio/README.md

Source: [evidence/ui/graph-studio/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/graph-studio/README.md). Historical record; retained rationale, methods and limitations.

### Rune Kit Graph Studio validation

The approved migration uses the pinned Rune checkout and adds its `rune-kit` crate.
No vendor source or analyzer contracts changed. The custom graph renderer consumes
shared theme colors; standard controls and the shell use Kit directly.

#### Evidence

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

#### Reproduction

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

#### Checks

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


<a id="source-evidence-ui-project-explorer-2026-11-12-readme-md"></a>

## evidence/ui/project-explorer-2026-11-12/README.md

Source: [evidence/ui/project-explorer-2026-11-12/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/project-explorer-2026-11-12/README.md). Historical record; retained rationale, methods and limitations.

### Synchronized project explorer

TASK-45 adds the filesystem tree below repository/build controls. The captures
use the production Rune view, layout and renderer with real temporary directories
and an explicit synthetic graph; no analyzer or saved user workspace is used.
`Shared.cs` contains classes from two analyzed build identities. Hidden and
non-source files remain visible alongside source files.

#### Visual evidence

- [Dark, 1420×900](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/project-explorer-2026-11-12/dark-1420.png)
- [Light, supported minimum 1050×680](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/project-explorer-2026-11-12/light-1050.png)

Both images were inspected after the file-framing correction: class cards stay
below the filter panel, and explorer controls and selected paths remain visible.

The optional capture branch requires native Metal access. The sandbox could not
see a Metal device; the same isolated test succeeded outside the sandbox.

#### Regression coverage

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

#### Reproduction

```sh
cargo test --locked -p codecallnavigator --bin codecallnavigator ui_tests::project_explorer -- --nocapture
CODECALLNAVIGATOR_EXPLORER_CAPTURES=evidence/ui/project-explorer-2026-11-12 \
  cargo test --locked -p codecallnavigator --bin codecallnavigator \
  ui_tests::project_explorer::explorer_layouts_and_optional_native_captures -- --nocapture
```

The capture environment variable is optional; the ordinary test suite does not
require GPU access or write images. Source paths and the workspace title vary
with the temporary directory name.

#### Results

- [Seven real-runtime regressions passed](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/project-explorer-2026-11-12/runtime-tests.txt), including resize and filter occlusion checks.
- [Two native Metal captures completed](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/project-explorer-2026-11-12/native-captures.txt); both were visually inspected.
- CodeScene review of the new runtime test module returned **10.0**, with no findings.

#### Repository validation

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


<a id="source-evidence-ui-relationship-filters-2026-11-11-readme-md"></a>

## evidence/ui/relationship-filters-2026-11-11/README.md

Source: [evidence/ui/relationship-filters-2026-11-11/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/relationship-filters-2026-11-11/README.md). Historical record; retained rationale, methods and limitations.

### Relationship meaning and target resolution

TASK-42 implements the approved independent semantic dimensions. The filter implementation preserves raw analyzer relationship facts and semantic
graph content; the approved follow-up below improves canonical input provenance. Classifications are derived
once for each indexed generation, and shared by filtering, aggregation, drawing
and details. Session filter version 2 resets legacy relationship choices once while
preserving code flags; current empty selections persist.

The panel occupies layout space above the drawable canvas. An overlay prototype
caused underlying graph hover previews to obscure filter controls; normal flow
removes that overlap and keeps camera fitting based on the actual canvas size.
Unused overlay-inset state, projected raw fields, and duplicate string-based
classification helpers were removed. Filter descriptors, session migration and
classification rules each have a focused owning module.

#### Visual checks

Native Metal captures were inspected with current production rendering:

- [Dark, 1420×900](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/relationship-filters-2026-11-11/dark-1420.png)
- [Light, supported minimum 1050×680](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/relationship-filters-2026-11-11/light-1050.png)

Full labels, grouping and wrapping are readable. The supported minimum is declared
in `src/main.rs`; smaller headless captures do not represent supported window sizes.
CPU UI tests cover both themes, panes open/closed, history, migration, empty
selections, accessible help, fit below the panel, and normal graph interaction.

#### Verification

- Baseline migration regression failed as expected before implementation: version
  was absent. Baseline grouped UI regression failed on the missing Code heading.
- Core tests: 55 passed.
- Focused graph/projection/filter tests pass, including evidence precedence,
  unknown analyzer values, separate reference/virtual selection, cycle/path/count
  consistency, and equal raw call/possible edges with different semantic meaning.
- Desktop check, all nonignored target tests, and workspace Clippy with warnings
  denied pass. All 94 nonignored UI tests and 16 edge/render tests pass.
- `python3.13 tools/package.py` built and ad hoc signed the desktop app and engine.
- Independent core and desktop review found no correctness blockers; requested
  evidence-precedence and complete kind coverage was added.
- CodeScene reviews report 10.0 with no findings on modified source after focused
  test/module extraction. Sonar is inactive: `.sonar/config.json` is absent.

#### Initial full-suite failures and targeted reruns

The exact debug include-ignored aggregate is not green. The outside-sandbox
`cargo test --workspace --all-targets --no-fail-fast -- --include-ignored --test-threads=1`
run reported 526 passed and 7 failed. Those failures comprise three benchmarks
that explicitly require release mode, one source-popup fixture placement already
corrected and covered by the final passing desktop run, two private acceptance
cases requiring explicit local fixture paths, and one TypeScript alias discrepancy.
The initial sandbox attempt also encountered restricted dependency restores and
process-group cleanup; the cancellation check passed with the required access.

- The private duplicate-declarations test passes against an existing isolated
  temporary reactor copy (152.92 seconds), without touching the original checkout.
- Before the approved follow-up, the TypeScript source-alias artifact-equality
  test failed with a freshly rebuilt engine. It also fails with the pre-change packaged engine
  `CodeCallNavigator.previous-20261111-120921.app` at the same assertion. This is a
  point-in-time packaged baseline comparison, not an exact-HEAD baseline proof.
  Differences concern dependency/missing-probe inputs and derived diagnostics;
  these analyzer/artifact paths were not changed for TASK-42.
- All public Java, C# and TypeScript compiler acceptance cases other than that
  alias discrepancy passed, including the new method-reference, virtual-dispatch,
  Java-lambda and React-framework classification checks.

- Compiled-private metadata acceptance passes with the existing isolated reactor
  metadata (69.25 seconds).
- All three release benchmarks pass with Metal and a copied saved cache (9.49
  seconds total). Dense CPU frame preparation p95 is 3.250–4.101 ms; saved CPU
  preparation p95 is 5.421 ms at scale 1 and 5.565 ms at scale 2; GPU-complete
  saved frame p95 is 11.290 ms at scale 2. The original workspace was untouched.

At that checkpoint, the remaining full-suite failure was the independently
reproduced source-alias artifact discrepancy. TASK-42 remained In Progress for
validation sign-off. The filter implementation was subsequently committed as
`31387fb`; the approved follow-up below resolves that discrepancy.

Detailed execution logs remain in `/tmp/codegraph-filter-full-unsandboxed.log`,
`/tmp/codegraph-filter-private-rerun.log`,
`/tmp/codegraph-filter-private-compiled-rerun.log`,
`/tmp/codegraph-filter-release-benchmarks.log`,
`/tmp/codegraph-filter-source-path-retry.log`, and
`/tmp/codegraph-filter-source-path-baseline.log`.


#### Investigation after commit 31387fb

The filter implementation was committed locally as `31387fb` on
`codex/standalone-graph-engine`; nothing was pushed.

A retained reproduction ran the same fixture four times: alias spelling,
canonical spelling, canonical repeat, and alias repeat. Results:

- The complete artifact structures are equal after removing only `graph.analysis`.
  All declarations, identities, relationships, evidence, source flows,
  architecture, boundaries and diagnostics match.
- Each spelling produces byte-identical artifacts when repeated. This is not
  nondeterministic graph output.
- The test rewrites the actual `tsconfig.json` source-path strings between
  spellings. Its observed configuration content hash therefore correctly differs.
  Comparing the whole artifact across these different input bytes is invalid.
- Canonical spelling retains ten additional portable missing-file observations.
  The adapter normalizes paths with `realpathSync` only when the entire path exists;
  absent probes below symlinked directories retain alias prefixes. Logical-root
  mapping cannot map those prefixes, and records them as `unmapped_input`.
  The alias run has 262 unmapped observations versus 251 for canonical spelling.
  Both independently verify as `unverifiable` with reason `unmapped_input`,
  so the verifier does not claim complete input verification.

The appropriate regression contract is semantic graph equality across source
spellings, byte-for-byte determinism across identical repeated inputs, and
independent validation of captured provenance/configuration hashes. Input hashes
must not be removed or normalized away to make the existing assertion pass.
Missing-path alias normalization is a separate analyzer/provenance concern.

Independent investigation review confirmed these results. At that stage TASK-42
had no unfinished filter implementation; only validation/sign-off was pending.
It remained In Progress until the incorrect test expectation was corrected or
explicitly accepted. The investigation itself changed no analyzer or test code;
the subsequently approved implementation is described below.

The reproducible driver is `/tmp/codegraph-alias-investigation.py`; its retained
fixture path is recorded in `/tmp/codegraph-alias-investigation.log`. Four artifact
snapshots, exact config spellings, raw metadata and per-spelling verification
reports remain in that temporary fixture directory. No private source payloads
were added to the repository.


#### Approved follow-up fixes

The user approved both recommended fixes after the investigation.

- `02010eb` normalizes missing TypeScript input paths through the nearest existing
  physical ancestor. Filesystem mappings are not cached; only missing-path errors
  are retried, and other filesystem errors propagate.
- Original compiler probe arguments and read closures remain intact. A separate
  observation of each original-to-physical input mapping detects symlink
  retargeting even when both destinations still lack the probed file.
- Adapter provenance version is now 6. No wire-format or relationship-inference
  change is required; rebuilt engine and desktop packages include the new scripts.
- The real-engine source-alias acceptance test compares full semantic artifact
  structures across spellings, verifies exact BLAKE3 hashes of actual configuration
  bytes, compares every other provenance field/input, and checks full-byte repeats
  separately for each unchanged spelling. The comparison orders cloned input rows
  by path after checking the configuration hash, because production JSON ordering
  includes that deliberately changed hash. Production artifact ordering is intact.

The eight real-filesystem/compiler path regressions had five expected baseline
failures before the fix and all pass afterward. The strengthened Rust acceptance
first reproduced missing portable input coverage, then passed after normalization.
All 112 JavaScript tests pass, including retained worker/source/probe integrity and
native binding tests. Independent review found no actionable issues; all six
changed source files score CodeScene 10.0 and the safeguard passes. Cargo check,
strict workspace Clippy, and rebuilt/signed packaging pass.

The full release desktop target passes all 188 tests, including all three CPU/GPU
benchmarks. A parallel debug run during packaging/JS load hit eight existing
2-second flow readiness timeouts. The serial debug all-target repeat passes
335 tests with 70 toolchain/performance cases ignored, without changing those
readiness tests. Complete release workspace validation passes as recorded below.


The alias-dependent missing-input gap is resolved. External dependencies outside
all declared logical roots still produce explicit `unmapped_input` limitations;
the fix preserves those limitations rather than claiming complete verification.
The strengthened acceptance test requires equivalent limitation counts across
spellings as well as equivalent normalized inputs.


#### Final sign-off

TASK-42 is complete. The corrected source-alias acceptance no longer conflates
changed configuration provenance with semantic graph equality, and missing input
probes retain equivalent physical/logical coverage across source spellings.

- Full release workspace: **533 passed, 0 failed, 0 ignored**, across 46 outer
  targets (excluding a duplicate result emitted by a nested subprocess test).
- All compiler fixtures, both private reactor/compiled-metadata checks, the
  strengthened source-alias acceptance, process/protocol tests, and all three
  CPU/GPU benchmarks pass in that single supported-profile run.
- Serial debug desktop all-targets: **335 passed, 70 ignored**. Those ignored
  cases are covered by the full release run.
- JavaScript compiler/adapter suite: **112 passed**, including all eight focused
  filesystem/compiler path tests.
- Cargo check, strict workspace Clippy, scoped formatting, CodeScene reviews and
  safeguard, independent review, and rebuilt/signed packaging pass. Sonar remains
  intentionally inactive because the project has no `.sonar/config.json`.

Full workspace command:

```sh
cargo test --release --workspace --all-targets -- --include-ignored --test-threads=1
```

The run uses the actual toolchains, a copied profiling cache, explicit isolated
private reactor and compiled-metadata paths, and the freshly built release engine.
The private reactor's frontend dependency metadata retries caused a long delay
but its complete Maven build succeeded; no frontend-skip profile was used.

Final logs: `/tmp/codegraph-alias-release-full.log`,
`/tmp/codegraph-alias-regular-serial.log`, `/tmp/codegraph-alias-all-js.log`,
`/tmp/codegraph-alias-fixed-test.log`, `/tmp/codegraph-alias-clippy.log`, and
`/tmp/codegraph-alias-package.log`. No private graph or source payloads are checked
in. Nothing has been pushed.


<a id="source-evidence-ui-search-modal-readme-md"></a>

## evidence/ui/search-modal/README.md

Source: [evidence/ui/search-modal/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/search-modal/README.md). Historical record; retained rationale, methods and limitations.

### Double Shift search verification

The four Metal captures show the production modal with the explicit synthetic
fixture, in light/dark themes at 1050×680 and 1420×900. They were visually
inspected for horizontal centering, window bounds, input focus, readable result
context, selection, and reachable pagination. They do not claim physical keyboard
or VoiceOver validation.

Reproduce after building the desktop:

```sh
./target/debug/codegraph --scene fixture --script evidence/ui/search-modal/search.json \
  --snap /tmp/search.png --tree /tmp/search.tree.json \
  --theme dark --size 1050x680 --scale 1 --at 2 --realtime
```

Use `--theme light` and `--size 1420x900` for the remaining matrix entries.
Metal access is required. Both the graph and flowchart workspaces use the same
modal composition; production Rune tests cover opening and activating from each.

The search runtime regressions cover Double Shift interruptions, actual input
focus and typing, dismissal and focus restoration, category filtering, hidden
class/method reveal, navigation history, architectural layers, and stale queries
and graph replacement. Core regressions cover substring/rank behavior, Unicode,
literal punctuation, overloads, duplicate class scopes, aliases, and paging.

Validation results and any unavailable acceptance fixtures are recorded on
TASK-37. The upstream observers are recorded in the pinned, checksum-verified
`patches/rune-input-observers.patch`; dependency migration tests cover both prior
patch checkpoints.

#### Validation results

- Desktop check and all-target tests passed; the desktop binary has 178 passing
  normal tests, including 11 modal regressions and five shortcut-detector tests.
- Core check, all 46 core tests, and strict workspace Clippy passed.
- Full workspace acceptance ran with `--include-ignored --no-fail-fast`. Its five
  initial failures were prerequisite-related: three benchmarks require release
  mode, and two billing tests require explicit private-fixture inputs. All five
  passed in their appropriate reruns: the three release benchmarks used an isolated
  saved-workspace copy; billing source and compiled-metadata acceptance used an
  isolated fixture copy with freshly rebuilt outputs (10,827 declarations and
  17,331 relationships). The stale copied compiled outputs initially could not
  resolve `ListRecurring`; rebuilding the fixture resolved it without code changes.
- Python packaging tests: 76 passed. Dependency-patch migration tests: seven
  passed. Rune input-observer integration: four passed.
- `tools/package.py` built and signed the app and ZIP. A real-time Metal replay of
  the packaged app confirmed input, modal mounting, and completed search results.
- CodeScene reviewed affected project and upstream sources; project implementation
  and test modules score 10.0 (the export-only core module has no numeric score).
  The safeguard passed for all 23 eligible files. Existing upstream complexity
  findings were left outside this feature. Sonar is inactive: this repository has
  no `.sonar/config.json`.

The plain debug full-suite command still cannot run release-only benchmarks in
its debug process. Passing release reruns are recorded separately; no benchmark
or missing-fixture check is represented as passing without execution.

#### Result activation follow-up (TASK-38)

Class hits now focus the selected class card, with class-level connections, rather
than drilling into separate method nodes. Existing class-card member previews are
preserved. Method hits explicitly switch to Method grouping and select the found
method. Both reuse the layout-completion centering operation.

`class-result.png` and `method-result.png` were captured from the rebuilt packaged
app and visually inspected. Reproduce with the corresponding JSON script and
`--realtime --scene fixture --theme dark --size 1420x900 --scale 1 --at 2.5`.

Twelve modal runtime regressions pass, including camera coordinates within 0.1
points of the graph viewport center, selected target identity, switching from
method to class and back, and one history entry per activation. The normal desktop
suite passes 179 tests. Desktop check, strict workspace Clippy, packaging, and
CodeScene reviews/safeguard pass; both changed Rust files score 10.0. Independent
review found no remaining issues.

The required debug full-workspace command again stops at the three existing
release-only benchmark guards (183 other desktop tests pass). All three benchmark
cases pass in the separate release run against the isolated saved-workspace copy.
Initial normal-suite prerequisite errors were resolved by rebuilding the standalone
engine after the build directory had been cleaned.


<a id="source-evidence-ui-selection-emphasis-readme-md"></a>

## evidence/ui/selection-emphasis/README.md

Source: [evidence/ui/selection-emphasis/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/selection-emphasis/README.md). Historical record; retained rationale, methods and limitations.

### Direct-connection emphasis verification

TASK-27 keeps depth-based visibility and applies selection emphasis separately.
The selected anchor and direct incoming/outgoing neighbors stay bright; other
complete cards use 35% opacity. Edges not incident to an anchor, including their
arrowheads, use 8% of their previous alpha and paint before highlighted edges.
A visible selection overrides focus; missing selection falls back to visible
focus roots. With no roots, everything retains normal brightness.

`GraphEmphasis` is a pure, per-render presentation helper shared by cards and
edges. Geometry, relationship colors/dashes, traversal, and persistence are
unchanged. Dim cards and method rows remain interactive.

#### Evidence

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

#### Validation

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

#### Reproduction

With a copy of a current Recurring-billing cache containing `fetchPrintTemplates`:

```sh
cargo build --locked --bin codegraph
CODEGRAPH_DATA_DIR=/path/to/cache-copy ./target/debug/codegraph \
  --scene cached-focus --script evidence/ui/selection-emphasis/explore.json \
  --snap /tmp/emphasis.png --theme dark --size 1900x1100 --scale 1 --at 12
```

Use `dense.json` for depth 5 and `--theme light` for the light theme. Native Metal
access is required. These scripts use existing cached analysis; no refresh runs.

#### Action log

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


<a id="source-evidence-ui-source-popup-readme-md"></a>

## evidence/ui/source-popup/README.md

Source: [evidence/ui/source-popup/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/ui/source-popup/README.md). Historical record; retained rationale, methods and limitations.

### Hover source viewer verification — TASK-15

Implemented the approved method-source and aggregate-file popup in the production
Rune view. Analyzer output, graph storage and CLI analysis contracts are unchanged.

#### Behavior evidence

- Thirteen real Rune runtime tests in `src/ui_tests/source_popup.rs` exercise the
  actual graph cards, temporary source files, asynchronous jobs and production UI.
  They cover idle hover timing, leave grace, popup/trigger overlap, pin/unpin,
  Escape/Close, file listing and Back, target-line reveal, both scroll axes,
  stale/invalid-location messages, obsolete worker results, navigation dismissal,
  pane/window resizing, visible-size resize origins, clipboard copy without line
  numbers, long Unicode lines, and isolation from covered graph gestures.
- Source tests read real UTF-8/CRLF files, preserve trailing empty lines, enforce
  bounded reads and explicit failures, and test multiline Java/C#/TypeScript/TSX
  lexical highlighting beyond Kit's 32 KiB cap.
- Projection tests verify nested directory membership, namespaces, filtering,
  deduplication, identical project names in different builds, and membership
  exceeding the graph's 1,500-node page limit.

#### Visual inspection

The explicit `source-preview` and `source-files` tooling scenes use actual
checked-in Java files with synthetic graph relationships. They never replace
normal startup. The screenshots come from the native Metal renderer.

| Capture | Appearance / window | Verified |
| --- | --- | --- |
| `source-dark.png` | Dark, 1420×900 | Syntax colors, location marker, path, controls and long unwrapped lines |
| `source-light-min.png` | Light, 1050×680 | Window clamping, readable code and reachable resize/close controls |
| `files-light.png` | Light, 1420×900 | Distinct readable relative source paths |
| `files-dark-min.png` | Dark, 1050×680 | Listing and controls fit the minimum window |
| `files-resized.png` | Dark, 1050×680 | Actual corner resizing and scrolling a listing longer than its viewport |

No actionable clipping or control-placement findings remain. A partially visible
first/last source line is normal scroll clipping; the path/header and footer stay
outside the scrolling viewport.

Reproduce, changing scene, theme, size and output as needed:

```sh
cargo build --locked --bin codegraph
./target/debug/codegraph --scene source-preview \
  --script evidence/ui/source-popup/hover.json \
  --snap /tmp/source-popup.png --theme dark --size 1420x900 --scale 1 --at 3
./target/debug/codegraph --scene source-files \
  --script evidence/ui/source-popup/resize-files.json \
  --snap /tmp/files-resized.png --theme dark --size 1050x680 --scale 1 --at 3
```

#### Validation and action log

1. Recorded the approved plan and criteria through the Backlog CLI. Its task-lock
   operation required access to `.git`; retry with approved sandbox escalation
   succeeded.
2. Observed the projection membership test fail on absent metadata, then added
   typed groups and file membership before pagination.
3. Implemented bounded background reads and isolated lexical highlighting. Source
   tests passed using actual files; syntax regressions were reproduced before fixes.
4. Connected transient state, absolute hover deadlines, root-layer rendering,
   virtualized code/list rows, line reveal, pinning and corner resizing.
5. Review found blank-area input pass-through, covered-node hover replacement and
   late deadline initialization. Fixed these and verified through runtime tests.
6. Further review fixed partial trigger overlap, visible-size resize origins,
   virtual-row key reuse, TSX prose quotes, constrained/defaulted generic arrows,
   and escaped Java text-block delimiters.
7. Ran compiler, runtime and broader regression checks. Scoped formatting to app
   source; reversed an initial formatter traversal of vendored Rune only where
   the file exactly matched rustfmt of its original revision. Vendor status is clean.
8. Inspected all five captures. GPU rendering needed an approved sandbox escalation
   because the sandbox could not obtain a Metal device.
9. Built the release app/ZIP with `python3.13 tools/package.py`; the packaging
   script verifies pinned dependencies and the ad hoc code signature.

Checks: 139 Rust all-target tests, Clippy with warnings denied, and all 37 Python
tests passed. Thirteen source-loader/highlighting tests and thirteen popup runtime
tests are included in the Rust total. Detailed Rust/Clippy output is stored alongside this document. Specialized
ignored analyzer/toolchain tests were not rerun because analyzers are unchanged.
The final renderer-footer extraction was additionally checked with the thirteen
popup runtime tests and Clippy. CodeScene reviewed modified source; the final safeguard checked 17 eligible files
and passed without regressions. Sonar is inactive: `.sonar/config.json` is absent.

Physical mouse/trackpad, native window-shell interaction and VoiceOver have not
been exercised. Pointer, wheel, magnification, clipboard and resize behaviors were
verified in the real Rune runtime, with native GPU screenshots for visual QA.


<a id="source-design-qa-md"></a>

## design-qa.md

Source: [design-qa.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/design-qa.md). Historical record; retained rationale, methods and limitations.

### Graph Studio Rune Kit design QA

final result: passed

#### Visual truth and comparison

- Source: `evidence/ui/graph-studio/reference-graph-studio.png`, the third displayed concept selected by the user (Graph Studio).
- Implementation: `evidence/ui/graph-studio/real-dark.png`, captured by the native Rune Metal renderer from the production view.
- Source pixels: 1487 × 1058. Its intended desktop frame was 1440 × 1024; the generated output has a slightly different raster size. Implementation: 1420 × 900 logical points and pixels at 1×, retaining the approved native window default.
- `comparison.png` uniformly normalizes source width to 1420 without stretching; vertical excess remains visible beside the shorter native viewport. This is an approved layout interpretation, not a pixel-identical clone.
- State: Search open, method selected, Callees, depth 2. The concept uses Java data; implementation uses a real cached PacificFate C# neighborhood. Existing graph topology, supported directions and data semantics take precedence over invented mock content.
- Full-view evidence: `evidence/ui/graph-studio/comparison.png`.
- Focused evidence: `comparison-controls.png` and `comparison-inspector.png` retain original pixels for control/typography inspection.
- Additional screenshots cover fixture and real-project states in dark/light at 1420 × 900 and 1050 × 680. All eight captures and element trees are under `evidence/ui/graph-studio/`.

#### Findings and fidelity surfaces

No actionable P0/P1/P2 findings remain.

- **Typography:** shared Kit UI type uses 14-point primary controls, smaller secondary labels and system monospace for graph names/signatures. Namespace remains fully readable. Long identifiers and signatures wrap within the inspector; graph labels and search rows deliberately truncate, with full accessible labels and selected-method details available.
- **Spacing/layout:** 48-point rail, resizable drawer/inspector, dominant canvas, top graph navigation and bottom direction/depth/zoom. Minimum windows use extra control rows; pane budgeting keeps at least 448 points for the center. Inputs stay inside 12-point drawer padding; inspector content stays inside 16-point padding. Scrolling retains access to lower sections.
- **Colors/tokens:** coordinated graphite and light neutral appearances, blue selection, amber uncertain edges/cycle outlines and purple callbacks. Solid backgrounds replace the concept's decorative glow. Kit supplies focus, disabled and interaction states.
- **Assets:** no raster assets are needed in this developer workspace. Icons come from the pinned Rune OpenCode catalog and Kit glyphs; source paths are copied unchanged with attribution. The graph remains actual Rune shapes rather than a bitmap or decorative imitation.
- **Copy/content:** existing discovery, refresh, failure, provenance and graph-count messages remain. The unsupported tree and Both direction from the generated mock are intentionally absent, as agreed. Full source paths and actual relationship certainty remain inspectable.

#### Comparison history

1. Review found that maximum pane widths could hide graph controls at minimum window size. Added a center-width budget, reapplication during live drags, and multirow controls. Runtime tests cover restored maximum widths and dragging after initial layout.
2. Initial captures showed a truncated Namespace label and fixed-width Kit inputs extending past drawer padding. Increased grouping width with corresponding center budget and set inputs to fill available width. The final screenshot matrix and explicit geometry regressions verify both corrections.
3. Empty-state runtime testing found its prompt was laid out after the canvas primitive and clipped. Positioned the empty state as a canvas overlay; actual empty/loading/discovery-error tests now pass.
4. Final full-frame and focused comparisons show no overlapping persistent controls or unreadable panel content. Long identifiers wrap; smaller fitted graph labels reflect the existing camera scale and remain zoomable.

#### Behavior and accessibility

Real Rune runtime tests cover rail switching/collapse, Cmd-F actual editor focus, keyboard segmented selection, hidden-symbol search reveal, filters, depth selection and Escape priority, disclosures, disabled actions, pane resize/collapse/reset, persistence, camera/history preservation, appearance switching, reduced motion, empty states and actual asynchronous directory discovery errors. Existing graph/navigation tests remain.

This is a native application, so browser console checks do not apply. Metal captures and runtime interaction verification are complete; physical trackpad/window-shell interaction and VoiceOver use were not exercised.

#### Follow-up polish

- P3: long method names can wrap within a camel-case identifier in a narrow inspector. Resizing the inspector reveals more of the name on a line; the full signature remains selectable and readable.

#### Implementation checklist

- [x] Shared Rune Kit theme and focused components
- [x] Default/minimum dark/light visual inspection
- [x] Source/full-frame/focused comparison
- [x] Runtime interaction, responsive geometry and presentation compatibility checks
- [x] No actionable P0/P1/P2 findings
