# Project cache clearing verification

Implemented for TASK-36. The desktop calls the engine machine protocol; both it
and `codegraph-engine clear-cache --project PATH` use one core operation under
`AnalysisLock`. Only inference.sqlite3 and its SQLite sidecars are removed.
Canonical graph, configuration, sources, navigation and build outputs remain.
Protocol version 1 gains additive clear_cache/cache_cleared messages; older engines
reject the unsupported request explicitly.

## Passed

- `cargo check --locked -p codegraph --all-targets`
- `cargo test --locked -p codegraph --all-targets`
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`
- `cargo test --locked -p codegraph-core -p codegraph-engine --all-targets`: passed on full retry.
- Core cache clearing regressions: corrupt cache and sidecars, idempotence,
  preserved unrelated files, held lock, symlink and hard-link rejection.
- `cargo test --locked -p codegraph-engine --bin codegraph-engine --test clear_cache`:
  5 entrypoint tests and 4 real executable tests, including CLI/protocol success,
  absent cache, invalid projects, lock conflicts and unsafe cache paths.
- `cargo test --locked -p codegraph --bin codegraph ui_tests::cache_clear -- --include-ignored`:
  3 native runtime tests covering adjacent controls, busy guards, real engine
  clearing, preserved visible graph, success feedback, failure and retry.
- `cargo test --locked -p codegraph --test incremental_inference cached_inference_matches_full_analysis_after_compiler_facts_change -- --ignored --nocapture`:
  real TypeScript compiler acceptance verifies warm cache hits, CLI clearing,
  preserved artifact, zero subsequent hits and identical canonical graph bytes.
- CodeScene reviews: modified source files have no findings; scored files 10.0.
  Pre-commit safeguard passed with 16 eligible files checked and no regressions.
- `python3.13 tools/package.py`: packaged and signed `dist/CodeGraph.app`.
- `git diff --check`.

## Full validation limitations

`cargo test --workspace --all-targets -- --include-ignored` failed in four
unchanged desktop tests: canvas_gpu_frame_completion,
dense_canvas_frame_preparation and saved_canvas_frame_preparation explicitly
require release mode; forced_cancellation_reaps_engine_and_its_separate_child_group
failed its detached-process cleanup assertion in the macOS sandbox. Cargo stopped
at that test binary, so later ignored workspace suites were not verified by this
command. These failures are outside the cache-clearing change.

An additional core/engine test run encountered the unchanged process deadline test
zero_deadline_never_starts_process_and_cancellation_takes_precedence failing its
marker assertion; its immediate isolated retry and subsequent full core/engine
retry both passed.

Sonar is inactive because this project has no `.sonar/config.json`.
The change is committed locally without pushing. TASK-36 remains In Progress
because the full validation limitations have not been waived.
