# CodeGraph

A native Apple Silicon macOS call-graph explorer built with Rune. Select a repository
folder, select discovered .NET solutions/projects or Gradle builds, and refresh.
The complete analyzed graph stays indexed; the canvas shows bounded aggregates or
method neighborhoods. Maven, diffs, source previews/editing, runtime tracing,
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

Installed toolchains remain prerequisites: .NET 10 to run CallAtlas, the SDKs and
reference packs needed by your .NET projects, and JDK 25 to run the Java analyzer.
Gradle projects need a working wrapper and their configured Java toolchains.
CodeGraph reports missing toolchains; it does not install SDKs. Analysis can
restore dependencies and compile all configured source sets, including tests,
but does not run applications or test-execution tasks.

Use Project, Namespace, Folder or Method grouping (including global-namespace C# code).
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
Java starts are public static void main(String[]). Framework handlers can appear
as uncalled candidates without application-start highlighting. Older cached graphs
still open; refresh them to enable verified-start highlighting.

Click groups to expand; double-click a method to focus. Overview restores the
entry roots in Method view. Search covers the entire index with pagination,
including currently filtered symbols. Choose Callees to read from the focus down
to its callees, or Callers to read from the focus down to its callers. All views
use top-down branch grouping; arrows always retain caller-to-callee direction,
so arrows point upward in Callers view. Shared methods appear once with cross-links,
and cycles remain visible. Depth bounds the neighborhood; entry-rooted Callers
views can contain only their roots. Shortest paths retain execution order.
Explore depth, shortest directed paths and history. Drag to pan, scroll/pinch to
navigate, or use Fit and zoom buttons. Cmd-O opens the folder chooser; Cmd-F focuses
search. The inspector shows signatures, project ownership, locations, relationship
certainty and analyzer provenance. No source code is displayed.

Production code is the default. Test, generated and body-unavailable filters are
explicit, with hidden/omitted counts. Gold dashed edges mean possible/uncertain
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

`analyzers.lock.json`, Cargo.lock and the Java dependency lock pin the source and
dependency versions. Bootstrap verifies the checked-in CallAtlas patch and refuses
to overwrite incompatible dirty checkouts. Packaging verifies the pinned sources,
bundles the analyzer artifacts, relocates Graphviz and its required dylibs, collects
third-party notices, ad hoc signs and verifies the app, and produces the ZIP.
No Homebrew installation is needed to run the packaged Graphviz.

The same production analyzer pipeline is available without opening a window:

```sh
cargo run --bin codegraph -- analyze fixtures/dotnet/Fixture.sln fixtures/gradle
```

## Engines and accuracy

C# uses compiler-backed CallAtlas. A small checked-in patch qualifies its serialized
symbol identities by compiler assembly identity and exports entry candidates plus
compiler-verified application starts; it changes no call binding.
Java uses existing java-callgraph2 4.1.0 with BCEL 6.13.0 over real Gradle compilation
output. The planned JDT LS integration failed the StoryEngine acceptance gate and
was replaced before UI implementation. No heuristic call resolver was substituted.

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

The ignored compiler regression requires bootstrapped artifacts and installed
JDK 25. Headless Rune GPU screenshots and interaction checks are under
`evidence/ui`; `--scene fixture` is an explicit synthetic test scene, never normal
startup data. Real-project results and scale measurements are summarized in
[evidence/VALIDATION.md](evidence/VALIDATION.md). Live physical trackpad/window
validation remains outstanding: the execution session reports zero active displays.
