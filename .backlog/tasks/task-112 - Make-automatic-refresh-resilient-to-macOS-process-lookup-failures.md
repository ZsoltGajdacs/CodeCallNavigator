---
id: TASK-112
title: Make automatic refresh resilient to macOS process lookup failures
status: Done
assignee:
  - '@codex'
created_date: '2026-11-24T16:46:00Z'
updated_date: '2026-11-24T20:07:00Z'
labels: []
dependencies: []
type: bug
ordinal: 121000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Automatic refresh can abort when macOS denies process identity sampling. The local probe obtained usable sysctl identities for the same 576 live children.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Automatic refresh survives recoverable macOS sampling failures and matches manual refresh on the same saved inputs.
- [x] #2 Shutdown requires verified ownership and reports unresolved cleanup uncertainty.
- [ ] #3 Native macOS inventory avoids ps and the Linux platform behavior is preserved.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Run final deterministic verification and CodeScene review.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extract platform readers and implement native Darwin inventory and targeted identity verification through an SDK shim.
2. Share periodic recovery across desktop and engine while preserving strict shutdown proof.
3. Cover native cleanup, failure recovery, and desktop manual/automatic parity. Run selected verification and Code Health.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Approved plan recorded. Working tree was clean at b85f0f3. Desktop and engine now share recovery policy, with a real desktop parity and active opt-out scenario. Initial CodeScene review prompted extracting the active opt-out assertions into one helper. Native backend and regression verification are in progress.

Implemented native Darwin sysctl inventory and targeted birth identity through a small SDK shim. The desktop and engine use the shared recoverable sampling entry point. Strict stop preserves retirement, revalidates signal targets and reports unresolved ownership. Linux inventory and lookup were extracted with the prior behavior preserved by source review. Linux runtime verification was not run.

Development selector changes-1l32dm97 passed 7 stages. Final selector changes-6veb3d85 passed 47, failed 1, and left 16 unrun. Verified the real desktop automatic/manual graph generation match, active opt-out, native identity/exit, PID reuse, independently grouped descendant cleanup, unresolved cleanup without signaling, and all 15 dedicated automatic-refresh journeys. Cargo check, Clippy, signed packaging, C# desktop and cleanup passed. Independent CodeScene reviewed all 8 source files at 10.0 and the safeguard passed.

Final blocker is the Native Compare acceptance timeout at FlowFirst(Depth(SecondHopSource)). Baseline b85f0f3 records the same depth-two source timeout in TASK-111. Those Compare paths are outside this fix. Later acceptance and performance gates and the selector-level CodeScene stage were not reached. Final timing was 390.5 s compilation, 10.2 s setup, 1272.5 s test execution, 48.9 s cleanup, and 142.2 s other checks. Task stays In Progress pending complete required final verification.

The user manually verified the macOS refresh fix and requested closure and a local commit. This accepts closure with the recorded automated verification limitations. Linux runtime verification and the full final gate remain unchecked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Replaced macOS process enumeration and identity reads with sysctl. Desktop and engine share recovery from temporarily unavailable samples and require fresh ownership proof during cleanup. Manual verification, desktop automatic/manual graph parity, native cleanup, all 15 automatic-refresh acceptance cases, and CodeScene passed. The complete selector passed 47 stages before the previously recorded Compare depth-two source-preview timeout and left 16 stages unrun. Linux runtime verification was outside this run.
<!-- SECTION:FINAL_SUMMARY:END -->
