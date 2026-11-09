# CodeGraph

A native Apple Silicon macOS call-graph explorer built with Rune. Select a repository
folder, select discovered .NET solutions/projects, Gradle builds, Maven projects or TypeScript configurations, and refresh.
The complete analyzed graph stays indexed; the canvas shows bounded aggregates or
method neighborhoods. Diffs, source editing, runtime tracing,
cross-service connections and AI features are outside this MVP.

## Run the local app

Run `./dev.sh` to bootstrap dependencies, build and launch directly from the
repository without installing the app. App arguments are forwarded, for example
`./dev.sh --scene fixture`. The `method-callees` and `method-callers` scenes show a
branching fixture using the production Graphviz layout. Both scripts work when
called from another directory.

Run `./prod.sh` to build the release app and create `dist/CodeGraph.dmg`. Open the
image and drag CodeGraph to Applications. The `.app` and ZIP are also kept in `dist`.
The scripts use Python 3.13 by default; set `PYTHON=python3.12` to use another
installed Python 3.12+ interpreter.

You can also open `dist/CodeGraph.app` directly. This local build is ad hoc
signed, not notarized. The packaged minimum macOS version is 27.0; broader OS
certification and Intel support are deferred.

Installed toolchains remain prerequisites: Node 24+ and project-installed TypeScript 6.0.x for TypeScript analysis, .NET 10 to run CallAtlas, the SDKs and
reference packs needed by your .NET projects, and JDK 25 to run the Java analyzer.
Gradle projects need a working wrapper and their configured Java toolchains.
Maven projects need Maven 3.9+ or Maven 4, preferably through their Maven Wrapper.
CodeGraph honors existing project/user settings, profiles and toolchains. Maven
analysis compiles through `test-compile` with standard test execution skipped;
normal plugins bound to earlier phases still run. Kotlin, Scala, Android and
specialized packaging requiring later phases are outside this Java integration.
CodeGraph reports missing toolchains; it does not install SDKs. Analysis can
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

Production code is the default. Test, generated and body-unavailable filters are
explicit, with hidden/omitted counts. Virtual / interface calls have their own
filter, enabled by default even for older saved views, independently of other
uncertain targets. These calls identify a declared method; an override may execute
at runtime. Gold dashed edges mean virtual/interface calls or other uncertain
targets; purple dashed edges mean callbacks/references. Amber outlines mark cycles.
A canvas is capped at 1,500 nodes and 6,000 edges; full search, refocusing and
paged neighborhoods keep the remaining indexed methods reachable.

Graphs, diagnostics, source fingerprints and navigation are stored in SQLite under
`~/Library/Application Support/CodeGraph`. Refresh builds a separate immutable
generation and publishes only on success. Failure/cancellation preserves the last
successful graph. Source/build changes mark it stale; refresh is manual.
`CODEGRAPH_DATA_DIR` selects an isolated cache for development or acceptance checks.

## Reproduce the build

Prerequisites: Apple Silicon macOS, Rust/Cargo, Xcode command-line tools, Python
3.12+, installed .NET/JDK toolchains above, and Graphviz 16.1.0 (`dot` on PATH).

```sh
python3.13 tools/bootstrap.py
cargo build --locked --bin codegraph
cargo run --locked --bin codegraph
python3.13 tools/package.py
```

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
cargo run --bin codegraph -- analyze fixtures/dotnet/Fixture.sln fixtures/gradle fixtures/maven/standalone/pom.xml
cargo run --bin codegraph -- analyze /path/to/project/tsconfig.json
```

Collect stage timings and work counters with an optional JSON report:

```sh
cargo run --release --bin codegraph -- analyze --timings /tmp/codegraph-timings.json /path/to/build
```

The native app uses the same measurements through its initial layout. Stage times
can nest and must not be summed; the CLI report covers its headless pipeline.
See [analysis performance evidence](evidence/performance/analysis.md) for the
release benchmark protocol, complete graph comparisons and measured results.

## Function flowcharts

Select a Java or TypeScript method and choose **Open flowchart** to explore its
source-derived branches beside the analyzed source snapshot. Follow known project
calls, inspect uncertain targets, and return to the unchanged call graph. Recognized
Mono/Flux stages distinguish assembly from deferred callbacks. Refresh older caches
to collect flow metadata. See [function flowcharts](docs/function-flowcharts.md) for
supported semantics and limitations.

## Architectural layers

Layer groups methods into configurable architecture bands with Spring/React presets,
repository rules and automatic import from Interactive Review. Expand layers into
projects, classes and methods; source facts are cached for recompilation-free rule
changes. See [architectural layers](docs/architectural-layers.md) for setup, matching,
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
project dependencies beforehand. CodeGraph does not run npm scripts, compiler
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
limits reject the refresh. See [analyzer acceptance](evidence/analyzers/ACCEPTANCE.md)
and [pipeline evidence](evidence/analyzers/native-pipeline.md).

## Validation

```sh
cargo clippy --all-targets --no-deps -- -D warnings
cargo test --all-targets -- --include-ignored
python3.13 -m unittest discover -s tests -v
cargo run --release --bin benchmark -- evidence/performance/cache-local
```

The ignored compiler/build regressions require bootstrapped artifacts, installed
JDK 25 and working Maven/Gradle toolchains. To test a pinned Maven distribution,
set `CODEGRAPH_TEST_MAVEN=/absolute/path/to/bin/mvn` when running
`cargo test --test java_builds maven_ -- --ignored --test-threads=1`.
TypeScript compiler fixtures run with `node --test tools/typescript/*.test.js`;
set `CODEGRAPH_TEST_TYPESCRIPT_ROOT` to an installed TypeScript package if needed.
The Rust TypeScript integration regressions are ignored by default alongside other
real-toolchain tests; run `cargo test --test typescript_pipeline -- --ignored --test-threads=1`.
See [TypeScript acceptance](evidence/analyzers/typescript.md) and [Maven acceptance](evidence/analyzers/maven.md) for the compatibility matrix. Headless Rune GPU screenshots and interaction checks are under
`evidence/ui`; `--scene fixture` is an explicit synthetic test scene, never normal
startup data. Real-project results and scale measurements are summarized in
[evidence/VALIDATION.md](evidence/VALIDATION.md). Live physical trackpad/window
validation remains outstanding: the execution session reports zero active displays.
