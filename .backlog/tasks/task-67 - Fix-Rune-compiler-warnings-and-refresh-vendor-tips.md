---
id: TASK-67
title: Fix Rune compiler warnings and refresh vendor tips
status: Done
assignee:
  - '@codex'
created_date: '2026-11-16T07:22:00Z'
updated_date: '2026-11-16T07:47:00Z'
labels: []
dependencies: []
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Current macOS compilation reports eleven Rune warnings. Refresh Git vendor sources and reassess integration patches against upstream while preserving reproducible bootstrap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reported Rune warnings are resolved in debug and release compilation
- [x] #2 Git vendor pins match fetched upstream default tips and retained patches are justified and checksum pinned
- [x] #3 UI verification, focused patch checks and quality results are recorded under the user-approved UI-only scope
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Inspect/reproduce warnings and verify recorded vendor diffs. 2. Fix precise cfg/pattern and nested source warning issues, refresh Rune to fetched dev and audit integration hunks; check other vendor tips. 3. Update pins, checksums, nested patch artifacts and canonical rationale. 4. Run patch migration, UI unit/integration and packaged acceptance, focused GPU regression, debug/release compilation and independent review; broader full verification excluded by user instruction.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planner/explorer completed. Refreshed Rune dev to 631e8a67; other Git vendor default tips unchanged. All existing integration hunks remain needed. Fixed reported warnings with narrow cfg/pattern cleanup, including Windows/wasm render-helper availability. Added required accesskit_macos workspace override. All 13 dependency migration tests passed. User limits remaining runtime verification to UI; full preflight previously unavailable due unset TS/private-Maven/profile inputs. Independent review caught and resolved wasm cfg issue.

User explicitly limits verification to UI because other components are unaffected. Initial UI run: 191 unit tests passed; integration preflight required TS modules. Located compatible installed TS6/React19/Next16 modules for rerun. Latest upstream introduced further warnings in Rune accessibility/backdrop and nested Parley/tiny-skia; resolving those without lint suppression, preserving nested recorded patches. Cargo.lock contains only required accesskit_macos path override; unrelated resolver churn was removed and locked metadata passes.

Final UI unit+integration passed: 191 unit and 251 integration tests, zero failures. Source compiler warnings absent; only expected three stale precompiled-shader notices remain. Timings: setup 9.109s, compile 167.349s, test execution 46.975s, cleanup 3.165s; owned cleanup passed and retained no build outputs. Evidence target/verification/task-67-ui-final. All 13 patch migration tests passed, exact Rune and CallAtlas checkout diffs/checksums verified. Independent review found no remaining issues; nested patches reproduced byte-exact against published archives. CodeScene reviewed changed sources; findings are pre-existing upstream complexity, fixture score 10, root safeguard passed (1 eligible file). Sonar unconfigured. UI acceptance running.

Packaged UI acceptance passed all 10 stages: setup 6.739s, release packaging 102.159s, signature/engine checks 0.304s, native C#/comparison/expansion journeys 116.047s, cleanup 2.498s. Cleanup reclaimed 1,556,245,036 bytes; 179,999,781 bytes of packages retained. Release has no Rust source warnings, only expected shader fallback notices. Evidence target/verification/task-67-ui-acceptance. Focused Rune GPU regression running.

Focused Rune segment GPU regression passed (1 test, 2189 filtered); compile 247s, execution 1.10s, wrapper total 262.355s, cleanup 0.719s reclaimed 6,968,344,868 bytes. Vendor Cargo.lock unchanged. Standalone upstream test harness retains two unrelated test-only dead-code warnings plus manifest lint warning; application debug/release source warnings are resolved. Evidence target/verification/task-67-rune-gpu. All authorized UI-only verification complete; no commit/push performed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed reported Rune warnings plus new source warnings introduced by refreshing Rune dev to 631e8a67. Retained integration patches remain necessary; all replacement/nested patches and checksum pins reproduce exact sources. Added required upstream macOS accessibility override without unrelated lock churn. Passed 13 migration tests, 191 UI unit tests, 251 UI integration tests, all packaged UI acceptance stages and focused GPU pixel regression; cleanup passed. CodeScene reviewed changes and root safeguard passed. Wider verification excluded by user instruction. Changes remain on the current branch, uncommitted.
<!-- SECTION:FINAL_SUMMARY:END -->
