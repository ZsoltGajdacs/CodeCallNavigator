# Full/light verification and artifact cleanup

The final configured native `verify-full.sh` invocation completed all 50 stages
successfully on 2026-11-12. See [summary.json](summary.json) for machine-readable
counts. Detailed local numbered logs, results.json and cleanup.json are retained
at `/private/tmp/codecallnavigator-verify-full-final/`.

## Delivered behavior

- `verify-light.sh`: changed-project-file formatting/syntax, regular workspace
  check/Clippy/tests, both Python suites and real CodeScene reviews/safeguard.
  Documentation-only changes use static checks and cleanup without runtime tests.
- `verify-full.sh`: explicit prerequisite checks, pinned analyzer bootstrap,
  TypeScript tests, matching debug/release engine builds, all release acceptance
  tests including ignored tests, isolated saved-workspace CPU/GPU benchmarks,
  storage benchmark, local packaging/signature checks and CodeScene.
- Checkout locking prevents competing verification runs. Stages have time limits
  and logs; failures remain failures, later ordinary stages remain not run, and
  cleanup runs as an always-finalizer. Live stages report running.
- Full cleans dev/release profile artifacts only after build stages were reached.
  Light cleans incremental state while keeping compiled dependencies. Both
  remove their temporary profiling/scale stores and old rotated app backups,
  retaining current packages, reports and the newest rollback per app name.
- Native lsof usage checks and path/symlink guards protect active artifacts and
  paths outside the selected repository/run. Cleanup failures fail the run;
  `--keep-artifacts` explicitly retains artifacts for investigation.
- Verification disables incremental compilation and enables Cargo's own aged
  dependency-cache cleanup. No shared Cargo-cache directories are manually purged.

Setup, mode selection and flags are documented in
[the validation reference](../../../docs/reference.md#validation).

## Verification

- Ordinary Rust workspace: **494 passed**, 84 intentionally ignored by this stage.
- Full release workspace: **578 passed**, **0 ignored**, including all three
  release-only canvas benchmarks and real compiler/build acceptance tests.
- Python: **154** tests under tests/ and **26** under tools/ passed in the full run.
- TypeScript: **112 passed**, no skipped tests.
- Final focused orchestration/MCP/cleanup suite: **49 passed**. It covers real Git
  change selection, process failures/timeouts and cleanup limitations, lock
  contention, SQLite WAL snapshots, source-review coverage, mode/stage selection,
  relative input paths, symlink escapes, active-artifact retention and deletion.
- Native Metal offscreen benchmarks used an isolated SQLite backup of a real
  saved workspace. Storage benchmark covered 250,000 nodes and 2,000,000 edges;
  all defined acceptance targets passed. Offscreen timing is not displayed FPS.
- Local app/engine packaging, signature verification and packaged-engine startup
  passed. No release was published.
- All 29 reviewed source files scored **10.0**; CodeScene safeguard passed with
  29/29 eligible files checked and no findings. Sonar is unconfigured/inactive.
- Independent reviews found no remaining blocking issues. Diff whitespace check
  passed. No commit, push or pull request was created.

The full cleanup succeeded with no blocked/uninspectable candidates and removed
an estimated **20.45 GiB** of generated artifacts. Cargo reported 14.6 GiB of dev
and 1.3 GiB of release artifacts removed; additional removals were old app backups
and run-local benchmark stores. Current packages and rollback copies remain;
`target/` was reduced to 8 KiB and `dist/` to 584 MiB. Estimates count file sizes,
so they are not a measurement of physical free blocks on APFS.

## Reliability fixes discovered by the verifier

The engine cancellation fixture now atomically publishes its PID marker instead
of exposing an empty file before its contents are written. The private compiled
Java acceptance test now obtains fresh real-build metadata through its owning
Maven adapter when no explicit metadata fixture is supplied. Explicit supplied
metadata remains supported. This removes a previously manual full-suite input.
The shared build-label helper now uses a typed internal kind to satisfy the
safeguard without changing the public BuildTarget contract.

Earlier debug/sandbox or partially configured failures are superseded by the
successful final native full run. Required full inputs remain explicit; no checks
were waived or silently omitted to obtain this result.
