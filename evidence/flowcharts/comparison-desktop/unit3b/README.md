# Comparison desktop Unit 3B checkpoint

Independent verification passed 63 distinct tests: 48 comparison client/controller/UI
cases, 12 ordinary flow controls, and three real compiler-backed C# cases in release
mode. No tests were ignored or skipped. The [independent report](independent-unit3b.json)
records commands, prerequisites, engine identity and the 56 source hashes checked
before and after execution. Independent source review approved the same frozen scope.

Comparison tests exercise actual Rune controls backed by Git artifacts and the packaged
CLI: catalog/search/paging, explicit manual counterparts/reset, moved refs requiring
Reload, missing/unavailable/partial states, cancellation and owned query cleanup,
endpoint-local selection/highlight, local zoom coordinates, target/Back history,
rendered source scroll and first-fit camera preservation. Git fixtures use explicit
tag refs; a later unrelated HEAD commit cannot change the saved after endpoint.

The minimum-window checks assert full control and visible rectangles at 1050x680
in both themes with a simulated native titlebar inset. These are headless UI checks;
actual desktop rendering and OS pointer delivery are not established by this checkpoint.
Native comparison acceptance and full verification belong to later task gates.

Both directly regenerated CodeScene reports contain 39 reviews scoring 10 and a
passing safeguard over all 56 modified source files. Scoped Clippy with `-D warnings`
passed; the logs retain existing warnings from the vendored Rune dependency.

The first independent C# attempt ran inside the sandbox and failed three analysis
setups because the MSBuild host could not connect to its pipe. Its bounded run
unwound normally; all known owned descendants were absent before retry. The three
cases then passed outside the sandbox in 27.35 seconds. Failed checkpoint and backend
outputs are retained to distinguish this prerequisite failure from feature assertions.

Red/green logs preserve the Back/source-scroll mount-order regression, camera
remeasurement, ordinary target-help compatibility and disabled unavailable Fit actions.
Temporary tracing was removed before freezing. Whitespace at the end of copied text
logs is normalized; fixture/source bytes and JSON reports are copied unchanged.
