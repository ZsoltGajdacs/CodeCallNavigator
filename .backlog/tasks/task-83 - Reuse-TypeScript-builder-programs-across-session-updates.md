---
id: TASK-83
title: Reuse TypeScript builder programs across session updates
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-18T14:08:00Z'
labels: []
dependencies:
  - TASK-82
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Current TypeScript worker serves one request and loses compiler incremental state. Slice 5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Repeated updates use project compiler builder APIs with noEmit
- [x] #2 Compiler configuration and reference changes invalidate programs correctly
- [x] #3 Real supported compiler versions preserve graph semantics and cancellation
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reuse TypeScript's createSemanticDiagnosticsBuilderProgram in the existing Node worker. Start each update with fresh configuration parsing and read observations. Reuse only builders for the same canonical project, compiler/resource identity and reference topology, with native resolution and source version checks. Preserve noEmit and source project-reference redirects.
2. Give preparation and extraction requests an explicit versioned response with a matching sequence and status. Keep successful and failed updates distinct, preserve current input guards and artifact generation, and retire the worker on cancellation or protocol/toolchain changes. Bound worker lifetimes and obsolete output files with existing engine ownership.
3. Retain the worker across edits inside one selected engine session. Keep one-shot cleanup explicit before publication. Scope invalidation to actual inputs, preserve unrelated selected builds, and have source-config/reference additions and removals trigger fresh ownership/metadata verification.
4. Extend real TypeScript 5.9/6.0 tests with cold-versus-warm graph parity, measurable native builder reuse, inferred signature and resolution changes, reference updates, no emission, and cancellation/cleanup. Run the computed selector and Code Health. Continue locked desktop deferral until the final Slice 6 checkpoint.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Started after Slice 4 commit 5575459. The existing compiler worker uses a versioned update/analyze boundary and TypeScript native affected diagnostics. The existing reference test also exercises a changed topology in the same process.

First selected run changes-aqkim7d3 passed Cargo check and stopped at Clippy on an unused import. Removed it and adopted the independently reviewed session preparation/extraction helpers (Code Health 10.0). This run did not reach tests.

Run changes-y2_nic1h with 6.0.3 passed 47 stages through all 15 refresh cases at the agreed pre-desktop guard. Run changes-1p306i0j with 5.9.3 passed unit/integration and Rust acceptance, but fresh-session published its third attempt with a digest different from the independent cold reference. Both pinned compiler sources pass the old Program to native resolution when using the root-name builder overload. Construct a fresh native Program to record all current resolution probes and pass it to the public builder overload that retains diagnostic state only. The acceptance case now captures mismatched warm/cold artifacts before asserting digest equivalence. All prior cleanup reports recorded zero errors.

Final corrected matrix: changes-e3l74_fh for TypeScript 5.9.3 and changes-0w6pt3hy for 6.0.3 each passed 48 stages, including all 15 refresh journeys, the native affected-file integration, the updated reference-topology case and all six Node worker cases. Native cutoff interrupted C# cleanup acceptance and left eight later stages not run for TASK-84. All category cleanup reports recorded zero errors. TypeScript 5.9.3 fresh-session attempts 2/3/5 checked 130/1/1 native diagnostic items and started 1/0/0 workers, with exact cold artifact equality.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Keep one owned native TypeScript builder in the engine session. Each update reads a fresh Program and reuses the compiler’s diagnostic state where configuration and references match. Verified 5.9.3 and 6.0.3 with affected-file savings, changed reference ownership, output-free compilation, exact cold graph parity and cleanup. Native checks are recorded for TASK-84.
<!-- SECTION:FINAL_SUMMARY:END -->
