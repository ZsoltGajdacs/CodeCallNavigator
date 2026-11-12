# CodeCallNavigator reference

Detailed usage, build, analyzer and validation notes. For a quick start, see the
[README](../README.md). Commands below run from the repository root.


## Build selection

Click **Find builds** to rescan the repository and open the scrollable build picker
beside the button. Checkboxes update the selection immediately; close the picker
with its close button, Escape or a click outside. Hover over **Find builds** to see
the selected builds. Rescanning preserves choices for existing targets and uses
the usual defaults for newly discovered builds.

Build labels show the last folder and the project name or configuration filename.
Declared names are read directly from Maven, Gradle and C# project metadata;
computed or unavailable names fall back to the folder or project file stem.
Discovery does not execute build tools to resolve names.

## Project Explorer

The left sidebar shows an expandable file tree below the repository and build
controls. It includes hidden, ignored and non-source files. Folders load when
expanded; symbolic links are listed without following linked directories. Use
**Refresh files** to reload listings after filesystem changes. Directory errors
include a retry action.

Click a file once to select its row. Double-click it to reveal and frame its
existing class nodes in the graph, including classes from multiple analyzed
builds. Current filters still apply; files without visible analyzed classes show
an explanation without changing graph navigation. Back/Forward retraces file
navigation. This does not open a source preview.

Selecting graph nodes highlights their files, expands their parent folders and
scrolls the tree to the first highlighted file. Graph keyboard focus stays in
the graph, and a closed sidebar stays closed. Files outside the project folder
are not added to the tree. Explorer selection and expansion are local to the
current session; refresh preserves entries that still exist.

## Run the local app

Run `./dev.sh` to bootstrap dependencies, build and launch directly from the
repository without installing the app. App arguments are forwarded, for example
`./dev.sh --scene fixture`. The `method-callees` and `method-callers` scenes show a
branching fixture using the production Graphviz layout.

Run `./optimized-dev.sh` for the same direct launch using release builds of the
desktop app and engine, without packaging or installing the app. For example,
`./optimized-dev.sh --scene fixture` launches the fixture with release optimization.
All three scripts work when called from another directory.

Run `./prod.sh` to build the release app and create `dist/CodeCallNavigator.dmg`. Open the
image and drag CodeCallNavigator to Applications. The `.app` and ZIP are also kept in `dist`.
The scripts use Python 3.13 by default; set `PYTHON=python3.12` to use another
installed Python 3.12+ interpreter. All three scripts clean Cargo's build directory
before building, removing previous Rust build artifacts and dependencies. This
forces a full Rust rebuild on each run.

You can also open `dist/CodeCallNavigator.app` directly. This local build is ad hoc
signed, not notarized. The packaged minimum macOS version is 27.0; broader OS
certification and Intel support are deferred.

Installed toolchains remain prerequisites: Node 24+ and project-installed TypeScript 6.0.x for TypeScript analysis, .NET 10 to run CallAtlas, the SDKs and
reference packs needed by your .NET projects, and JDK 25 to run the Java analyzer.
Gradle projects need a working wrapper and their configured Java toolchains.
Maven projects need Maven 3.9+ or Maven 4, preferably through their Maven Wrapper.
CodeCallNavigator honors existing project/user settings, profiles and toolchains. Maven
analysis compiles through `test-compile` with standard test execution skipped;
normal plugins bound to earlier phases still run. Kotlin, Scala, Android and
specialized packaging requiring later phases are outside this Java integration.
CodeCallNavigator reports missing toolchains; it does not install SDKs. Analysis can
restore dependencies and compile all configured source sets, including tests,
but does not run applications or test-execution tasks.

