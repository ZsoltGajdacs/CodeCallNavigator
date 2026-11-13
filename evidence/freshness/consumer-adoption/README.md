# Desktop and MCP freshness adoption (TASK-61)

Desktop observes the loaded CLI project/root binding asynchronously. Matching,
stale, unverifiable and failed observations remain distinct. Editing the pending
folder does not retarget the monitor, Reload control or displayed workspace
identity. A newer saved generation offers explicit Reload; ordinary observation
does not analyze or automatically adopt it.

Same-project adoption preserves valid navigation, history and camera while
invalidating old generation/binding/epoch results. New projects reset the
workspace. Linked-root remapping preserves valid context but replaces hydrated
source addresses and rejects old bindings, even with identical portable bytes.
Deferred architecture assignments reconcile all layer/history/expanded targets
without late auto-fit or recentering.

The CLI snapshot owns displayed graph validity. One active/latest-pending worker
retains write-only profiling data; it never saves the session. Only an accepted
UI binding may install a cache marker and persist the current descriptor,
navigation and opaque settings. A local retention failure preserves the valid
displayed CLI graph. A cancelled publication can transiently leave a derived
active pointer, which is not a production graph-query source; marker validation
and the next accepted retention recover it.

## Desktop verification

The actual TypeScript compiler/CLI/Rune/Metal journey covers saved edits, external
publication, explicit Reload, selection/history/camera preservation, obsolete
layout output, deleted selections, unverifiable provenance, terminal failure and
manual retry, and project changes. Header identity is checked before and after
editing the pending folder, Reload and the eventual project switch.

Seven native GPU captures and reproduction commands are retained in
[task61-desktop](../task61-desktop/README.md), including a compact light-theme
failure/Retry state and an explicit fitted view. The parent visually inspected
the regenerated images. They are offscreen production Rune/Metal renders driven
by real CLI/compiler controls, not screenshots of a manually operated window.

Focused checks include real SQLite supersession/retention failure, actual CLI
linked-root remapping, capability retry after an in-place CLI upgrade, malformed
wire frames, all focus variants and delayed layer assignments. The ordinary root
release suite passed 240 tests; 75 projection integrations and release Clippy
also passed. Independent desktop validation covered 20 focused tests.

## MCP verification

`graph_freshness` forwards the negotiated one-shot CLI observation unchanged,
including generation, live revision, validity, coverage, limitations and timing.
Independent DTO validation binds it to both the query envelope and explicit
legacy-summary conversion. MCP owns no watcher, graph cache or graph algorithm.

The new real stdio/compiler regression first failed with method-not-found and
then passed. The complete MCP release suite passed 36 tests, including existing
comparison, cancellation, disconnect and malformed-wire coverage. All nine
affected source files passed CodeScene at 10.0. The independent dependency/source
boundary check passed with 89 packages, plus four boundary tests.

[mcp-legacy-cli.json](mcp-legacy-cli.json) retains the actual pre-observation CLI
capabilities, binary hashes and native MCP responses: existing status remains
usable, only the optional freshness tool returns `unsupported_capability`, ping
and generation-bound status continue to work, and the graph bytes stay unchanged.
Both processes shut down normally. An independent tester reproduced these facts
against the same binary hashes.

Reproduce the complete MCP process suite with the current CLI built:

```sh
CODECALLNAVIGATOR_TEST_ENGINE="$PWD/target/release/ccnavigator" \
CODECALLNAVIGATOR_TS_NODE_MODULES=/path/to/pinned/node_modules \
cargo test --locked --release -p codecallnavigator-mcp \
  -- --include-ignored --test-threads=1
python3.13 tools/mcp_boundary.py --root .
```

Full workspace verification passed all **72 stages**, including the complete
release acceptance suite, benchmarks, packaging/signatures, installed CLI and
independent MCP journeys. The pre-commit CodeScene safeguard passed all 46
eligible source files with no regressions. See
[verification/results.json](verification/results.json) and the adjacent logs.
