# Desktop CLI adoption acceptance

The production desktop obtains graph information through the public CLI. See
[ownership and lifecycle](../../../docs/desktop-cli.md).

## Native workflow

`ui_tests::cli_backend::actual_cli_keeps_graph_context_across_edited_root_and_drives_search_source_flow`
analyzes a real TypeScript fixture with the actual engine process, then drives
Double Shift search, method activation, relationships, explorer/source and
function-flow navigation. Editing the folder field preserves the loaded graph's
original query/source context. The test also returns from flow to the call graph.

The native Metal test passed. The [dark](dark-flow.png) and
[light](light-flow.png) captures were inspected after waiting for frame settling:
source highlighting, branch chart, toolbar and panels render without overlap.
Separate focused tests cover layer/filter selection parity, cancellation,
obsolete search activation, corrupt saved navigation and transport failures.

## Query responsiveness

[Raw measurements](cli-query-latency.json) use the existing 113,628,316-byte
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
[Earlier render-only samples](cli-render-latency.json) retain the pre-bounded
diagnostic payload for comparison; the table uses the newer bounded responses.

Release canvas benchmarks on Apple M4 Pro / Metal passed in the final full run.
Dense 300-node/6,000-edge CPU frame preparation medians were 2.85–3.56 ms across
the tested zooms and scales. The isolated saved workspace (19 visible nodes,
23 edges) measured 4.14–4.19 ms median preparation. GPU-complete offscreen frame
medians were 8.88/12.54 ms for dense scale 1/2 and 7.92/9.66 ms for the saved
workspace. These synchronous offscreen measurements are not displayed FPS.

## Saved-workspace compatibility

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
