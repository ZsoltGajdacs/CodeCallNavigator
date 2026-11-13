# Desktop freshness and explicit adoption

These images come from actual TypeScript compilation, the public CLI observer and
snapshot commands, production Rune controls, and native Metal offscreen rendering.
Temporary fixture checkouts and the SQLite profiling store are isolated. The
folder editor intentionally points at another temporary project while the initial
graph, workspace header and monitor remain bound to the loaded project. The
journey asserts the header before/after folder editing, after adoption and after
a real project switch.

| Image | Observed state |
| --- | --- |
| `stale.png` | Saved body edit observed as stale; existing selected method remains displayed. |
| `reload-offered.png` | External analysis publishes another generation; adoption requires the Reload control. |
| `adopted.png` | Selected symbol/history and the deliberately nondefault camera survive adoption. |
| `unverifiable.png` | The real compiler graph is republished as a supported legacy artifact without input provenance; observation reports `missing_analysis`. |
| `deleted-selection.png` | Another compiler refresh removes the selected method; obsolete selection/focus disappear and camera remains stable. |
| `fitted-view.png` | An explicit user Fit shows the remaining graph at a readable scale. |
| `failed-light-compact.png` | Removing the saved artifact terminates observation with contextual failure; the loaded graph remains visible and Retry stays available at 1050×680 in the light theme. Restoring the file and clicking Retry recovers. |

The first five camera-preservation captures intentionally retain the test's pan
and zoom even when that positions part of the graph under the fixed controls.
`fitted-view.png` uses an explicit user action after those assertions. Freshly
compiled TypeScript observations can also be unverifiable because the fixture
records unmapped dependency inputs; they are not converted to matching status.

Reproduce from the workspace root with the engine built:

```sh
CODECALLNAVIGATOR_TS_NODE_MODULES=/path/to/pinned/node_modules \
CODECALLNAVIGATOR_FRESHNESS_UI_CAPTURES="$PWD/evidence/freshness/task61-desktop" \
cargo test --locked --release -p codecallnavigator --bin codecallnavigator \
  actual_cli_freshness_reload -- --ignored --nocapture
```

Metal access outside the macOS sandbox is required when captures are requested.
The final capture journey passed in 5.77 seconds. The capture helper invalidates
its font atlas for each new offscreen renderer so subsequent images upload the
same fonts correctly; no image pixels are edited afterward.

Focused release checks additionally cover actual CLI cancellation and capability
renegotiation after an in-place upgrade, malformed/contradictory wire frames,
all presentation focus variants, linked-root remapping with identical portable
bytes, delayed layer assignments, and real SQLite retention. Retention tests
hold the existing writer lock to supersede an active and a pending request,
then verify only the latest accepted descriptor/cache marker is persisted while
newer camera and opaque settings survive. A deliberate cache-directory failure
preserves the valid displayed CLI graph and reports no successful marker.

The root ordinary release suite passed 240 tests with nine explicit ignored
acceptance/benchmark tests. Independent source review and focused desktop/MCP
validation passed. Full workspace verification passed all 72 stages; the parent
record retains [results and quality logs](../consumer-adoption/verification/results.json).
