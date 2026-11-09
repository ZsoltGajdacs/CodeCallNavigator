---
id: TASK-15
title: Add hover source viewer and group file listings
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T10:26:00Z'
updated_date: '2026-11-09T10:46:00Z'
labels: []
dependencies: []
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Graph exploration currently requires leaving CodeGraph to read implementation. Add the approved full-file hover preview and group file listings without changing analyzers or persistence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Methods preview current source at the recorded line with selectable syntax-colored text and two-axis scrolling
- [x] #2 Directory and Namespace groups list distinct filtered source files across nested membership before pagination; files open with Back navigation
- [x] #3 400 ms opening and 300 ms closing delays, pin, Escape/Close, bounded resizing and graph gesture isolation work
- [x] #4 Asynchronous latest-request reads handle invalid paths, UTF-8, locations and 16 MiB limit; navigation and generation changes clear previews
- [x] #5 Runtime and source tests, light/dark default/minimum screenshots, Rust checks and Code Health checks pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Confirm projection, Rune overlay and worker boundaries. 2. Add typed group file membership and bounded asynchronous source model with syntax spans. 3. Implement transient hover lifecycle and resizable root popup with virtual rows. 4. Verify actual file reads and Rune interactions, capture screenshot matrix, run Rust/Clippy and CodeScene; record evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Recorded approved plan. Projection membership test first failed on missing group/file metadata, then passed after implementation. Added bounded real-file loading, cross-line Java/C#/TS highlighting, transient lifecycle and root overlay. Reviewer identified blank-area input pass-through, covered-node retargeting and late timer initialization; corrected all three. Ten real Rune runtime tests now pass, including idle timing, overlap, resizing, scrolling, Back, pin and obsolete-result regressions. Sonar inactive: no .sonar/config.json.

Final review resolved TSX constrained/default generic arrows, Java escaped text blocks, partial trigger overlap and resizing after window constraints. Thirteen source and thirteen popup runtime tests pass, including exact virtualized clipboard copy and Unicode horizontal reach. Final broad validation: 139 Rust tests passed (31 specialized ignored tests unchanged), 37 Python tests passed, Clippy warnings denied passed. Five native Metal screenshots inspected across dark/light and default/minimum sizes plus resized/scrolled listing. Release app/ZIP rebuilt and signature-verified. Final Code Health safeguard passed for all 17 eligible source files; final source components score 10. Evidence and chronological action log: evidence/ui/source-popup/README.md. Physical input hardware/VoiceOver remain unexercised; real Rune runtime and GPU rendering verified requested behavior.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added source previews on method hover and filtered pre-pagination file listings in Folder/Namespace modes. Popups support syntax-colored selectable source, line reveal, two-axis scrolling, pinning, dismissal and bounded resizing. Current-file reads are asynchronous, bounded and invalidated on navigation. Runtime/source/projection regressions, broad Rust/Python checks, Clippy, Code Health, native visual QA and signed packaging passed. Evidence: evidence/ui/source-popup/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