Use Project, Namespace, Folder, Layer or Method grouping (including global-namespace C# code).
The Graph Studio workspace uses Rune Kit controls and follows the system's light or
dark appearance. The activity rail switches between Repository, Search and Graph
filters; pressing the active rail button collapses its drawer. Drag the drawer or
inspector's inner edge to resize it, or double-click the edge to restore its default
width. The Inspector button toggles the right pane. Pane sizes, visibility and the
active drawer are restored with the saved workspace, independently of graph history.
At smaller window sizes, pane widths are constrained to keep graph controls reachable.
Cmd-F opens the Search drawer and focuses the field; depth is selected from its menu.
The graph canvas still uses Rune's custom shapes and production Graphviz geometry.

Method opens all entry-point candidates, including uncalled methods, with verified
C# and Java application starts highlighted. C# starts come from the compiler;
Java starts are public static void main(String[]). TypeScript module initializers and
uncalled functions remain candidates; recognized Next.js App Router exports are
labeled framework entries separately from verified executable starts. Framework handlers can appear
as uncalled candidates without application-start highlighting. Cached graphs from
an older method policy require a refresh before they can be opened.

Click an element to center its call tree: callers appear above, callees below,
and unrelated elements are omitted. Depth applies independently to both sides;
click a neighbor to continue walking the tree. A single-click waits for the system
double-click interval before moving the graph, so the second click can land reliably.
Focused connections match the selected element: classes connect to classes,
methods to methods, and namespaces/packages to complete namespaces/packages.
Folders connect at the selected directory depth, using shallower containing folders
where needed. Project and layer selections likewise show their own level. Focus
automatically aligns previously expanded or collapsed branches without changing
the saved overview expansion state.
Double-click a group to drill through its hierarchy to classes. Each class box
lists its filtered methods, with up to 12 rows visible and independent scrolling
for longer lists. Click a method row to enter that method's tree, or double-click
the class header to see its methods as individual nodes with external call context.
Double-clicking a method keeps the call-tree view; use the inspector's explicit
function-flowchart action to inspect its internal execution.

Overview restores the active grouping overview, including entry roots in Method
view. Search covers the entire index with pagination, including currently filtered
symbols. Overview's Callers/Callees controls choose its top-down traversal; focused
trees always show both sides. Arrows retain caller-to-callee direction. Shared
methods appear once with cross-links, and cycles remain visible. A node reachable
on both sides appears on the nearer side, with ties above the focus. Shortest paths
retain execution order. Back/Forward restores exploration context and camera.

Drag to pan, scroll or pinch to zoom, or use Fit and zoom buttons. Scrolling inside
a class method list scrolls that list. Focus changes preserve zoom and center the
selected element; Fit frames the visible graph. Cmd-O opens the folder chooser;
Cmd-F focuses search. The inspector shows signatures, project ownership, locations,
relationship certainty and analyzer provenance. Source previews and function
flowcharts open separately.

Graphs retain source-declared methods and authored functions, callbacks, accessors,
and top-level execution at every visibility level. Constructors, initializer
scaffolding, compiler-created helpers, inherited reference placeholders and Object
method signatures (including explicit overrides) are excluded from the index.
Unrelated overloads remain. Removing a symbol removes its incident relationships;
paths are not reconnected through excluded code. Methods declared in generated
source files remain available through the generated-code filter. A project with
only excluded symbols produces a valid empty graph.

Production code is the default. The canvas filters separate three dimensions:

- **Code:** test code, generated code, and methods without implementation.
- **Relationship type:** calls, function references, framework/lifecycle,
  HTTP/navigation, and other relationships (including structural and bootstrap links).
- **Target resolution:** directly resolved, virtual/interface dispatch, possible
  targets, and other/unspecified analyzer certainty.

Both a relationship's type and its target resolution must be enabled, and both
endpoint methods must pass the code filters. Resolving a target does not prove
execution; passing, storing or registering a function does not establish invocation.
Details show these categories alongside the original analyzer facts and evidence.
New views enable all relationship categories. Upgrading an older saved view keeps
its code filters and resets all relationship choices to enabled once; subsequent
choices persist through reload and navigation history.

Muted solid edges are directly resolved calls. Purple dashed edges are function
references; amber dashed edges are remaining relationships. Amber outlines mark cycles.
Hidden and omitted counts reflect the active filters.
A canvas is capped at 1,500 nodes and 6,000 edges; full search, refocusing and
paged neighborhoods keep the remaining indexed methods reachable.

Graphs, diagnostics, source fingerprints and navigation are stored in SQLite under
`~/Library/Application Support/CodeCallNavigator`. Refresh builds a separate immutable
generation and publishes only on success. Failure/cancellation preserves the last
successful graph. Source/build changes mark it stale; refresh is manual.
`CODECALLNAVIGATOR_DATA_DIR` selects an isolated cache for development or acceptance checks.

**Clear cache**, beside Generate graph / Refresh, removes the selected project's
reusable inference cache. Wait for “Cache cleared · start analysis to rebuild.”,
then start analysis manually. The last successful graph and navigation remain
available. This does not clean build outputs or dependency caches. Clearing and
analysis cannot run together for the same project.

The same operation is available without opening the desktop:

```sh
codecallnavigator-engine clear-cache --project /path/to/project
```

It removes only `.codecallnavigator/inference.sqlite3` and its SQLite sidecars; an absent
cache is already clear. It preserves `.codecallnavigator/graph.json` and project files.
Unsafe cache paths, deletion errors and a project lock held by analysis cause a
reported failure; retry after resolving the error.

## Search classes and methods

Press Shift twice within 400 ms, use Cmd/Ctrl+F, or click Search in the toolbar.
The modal searches names anywhere in the complete loaded graph, including symbols
hidden by visibility filters. All, Classes and Methods narrow the results;
TypeScript file-backed groups are marked File. Classes without indexed methods
are not available.

Use Up/Down to select, Enter to focus the result in the graph, and Escape to close.
Class results select a class-level node; method results select a method-level
node. The selected node is centered. Results selected from a flowchart return to
the graph. Previous/Next reaches further pages. Reopening retains the query and
category for the current session. Source bodies and file paths are not searched.

## Reproduce the build

Prerequisites: Apple Silicon macOS, Rust/Cargo, Xcode command-line tools, Python
3.12+, installed .NET/JDK toolchains above, and Graphviz 16.1.0 (`dot` on PATH).

```sh
python3.13 tools/bootstrap.py
cargo build --locked --bin codecallnavigator
cargo run --locked --bin codecallnavigator
python3.13 tools/package.py
```

A pre-rename JDT cache with only `.codegraph-sha256` is not accepted by the renamed
bootstrap. Use a fresh `vendor/jdtls` cache when bootstrapping an older checkout;
the verified download writes `.codecallnavigator-sha256`.

`analyzers.lock.json`, Cargo.lock and the Java adapter dependency locks pin the source and
dependency versions. Bootstrap verifies the checked-in CallAtlas patch and refuses
to overwrite incompatible dirty checkouts. Packaging verifies the pinned sources,
bundles the analyzer artifacts, relocates Graphviz and its required dylibs, collects
third-party notices, ad hoc signs and verifies the app, and produces the ZIP.
No Homebrew installation is needed to run the packaged Graphviz.

One-shot build/analyzer subprocesses receive closed standard input, so launching
through a terminal cannot suspend Gradle on background terminal reads. TypeScript's
internal worker keeps its dedicated protocol pipe.

Analysis reuses classpath and source inventories only within the current refresh.
Gradle ownership discovery remains separate; compilation and final metadata export
share one invocation. TypeScript retains its compiler for a single selected target;
multi-target analysis bounds memory by recreating one verified worker at a time.
All content snapshots and per-context classpath rules remain enforced.

The same production analyzer pipeline is available without opening a window:

```sh
cargo build --locked -p codecallnavigator-engine --bin codecallnavigator-engine
cargo run -p codecallnavigator-engine --bin codecallnavigator-engine -- analyze --project /path/to/project tsconfig.json
cargo run --bin codecallnavigator -- analyze fixtures/dotnet/Fixture.sln fixtures/gradle fixtures/maven/standalone/pom.xml
cargo run --bin codecallnavigator -- analyze /path/to/project/tsconfig.json
```

The standalone engine updates the single `.codecallnavigator/graph.json` beside project
configuration. Commit that file to retain graph history in Git. Compare revisions
with `codecallnavigator-engine compare --project /path/to/project --base HEAD~1 --head HEAD`
or compare a committed graph with `--head WORKTREE`. See
[the engine contract](../docs/standalone-engine.md) for packaging, identities and
language extension boundaries.

Collect stage timings and work counters with an optional JSON report:

```sh
cargo run --release --bin codecallnavigator -- analyze --timings /tmp/codecallnavigator-timings.json /path/to/build
```

The native app uses the same measurements through its initial layout. Stage times
can nest and must not be summed; the CLI report covers its headless pipeline.
See [analysis performance evidence](../evidence/performance/analysis.md) for the
release benchmark protocol, complete graph comparisons and measured results.

## Function flowcharts

Select a Java or TypeScript method and choose **Open flowchart** to explore its
source-derived branches beside the analyzed source snapshot. Follow known project
calls, inspect uncertain targets, and return to the unchanged call graph. Recognized
Mono/Flux stages distinguish assembly from deferred callbacks. Refresh older caches
to collect flow metadata. See [function flowcharts](../docs/function-flowcharts.md) for
supported semantics and limitations.

## Architectural layers

Layer groups methods into configurable architecture bands with Spring/React presets,
repository rules and automatic import from Interactive Review. Expand layers into
projects, classes and methods; source facts are cached for recompilation-free rule
changes. See [architectural layers](../docs/architectural-layers.md) for setup, matching,
configuration migration and cache behavior. New generations retain source text for
content-pattern classification; older caches require refresh for Layer mode.

## Reading source on the graph

Rest the pointer over a method for 400 ms to open its current source file at the
recorded line. In Folder or Namespace mode, hover an aggregate card to list the
source files represented by its methods under the current filters; click a file
to read it and use Back to return to the list.

Move into the popup to keep it open, or Pin it while exploring the graph. Drag its
bottom-right corner to resize it, scroll in either direction, and select source
text to copy it. Close or Escape dismisses it. Popup size lasts for the session;
changing the graph view clears the preview. Files are read from disk rather than
saved with the graph, with a 16 MiB limit and an explanation for unavailable files
or recorded lines. Refresh a stale graph to update its method locations.

## Engines and accuracy

C# uses compiler-backed CallAtlas. A small checked-in patch qualifies its serialized
symbol identities by compiler assembly identity and exports entry candidates plus
compiler-verified application starts; it changes no call binding.
Java uses existing java-callgraph2 4.1.0 with BCEL 6.13.0 over real Gradle or Maven compilation
output. The planned JDT LS integration failed the StoryEngine acceptance gate and
was replaced before UI implementation. No heuristic call resolver was substituted.

Java build adapters provide evaluated module/source ownership to the shared bytecode
pipeline. Maven reactors retain cross-module calls and production/test/generated
classification. Parent-only aggregators contribute module ownership and build
inputs. Nested POMs remain selectable, with outermost projects selected by default;
when both build tools share a directory, Gradle is selected initially. Overlapping
selected modules are rejected before compilation. The CLI accepts POM files and
repository folders. Profile/property controls are not added to the app.

Java lambdas use their assigned field or local variable name when source attribution
is unambiguous, for example `COST_TYPE_VALIDATION`. Anonymous callbacks show their
enclosing context and source location. Overlapping lambdas, same-line ambiguity,
and builds without debug lines use readable contextual fallbacks. Original bytecode
signatures remain available in the inspector and search. Refresh existing workspaces
to rebuild cached graphs with these labels.

TypeScript uses the selected project's installed TypeScript 6.0.x compiler with
Node 24+. The bundled bridge uses public compiler APIs, configured source ownership
and source redirects for project references; no JavaScript is emitted. Install
project dependencies beforehand. CodeCallNavigator does not run npm scripts, compiler
plugins, Next.js configuration or dependency installation. Missing dependencies,
configuration/resolution/type errors and unsupported compiler versions reject the
refresh and retain the last successful graph.

The UI and CLI discover `tsconfig.json` and `tsconfig.*.json`; the standard config
is selected by default in each directory, with variants still selectable. Identical
referenced configurations share graph identities; different configurations owning
the same source are rejected. `.ts`, `.tsx`, `.mts` and `.cts` callables, anonymous
callbacks and module initializers are indexed. JavaScript/allowJs projects are deferred.

React rules recognize resolved imports, component rendering, memo/forwardRef,
static event/prop callbacks and supported hook callbacks/cleanup. Framework edges
carry explicit kinds and provenance; possible execution and references do not
establish runtime order or frequency. `useCallback` caches a function binding and
does not itself invoke that function. Next.js rules recognize standard App Router
exports under `app` or `src/app`. Pages Router, custom routing configuration,
mutable callback maps/refs and arbitrary higher-order wrappers are deferred;
dynamic delegation such as `withStableCallbacks` remains explicitly unresolved.

Only project-owned symbols are indexed. Framework-invoked declarations remain
visible even without project callers. Possible targets and callback references
retain engine provenance; they do not establish runtime execution. DI, reflection
and framework wiring are supported only where reported by the engine. C# source
generator output and some operators/dispatch candidates have upstream gaps. Java
retains compiler bridge/lambda declarations and explicit inherited/reference
placeholders with unavailable bodies; bytecode supplies lines but no columns.
Build/import errors, incomplete output, concurrent source changes and analyzer
limits reject the refresh. See [analyzer acceptance](../evidence/analyzers/ACCEPTANCE.md)
and [pipeline evidence](../evidence/analyzers/native-pipeline.md).

## Validation

Use the repository entry points, which work from any current directory:

```sh
./verify-light.sh
./verify-full.sh
./verify-full.sh --dry-run
./verify-light.sh --base origin/main
```

Install the locked test-only Python dependencies once in the repository's isolated
verification environment (the scripts select it automatically):

```sh
python3.13 -m venv .venv-verification
.venv-verification/bin/python -m pip install --require-hashes -r requirements-verification.txt
```

The lock includes the existing JSON Schema contract-test dependency. Set `PYTHON`
only when intentionally using another interpreter with these locked packages.
Documentation-only light checks do not require the test packages.

Choose light for small changes with minimal production-code effect. It checks
whitespace and the syntax/formatting of changed project files, builds the matching
debug engine, runs workspace compilation, Clippy, ordinary Rust tests, Python
suites in both `tests/` and `tools/`, and changed-source CodeScene reviews plus the
repository safeguard. For documentation-only changes, it performs static checks and artifact cleanup
without builds, runtime tests or benchmarks. Formatting uses `rustfmt --check`
with `skip_children=true` only on changed project-owned Rust files. It never
rewrites files or recursively formats vendored sources.

Choose full for substantial runtime/UI, analyzer, dependency, bootstrap or
packaging changes. In addition to the regular checks, it bootstraps pinned
analyzers, runs TypeScript tests, builds a matching release engine, runs **all**
Rust tests including ignored tests in **release mode**, runs the storage benchmark,
and builds/signature-verifies the local app and engine packages. Tests are serial
and benchmark paths point to an isolated snapshot. This avoids invoking the
release-only canvas benchmarks in debug mode. Offscreen GPU throughput does not
claim interactive/native displayed FPS; perform change-specific visual acceptance
separately when applicable.

Full requires Apple Silicon macOS, native Metal/process-table access, Python 3.12+
(the local verification environment, falling back to `python3.13`), Rust with Clippy/rustfmt, Node 24+, JDK 25,
.NET SDK 10, Maven 3.9+/4, Graphviz and Xcode command-line/signing tools. Run it
outside the macOS sandbox. It also requires explicit inputs, configured once in
your shell (replace these example paths with your actual installations):

```sh
export CODECALLNAVIGATOR_TS_NODE_MODULES=/absolute/path/to/node_modules
export CODECALLNAVIGATOR_RECURRING_BILLING=/absolute/path/to/Recurring-billing
export CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE="/absolute/path/to/saved-application-data"
./verify-full.sh
```

The modules directory must contain TypeScript 6.0.x, React 19 and Next 16 fixture
dependencies. The runner supplies the matching
`CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT` and engine binary paths; conflicting
TypeScript settings fail preflight. The private Recurring Billing acceptance
requires its Maven checkout and private-repository access; it is not silently
excluded. `CODECALLNAVIGATOR_TEST_MAVEN` can choose Maven for fixtures that support
that override; `mvn` must also be available on PATH for the other fixtures.

The profiling source is an application data directory with `catalog.sqlite3`,
saved navigation and an active published graph with relationships. You can also
pass `--profile-source /absolute/path`. The runner uses the SQLite backup API to
snapshot the catalog (including committed WAL data), copies the immutable graph
generation, and benchmarks only that copy. It never points benchmarks at the live
application store. Keep the selected source available through snapshot creation.

Artifact cleanup runs after verification, including failed/interrupted attempts,
and has its own `cleanup.json` report. An active or uninspectable artifact is
retained and cleanup reports a failure instead of deleting it. Use
`--keep-artifacts` to retain all artifacts for investigation explicitly.

- Light removes incremental compiler state while retaining compiled dependencies
  and reusable debug/release binaries. Verification sets `CARGO_INCREMENTAL=0`
  so its builds do not recreate that state.
- Full cleans the debug and release Cargo profiles after verification/packaging,
  including compiled dependency artifacts. The next build recompiles them. A full
  run that fails before reaching build stages does not clean existing profiles.
- Both remove their run-local profiling and scale benchmark data, and prune
  rotated app backups, keeping the newest rollback for each current/legacy app
  name. Current app/engine packages, ZIPs/DMGs and verification reports remain.
- Cargo's own age-based cleanup is enabled for downloaded dependency caches using
  `CARGO_CACHE_AUTO_CLEAN_FREQUENCY=always`; shared caches are not manually purged.
  Cargo retains recently used cache entries according to its
  [documented global-cache policy](https://doc.rust-lang.org/cargo/reference/config.html#global-caches).

The checkout lock prevents competing verification runs. Cleanup additionally
checks open-file usage with native `lsof` before removing artifact directories;
insufficient access or active users produce an explicit cleanup failure.

CodeScene uses the installed `cs-mcp` executable over stdio MCP, including its
normal authentication/license configuration. Select another executable path with
`--codescene-command`. Missing tools, unavailable checks, quality regressions and
incomplete results fail verification. Sonar remains a separate connected quality
gate if `.sonar/config.json` is configured; it is currently absent in this project.

A checkout lock prevents competing full/light runs from sharing build/package
outputs. Each invocation writes fresh numbered stage logs, `results.json`, `cleanup.json` and `changes.json`
under `target/verification/`. Failed, timed-out and not-run stages remain distinct;
a failed/incomplete full run never reports full success. `--timeout SECONDS`
bounds each build/test/package stage (default 1800); interruption/timeout stops
the process group. If OS restrictions prevent descendant cleanup, the script
reports `cleanup-unavailable` and fails. `--output /new/directory` retains evidence
elsewhere; existing output directories are rejected. `PYTHON=python3.12` chooses
another supported interpreter. `--base` controls committed changes plus staged,
unstaged and untracked files; the default base is HEAD. No-change light runs are
static-only, so use full for an unconditional complete verification.

See [TypeScript acceptance](../evidence/analyzers/typescript.md) and [Maven acceptance](../evidence/analyzers/maven.md) for the compatibility matrix. Headless Rune GPU screenshots and interaction checks are under
`evidence/ui`; `--scene fixture` is an explicit synthetic test scene, never normal
startup data. Real-project results and scale measurements are summarized in
[evidence/VALIDATION.md](../evidence/VALIDATION.md). Live physical trackpad/window
validation remains outstanding: the execution session reports zero active displays.
