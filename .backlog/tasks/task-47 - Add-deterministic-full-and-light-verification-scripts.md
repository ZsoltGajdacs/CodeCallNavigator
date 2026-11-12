---
id: TASK-47
title: Add deterministic full and light verification scripts
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T10:37:00Z'
updated_date: '2026-11-12T11:37:00Z'
labels: []
dependencies: []
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Repeated ad hoc verification runs use debug mode for release-only benchmarks and omit profiling workspace prerequisites. Provide reliable full and light entry points with explicit prerequisites, bounded execution and recorded results.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Full verification checks formatting, lint, ordinary and compiler acceptance tests, correctly configured release benchmarks, Python and TypeScript checks, CodeScene and local packaging without silent required skips.
- [x] #2 Light verification handles minimal changes, scopes formatting to project-owned changed files and avoids runtime checks for documentation-only changes.
- [x] #3 Scripts prepare an isolated profiling snapshot, use matching engine binaries, report missing prerequisites and timeouts clearly, and return nonzero on failed or incomplete required checks.
- [x] #4 Usage and project verification guidance consistently point to the scripts; orchestration and failure paths have automated coverage.
- [x] #5 Both modes reclaim appropriate build/unused artifacts with active-use checks, preserve reports/current packages and useful light dependency caches, and report cleanup failures and reclaimed space.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Inspect existing verification and profiling snapshot helpers. 2. Implement shared Python verification runner and full/light shell entry points with deterministic stages and prerequisite validation. 3. Add orchestration, process cleanup and quality integration regressions. 4. Update guidance, run scripts and required quality checks, record any environment limitations.

5. Add mode-specific cleanup: light incremental state, full debug/release profile caches, run-local profiling/scale data and rotated app backups; preserve reports/current package and check active artifact usage before removing anything. Use Cargo native age-based global dependency GC and disable incremental compilation during verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented full/light entry points with stage manifests, scoped format/syntax checks, serial regular tests, release all-ignored acceptance, SQLite profiling snapshots, local packaging, exact hash-locked Python test dependencies and actual stdio CodeScene reviews/safeguard. 25 orchestration/protocol regressions pass. Independent review found and verified fixes for supported-language file forwarding and denied MCP process-group cleanup. All source health reviews score 10; safeguard passes 26 eligible files. Full preflight correctly reports three unconfigured inputs (TypeScript modules, private Maven checkout, profiling source) before executing expensive checks. End-to-end native light run is in progress; earlier sandbox runs correctly exposed missing jsonschema and terminal access limitations.

Native light run reproduced an existing test synchronization race (process_protocol cancellation fixture parsed child.pid while empty). Fixed test-only marker publication with temporary-file+atomic-move; all 4 focused protocol tests now pass. Added hash-locked verification Python requirements and isolated ignored .venv-verification; wrappers select it automatically and preflight checks exact package versions. Located existing local inputs for full verification and started a configured native full run; benchmarks will use an SQLite snapshot of the source, never the live store.

User expanded scope to build/unused artifact cleanup. Measured target/debug 8.1GB (incremental4.2GB), release1.3GB, dist4.7GB. Chosen light retention preserves compiled dependency caches; full reclaims debug/release after checks. Retain current packages and one rollback per app prefix. Cleanup will be logged, fail closed on active/uninspectable artifacts and run on failed attempts after build stages as well as success. Existing native full run remains active; no current build artifacts deleted.

First configured native full run completed 577 release tests successfully, including all CPU/GPU benchmarks on the isolated saved workspace; one engine-lib test failed solely because it expected manual CODECALLNAVIGATOR_COMPILED_METADATA. Replaced that test-only prerequisite with the existing owning Maven build::compile operation (explicit supplied metadata remains supported). Its real compiled-output regression then passed against all13 source contexts (10921 declarations/17443 relationships). Cleanup unit/integration cases pass with no real deletions yet; source health cleanup10.0. Started final configured full run that includes post-packaging artifact cleanup.

Cleanup implementation/review complete: full/light always-finalizer runs after verification failures, reports active running stages, disables new incremental state, checks native lsof usage, rejects symlink escapes, preserves own reports/current packages/one rollback per app prefix and supports --keep-artifacts. Full uses explicit dev/release Cargo profiles only after reached build stages; missing-preflight runs retain existing build caches. Source/target build dir overrides are pinned to this checkout. 48 focused verification/protocol/cleanup tests passed before adding one live-running-status regression (that regression also passed separately); final combined rerun in progress. All modified source reviews remain10.0 and latest safeguard passed29 eligible files. Native final full run is at compiler acceptance; cleanup has not run yet.

Final configured native verify-full.sh PASSED all50 stages. Regular Rust494 passed/84 ignored; full release578 passed/0 ignored; Python154+26 passed; TypeScript112 passed/no skips; focused verifier49 passed. All29 source reviews10.0, safeguard29/29/no findings. Storage250k nodes/2m edges met targets; all CPU/GPU benchmarks used snapshot and passed. Packaging/signature/engine startup passed. Cleanup succeeded without blocked/uninspectable paths, removed estimated20.45GiB (Cargo dev14.6GiB/release1.3GiB plus old backups/scratch); target now8KiB, dist584MiB, current packages/reports/one rollback per app name retained. Evidence evidence/verification/2026-11-12/README.md; detailed local logs/results/cleanup under /private/tmp/codecallnavigator-verify-full-final. No waivers, commit, push or PR.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added deterministic full/light verification entry points with explicit prerequisites, scoped formatting, lint/tests, correct release benchmarks and isolated profiling snapshots, CodeScene, bounded execution and durable reports. Added mode-specific safe artifact cleanup, native usage checks, retention and explicit keep option. Fixed test fixture setup races/manual metadata prerequisites found by full verification. Native full run passed all50 stages,578 release tests and all packaging/quality gates; cleanup removed estimated20.45GiB. Documentation and evidence updated; no commit/push.
<!-- SECTION:FINAL_SUMMARY:END -->
