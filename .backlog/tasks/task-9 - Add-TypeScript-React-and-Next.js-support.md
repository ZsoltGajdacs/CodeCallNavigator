---
id: TASK-9
title: Add TypeScript, React and Next.js support
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T05:46:00Z'
updated_date: '2026-11-09T06:08:00Z'
labels: []
dependencies: []
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend the complete-generation workflow to configured TypeScript projects and statically supported React/Next.js relationships, with compiler binding and honest uncertainty.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Shared classification and prepared jobs preserve existing C#/Java behavior
- [x] #2 UI and CLI analyze TypeScript 6.0.x, references and TSX on Node 24+, with errors rejecting publication
- [x] #3 React static relationships and Next App Router entries retain certainty and provenance, diagnosing dynamic delegation
- [x] #4 Persistence, freshness, cancellation, mixed languages and old-cache compatibility are verified
- [x] #5 MapEditor CLI and packaged-app acceptance, Rust/Python checks, packaging and CodeScene reviews pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Refactor relationship classification and typed prepared jobs with regression coverage. 2. Add modular project-installed compiler adapter with validated config/reference ownership. 3. Implement React/Next rules, provenance and framework entries. 4. Integrate discovery, fingerprints, UI/CLI, packaging and documentation. 5. Validate fixtures, MapEditor, existing regressions, independent review and CodeScene, recording evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created task and recorded approved plan. Planner/explorer checked persistence compatibility, callback attribution and source-only project-reference risks. Refactored shared edge classification and typed prepared jobs; added framework entry metadata and edge provenance in backward-compatible generation metadata. Initial compile hit disk exhaustion; removed only checkout debug artifacts and rebuilt with debug symbols disabled. Sonar intentionally inactive: no .sonar/config.json.

Implementation and independent review complete. Reviewer findings fixed with regressions: recursive empty-directory freshness, language-scoped output exclusions, constructor-field attribution, synthesized union signature safety, bracket accessors, Next convention scope and propagated-callback certainty. Full Rust include-ignored suite passed 108 tests, Python 27, Node 17; Clippy clean. Package verified pinned sources and ad-hoc signature. Real packaged CLI published MapEditor 5850 symbols/9090 relationships; mixed C#/Gradle/Maven/TypeScript generation 5879/9117 with two framework entries and three verified starts. Initial sandbox .NET restore and Metal access required toolchain/GPU access; reruns passed. Cargo fmt vendor formatting was proven against formatted pinned originals and fully restored; future formatting scoped to codegraph.

Final verification: 19 Node fixtures passed, including absent generated-directory watches and parenthesized propagated callback certainty; seven Rust TypeScript production tests rerun successfully. All modified source modules reviewed at Code Health 10.0 after separating compiler setup, dependency inputs and accessor analysis; final safeguard passed 44 eligible files. Final packaged MapEditor run retained 5850/9090 in 8.68s with 867942400 bytes maximum RSS. Packaged Metal screenshot/UI tree confirms real Page -> EditorPage possible render edge and framework-entry label. Updated DMG checksum verified. Evidence: evidence/analyzers/typescript.md, typescript-summary.json and evidence/ui/typescript. No Sonar config; no PR or push requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented project-installed TypeScript 6.0.x/Node24+ analysis, compiler-backed React static relationships and Next App Router framework entries. Unified relationship classification and prepared-job orchestration; preserved existing language behavior and old graph caches. Verified 108 Rust tests, 27 Python tests, 19 real-compiler Node fixtures, Clippy, independent review, CodeScene and signed app/ZIP/verified DMG. Real MapEditor and mixed-language packaged pipelines passed. Dynamic delegation, JavaScript/allowJs, Pages Router and custom routing remain deferred as approved.
<!-- SECTION:FINAL_SUMMARY:END -->
