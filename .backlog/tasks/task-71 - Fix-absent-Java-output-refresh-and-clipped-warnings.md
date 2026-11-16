---
id: TASK-71
title: Fix absent Java output refresh and clipped warnings
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T16:58:00Z'
updated_date: '2026-11-16T17:44:00Z'
labels: []
dependencies: []
type: bug
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Automatic refresh rejects the test-only spec module because its exact owned target/classes directory is absent; Analysis details clips warnings beside wide controls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Exact certified missing output roots permit scheduling while nested unknown inputs remain rejected.
- [x] #2 Freshness warnings and actions remain readable and usable in constrained Analysis details layouts.
- [x] #3 Regression tests and required quality and behavioral verification pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reproduce missing exact Java output-root rejection and clipped detail warnings with focused regressions.
2. Permit only exact certified missing output roots; preserve nested-input, source/config overlap, binding and symlink safeguards.
3. Stack full-width wrapped detail messages above separately wrapping actions and wrap the legend.
4. Verify strictly related behavior: core ownership suite, real capture/filter regression, Rune warning geometry and rendered captures, automatic-refresh acceptance, changed-source Code Health, inventory/format/diff checks.
5. Stop unrelated verification per the latest user instruction, safely clean owned runs, and record honest outcomes plus the one-time manual-refresh requirement.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner completed read-only scope and invariant review. Explorer is checking existing runtime and verification boundaries. Sonar is inactive: .sonar/config.json is absent.

Red regression confirmed: exact missing owned @repo/spec/target/classes is rejected by only_bytecode_inputs (target/verification/task71-iteration/core-red.log). Real capture/filter and Rune geometry regressions are being run before production changes. Full prerequisites validated with local TypeScript modules, private Maven checkout and saved profile source; initial missing environment selections were resolved without changing configuration.

Implemented exact missing owned-output-root exception beneath existing certification and overlap checks. Detail warnings now wrap across full width above separately wrapping actions; legend wraps. All three regressions failed before fixes, then core ownership suite (8 cases) and UI geometry test passed at 600x800 and 1050x680. Root inspected both offscreen Metal captures; this proves rendered paint/layout, not physical-window presentation. Independent reviewer found no actionable issues. All six modified Rust files scored Code Health 10.0; new UI Large Method finding was resolved with a cohesive assertion helper. MCP safeguard passed 6 eligible files with no issues. Iteration cleanup passed and reclaimed 6.49 GB. Evidence: target/verification/task71-iteration/iteration-summary.json and captures/. Full verification is running under the tester role.

First full run stopped at Clippy: pre-existing nested conditional in touched src/ui_tests.rs click helper violates collapsible_if under -D warnings. Cargo check passed; later behavioral stages were not run. Cleanup passed. Applying the suggested behavior-preserving let-chain collapse, then rerunning full to a fresh evidence directory.

Full-r2 passed static checks, unit and integration categories, packaging, Rust acceptance, automatic-refresh acceptance and C# cleanup acceptance, then native desktop setup failed because AeroSpace did not enumerate the owned test window. An unchanged standalone retry failed similarly; no application assertion or window layout mutation ran. User clarified the desktop was locked. Current console appears unlocked; native gates will be retried sequentially after unaffected performance verification. No application or window-manager configuration changes were made. Final CodeScene CLI invocation was initially rejected by automatic approval review as source export; retry cited explicit user-supplied AGENTS requirements and was approved.

Latest user instruction: Only run verification that is strictly related to the changes. This supersedes the earlier repository full-verification scope for this task. Stopping the broad acceptance continuation and launching no further unrelated gates. Related checks already passed: all eight refresh-input ownership cases including the two new regressions, new Rune layout regression at both sizes, two inspected offscreen Metal captures, packaged automatic-refresh acceptance, changed-source Code Health 10.0 and safeguard, format/inventory/diff checks. Full-r2 is not claimed as passed; it stopped at locked-desktop native setup, which subsequently passed unchanged after unlock. Only owned cleanup and final task bookkeeping remain.

Unrelated continuation stopped on user request; no more gates launched. Its current gate reported interrupted group cleanup unavailable, then the standard always-run category and final cleanup passed after establishing owned paths were idle; post-stop inventory found no owned continuation/TMPDIR/native-gate processes. The standalone failed native retry fixture was ownership-validated, proven idle and relocated under target/verification/task71-native-retry/retained-fixture (96,112,722 bytes); original temporary path is absent. Failed/interrupted native diagnostics are retained as ignored evidence; Rust build outputs and temporary fixtures from successful/aborted categories were cleaned. No commit, push, installation or publication performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed automatic-refresh eligibility for exact absent certified Java output directories while preserving nested-input and ownership safeguards. Analysis details now gives warnings full width above wrapping action controls, and the relationship legend wraps. Added pure, filesystem and real Rune regressions, observed each failure before fixing it, and verified the related ownership suite, geometry at two sizes, inspected Metal captures, automatic-refresh acceptance, changed-source Code Health 10.0/safeguard, inventory, formatting and diff checks. Verification scope was narrowed by the user; the interrupted broad full run is not claimed as passed. Existing incomplete artifacts need one manual refresh in the rebuilt app. Evidence: target/verification/task71-iteration/iteration-summary.json, target/verification/task71-quality.json and task71-acceptance-continuation cleanup reports.
<!-- SECTION:FINAL_SUMMARY:END -->
