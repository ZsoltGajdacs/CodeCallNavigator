---
id: TASK-104
title: Replace Compare revision fields with commit lists
status: Done
assignee:
  - '@codex'
created_date: '2026-11-23T06:41:00Z'
updated_date: '2026-11-23T11:25:00Z'
labels: []
dependencies: []
references:
  - TASK-103
type: feature
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow up TASK-103 with native Git history selection and explicit worktree refresh.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Base and After browse local commits with full messages, with Worktree available on After.
- [x] #2 Confirmed Worktree refresh compares the exact new graph and preserves the prior result on cancellation or failure.
- [x] #3 Deterministic, native and CodeScene verification pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reuse the core Git runner for stable paged history and add typed native revision selection.
2. Route confirmed Worktree refresh through manual analysis and replace comparison only after validation.
3. Update acceptance and run verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added shared Git history tied to the selected checkout, native commit controls, and confirmed Worktree refresh with generation-bound publication. Updated real Git and native C# acceptance coverage. First compilation found private helper use and a driver dispatch type mismatch, both corrected. Development run changes-zx5ruwmw passed seven stages before the final structural cleanup.

Final run changes-pnsazudk passed formatting, Clippy, every integration stage, real Git history and desktop picker regressions, and package signature/engine/MCP checks. Packaged Rust acceptance stopped at the snapshot controller fixture because it set comparison revisions before the Compare history lifecycle initialized; the fixture now selects them after opening Compare. The previous integration run's symbolic-tag expectation was updated to prove pinned commit selection while the low-level moving-ref regression remains.

Native dark replay changes-q0xc0ap_ passed the commit message, keyboard diff, pinned tag, snapshot binding, owned query cancellation and retry journeys. The light replay reached an unstaged C# edit. Its driver used the broader build-review guard before the Refresh worktree control enabled, and the outer 30-second wait could interrupt compilation. Updated the driver to use the exact control guard and the light replay to use its configured timeout. Prior native runs exposed and fixed macOS Tab traversal and a late outline result overriding Functions.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compare now browses pinned local Git commits on both sides, shows full messages and refreshes Worktree through the existing build confirmation. Native Compare passed in both themes. The light run proved that cancelling confirmation preserves the previous pair and that rebuilding displays the uncommitted C# edit from the exact published generation.

Final verification changes-l6dhe_xx passed 112 stages, including CodeScene, performance and native flow expansion. Its C# desktop gate was deferred while macOS was locked. The user explicitly accepted the passing C# desktop gate from changes-q0xc0ap_ against the same production code.
<!-- SECTION:FINAL_SUMMARY:END -->
