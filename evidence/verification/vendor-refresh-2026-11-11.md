# Vendor refresh, 2026-11-11

Fetched all three Git source dependencies and queried their upstream default branches:

| Source | Branch | Verified tip |
| --- | --- | --- |
| Rune | dev | bba30a61cfcec6683e3fdedf156a92dae89d8b04 |
| CallAtlas | main | 916c2419c0cdb91b5b86b84df4842dadb17bb6b4 |
| java-callgraph2 | main | 77f566d0878958bab929b4cdeafcc2c762ff555e |

Only Rune advanced. Before switching revisions, its complete diff and CallAtlas's
complete diff were verified against their checksum-pinned patches. Rune's patch
was rebased, preserving new visibility callbacks and moving observer wiring to
the upstream virtual desktop module. All migration checkpoints now target the new
base; see `patches/README.md` for application order and old-base migration.

The combined patch makes `CommitMeter::handler` and `frame_ends` debug-only,
matching their callers. Three additional warnings introduced upstream were fixed
by narrowing module-local helper visibility. Cargo.lock gained two already
resolved Rune dependency edges (`skrifa`, `windows-core`). Downloaded investigation
binaries retain their separately pinned versions.

## Verification

- Final `cargo check --locked -p codecallnavigator --all-targets`: passed, no warnings.
- `cargo test --locked -p codecallnavigator --all-targets`: 329 passed, 69 ignored.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`: passed.
- `python3.13 tools/bootstrap.py`: passed (CallAtlas and Java adapters rebuilt).
- `python3.13 tools/package.py`: passed; release engine and desktop built without
  compiler warnings, signed application and ZIP produced in `dist`.
- `python3.13 -m unittest discover -s tests -v`: 76 passed.
- Dependency patch tests: 7 passed, including migration and unknown-edit rejection.
- All 4 debug input observer regressions passed.
- Release `ui_tests::canvas_performance::dense_canvas_frame_preparation`: passed.
- Independent implementation review and patch/checksum verification passed.
- `git diff --check`: passed.

## Limits

The exact required `cargo test --workspace --all-targets -- --include-ignored`
command fails because three ignored performance tests explicitly require release
mode; two also require a copied saved profiling workspace. A continuation
excluding those three passed compiler-backed ASP.NET, flow, incremental inference,
Maven and Gradle checks, then stopped at the private billing fixture test:
`recurring_billing_retains_duplicate_generated_declarations` requires
`CODECALLNAVIGATOR_RECURRING_BILLING`, which is unset. Subsequent acceptance
binaries were not run by that command. Full workspace acceptance is not claimed.

CodeScene reviewed the six source files changed by conflict resolution or warning
fixes. Scores: app 7.32, window 8.39, virtual desktop window 8.96, commit meters
9.53, motion flights 7.61, press repeat 10. Existing findings concern unchanged
upstream logic; review found no reason to broaden this vendor update into those
refactors. The root pre-commit safeguard passed with no eligible source files
(the root tracks patches rather than vendor source). Sonar is unconfigured.

Task TASK-41 retains the full-validation limitation; no waiver was assumed.
