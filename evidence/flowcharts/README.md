# Function flowchart verification

Verified on Apple Silicon macOS on 2026-11-09.

## Behavior and regressions

The shared builder tests verify completion-preserving finally/cleanup, return
replacement, resource acquisition failures, labeled loop jumps, switch yields,
short-circuit evaluation, ordered effectful case tests, catch-all handling,
opaque boundaries, detached callbacks and stable Graphviz link identities.
Inventory tests cover composition across supported/unsupported languages, symbol
pruning and visible chart-size limits.

Production acceptance executes actual Java and TypeScript fixture programs and
checks that their observed marker traces follow chart paths. Early returns cannot
resume at statements after finally. Java checks include two-resource acquisition
failure and finally overriding a return. Reactor 3.8.7 executes real assembly and
subscription traces; charts keep eager argument evaluation outside deferred
callbacks and reject lookalike application methods.

Compiler tests also cover overload identity, duplicate contexts, ambiguous lambdas,
Unicode/CRLF byte ranges, metadata persistence and deeply nested source that must
produce bounded partial IR without rejecting the method graph.

Native runtime tests exercise inspector entry, actual target-button navigation,
source-row highlight movement, flow camera/history restoration, unchanged graph
navigation, stale generation rejection, source snapshots without disk files,
old-cache messaging and minimum-window bounds and actual source scroll restoration through A → B → A
visits. Review/testing found and fixed
lost labeled-block siblings, eager nested conditional calls, effectful switch
ordering, a fast initial-fit race, clipped dynamic footer controls and the C#
inspector layout regression. A final long-source test exposed shared scroll-state
ownership across visits; per-visit scroll identities and captured restore offsets
now keep each history entry independent.

## Checks

- 250 Rust tests passed across regular and real-toolchain runs, with the two
  unavailable private fixtures excluded. This includes 11 shared semantic/layout tests, 3 flow
  inventory tests, 2 real TypeScript flow tests and 5 real Java flow tests.
- Two archived private-project checks were explicitly excluded because their
  `CODEGRAPH_COMPILED_METADATA` / `CODEGRAPH_RECURRING_BILLING` inputs are unavailable.
  These are the same existing private acceptance fixtures documented by earlier work.
- Python suite: 49 passed, including six real JVM inventory checks.
- TypeScript Node suite: 45 passed.
- All-target Clippy with warnings denied and package-scoped formatting passed.
- CodeScene individual source reviews prompted focused refactoring; the final
  safeguard passed without regressions. Type-only module wrappers have no score.
- SonarMCP remains inactive: no local `.sonar/config.json` is configured.

## Performance and invariance

[TypeScript comparison](typescript-export-comparison.json) uses 200 methods and 200
callbacks, producing 602 nodes, 800 edges and 601 flows. Before/after node, edge,
entry-point and diagnostic JSON is identical. Median extraction increased from
92.1 to 155.2 ms; serialized output grew from 625,477 to 1,560,581 bytes, with
935,086 bytes of flow IR. This synthetic measurement describes export overhead,
not a whole-project refresh benchmark.

[Java comparison](../performance/flow-java.json) uses the same real Maven reactor
paths, compiled artifacts and graph before/after changing only the source inventory.
All 10 nodes and three relationships are identical. The number of subprocesses
remains 14. SQLite grows from 110,592 to 118,784 bytes; flow IR is 9,183 bytes.
Warm refresh samples were approximately 5.9 s before and 5.6 s after; concurrent
machine load makes this unsuitable as evidence of a speed improvement.

Selected-chart results are cached independently of the graph. Source uses virtual
rows, while chart cards and connectors outside the camera viewport are culled.

## Packaging and visual evidence

The existing Rune checkout has a pre-existing formatting diff and was preserved.
Packaging ran from the isolated staging root established for TASK-20 using the
clean pinned Rune revision, current CodeGraph source and current analyzer artifacts.
The unchanged provenance gate, release build, dependency/license bundling, ad-hoc
signing and strict signature verification passed. `dist/CodeGraph.app` and the ZIP
contain the resulting package; ordinary packaging against the dirty checkout is
not claimed to pass.

[Packaged smoke metadata](packaged-smoke.json) confirms real Maven analysis creates
three valid flow symbols and a cached source file. The packaged flow scene also
rendered with PATH restricted to system tools, exercising bundled Graphviz.

[Native captures and reproduction](../ui/function-flowcharts/README.md) cover light
and dark appearance at 1420×900 and 1050×680, plus a restricted-PATH packaged capture.
The scene is explicitly synthetic; real compiler and Reactor acceptance are separate.
Physical trackpad and VoiceOver validation were not performed.
