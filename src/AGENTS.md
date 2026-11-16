# Desktop and UI testing

Follow [root guidance](../AGENTS.md) and [the testing strategy](<../.backlog/docs/testing/doc-12 - Testing-strategy.md>).

Prefer unit tests for geometry, presentation transformations, filtering and isolated
state policies. Exercise production functions with explicit data. Put behavioral
combinations here instead of repeating them through full UI journeys.

Use headless production Rune views and real input handlers for integration tests
of widgets, state, background jobs and navigation. Use real temporary files,
SQLite and CLI processes when proving those boundaries. A headless view test is
not native-window acceptance, even when it invokes the real CLI. Native display,
Metal, capture and packaged-window behavior require representative native journeys.
Keep performance tests separate from functional acceptance and run canvas timing
in release mode against isolated copied profiling data.

```sh
./verify-changes.sh
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

Run from the workspace root. Inspect the report-only command for required CLI, Graphviz,
compiler, display and release-mode prerequisites. These depend on the selected test,
not its file name. Category scripts clean owned Rust outputs and temporary fixtures
after each run, including failures; `--keep-artifacts` is diagnostic only.
Do not mock business behavior or introduce a fixture-query fallback into production.
Only documented integration fault probes may replace process/protocol failures.
State why a new expensive test needs this layer. Preserve generation binding,
stale-result rejection, camera/history state and graph preservation across failures.
A roughly one-minute warm unit loop is aspirational, never a correctness tradeoff.

Agents must use `./verify-changes.sh` for verification selection and
`./verify-changes.sh --run` for all test and verification execution. Do not
substitute raw test commands or manually selected category/component scripts.
Execute the complete computed plan; full verification is selected automatically
by checked-in rules, with no manual scope override. Use `--base REF` to include
committed changes. Run automatically selected full verification outside the macOS
sandbox. Report unavailable prerequisites as not run, never as passing checks.
