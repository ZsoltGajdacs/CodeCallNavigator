# TypeScript, React and Next.js acceptance

Validated 2026-11-09 on Apple Silicon macOS with Node 24.15.0,
project-installed TypeScript 6.0.3, React 19.3.1 and Next.js 16.4.2.
The supported compiler range is 6.0.x with Node 24+; other compiler versions
fail explicitly rather than silently changing binding behavior.

## Real-project and packaging gate

The signed packaged executable analyzed PacificFateMapEditor/tsconfig.json
without emitting JavaScript, running scripts, installing dependencies or launching
the editor. It published **5,850 symbols and 9,090 relationships**, including
`Page -> EditorPage` as `react-render`, certainty `possible`. `RootLayout` and
`Page` are framework entries; neither is labeled a verified executable start.
A timed run took **8.68 s**, with **867,942,400 bytes maximum resident set size**
reported by macOS time (process/toolchain high-water measurement, not a total
live-memory budget). Disk-image generation was active concurrently.

A second packaged run combined the C# fixture, Gradle fixture, standalone Maven
fixture and PacificFateMapEditor into **5,879 symbols and 9,117 relationships**.
It preserved both framework entries separately from three verified C#/Java starts.
`typescript-summary.json` records languages, relationship kinds, engine versions,
representative effect/event bindings and unresolved counts.

`../ui/typescript/map-editor.png` and its UI tree show the actual cached graph,
searching the real page source, focusing `Page`, its possible render edge to
`EditorPage`, and the explicit framework-entry Inspector label. The screenshot
uses the packaged app and production Graphviz layout, with no synthetic graph.
Headless native Metal interaction passed; physical mouse/trackpad validation is
not claimed.

## Compiler and integration regressions

The Node fixtures create temporary projects against actual installed compiler,
React and Next.js dependencies. They cover aliases/re-exports, overloads, anonymous
callback ownership, constructor fields, accessors including bracket/compound
writes, conditional function targets, source-only project references, config/type
errors, React callbacks/cleanup/references and Next.js export conventions.
Negative fixtures prevent shadowed hooks, cached callbacks, private app folders,
non-route utility exports and unsupported delegation from creating proven calls.

Seven Rust production-pipeline tests cover callback attribution, reference closure
deduplication, conflicting owners, consumed dependency declarations and source
membership freshness, framework provenance/cache round trips, packaged resource
layout, CLI failed-refresh preservation and cancellation. Discovery tests also
protect Java packages named coverage/dist from TypeScript output exclusions.
Old generation metadata loads with empty framework-entry/provenance defaults.

Validation commands:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --all-targets --locked -- --include-ignored --test-threads=1
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --all-targets --no-deps -- -D warnings
python3.13 -m unittest discover -s tests -v
node --test tools/typescript/adapter.test.js
python3.13 tools/package.py
```

All 108 Rust tests (including existing C#/Java compiler and packaged-resource
regressions), 27 Python tests and all 19 Node compiler fixtures passed. Packaging
validated pinned dependency sources, bundled the bridge and Graphviz, generated
ZIP/app artifacts, and verified the ad hoc signature. The refreshed DMG also passed
`hdiutil verify`. The final CodeScene safeguard passed across all 44 eligible files. The upstream Rune release
build retains its existing dead-code warning; CodeGraph Clippy passed with
warnings denied.

CodeScene reviewed all modified source. All modified source modules score 10.0 after compiler setup, dependency inputs,
accessor handling and framework rules were separated by responsibility.
Independent planning/exploration, implementation, static review and compiler
acceptance testing were performed. Sonar is inactive because this project has
no .sonar/config.json.

## Accuracy and compatibility

Calls are bound through public compiler APIs. React/framework rules remain
separate from compiler binding and carry explicit provenance. Propagated callback
targets are possible; references and rendering do not establish runtime ordering,
frequency or context-specific execution. The real project reports 616 unresolved
dynamic call sites and explicit unresolved React callback/component/event/spread
counts. Mutable refs, callback maps and withStableCallbacks delegation are not
resolved by arbitrary data-flow guesses.

JavaScript/allowJs, Pages Router, custom Next.js routing configuration and general
higher-order-function propagation remain outside this release. TypeScript config
inheritance, project references, missing resolution candidates, dependency type
inputs and watched source directory membership participate in freshness checks.
Errors, cancellation and stale inputs cannot replace a successful generation.
