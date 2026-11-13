---
id: TASK-63
title: Make C# cold and warm analysis input coverage consistent
status: Done
assignee:
  - '@codex'
created_date: '2026-11-13T08:21:00Z'
updated_date: '2026-11-13T11:54:00Z'
labels: []
dependencies:
  - TASK-59
references:
  - TASK-59
documentation:
  - evidence/freshness/README.md
type: bug
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The TASK-59 independent clean-project audit finds equal C# graph semantics but different observed-input coverage in all nine saved-edit scenarios: clean reference includes obj/Freshness.csproj.nuget.g.targets while warm candidates omit it. Determine whether MSBuild restore/evaluated imports or analyzer capture causes this discrepancy before relying on that manifest for continuous freshness; do not hide the input or weaken artifact comparison.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A real .NET fixture reproduces and explains the clean/warm generated-NuGet-target input discrepancy with retained before/after manifests.
- [x] #2 The owning analyzer has one documented evaluated-input policy that preserves every consumed relevant build input and handles generated restore outputs without false freshness or refresh loops.
- [x] #3 Independent clean and warm analyses satisfy the documented full-artifact/input-coverage contract while retaining equal graph semantics and uncertainty/provenance.
- [x] #4 Source/build-input changes and failed refresh recovery remain covered by real compiler acceptance, full verification and CodeScene.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reproduce the timestamp-sensitive MSBuildAllProjects manifest discrepancy with a real SDK11 fixture before changing production code.
2. Extract C# evaluated import inventory into csharp/inputs.rs using MSBuild preprocess output, strict XML comment-frame validation, explicit root/import inputs and contextual unsupported-format errors. Use the already-locked roxmltree parser directly if needed.
3. Restore first, discover imports, snapshot explicit inputs, repeat inventory under that snapshot and evaluate source metadata before CallAtlas; reject changed inventories/content. Stop reading MSBuildAllProjects as the imported-input list.
4. Add real compiler regressions for generated NuGet imports, identical-byte rewrites, property-only/nested/external imports and stale detection; preserve invalid-refresh artifacts. Document external mapping/absent-probe limits and version the C# input policy.
5. Review, test, apply CodeScene/full verification, finalize evidence and commit TASK-63 on the current branch before beginning TASK-60.

7. Independent review exposed consumed design-time-only imports. Align both metadata and preprocessing with explicit normal plus pinned Roslyn 5.9 NET design-time contexts and framework inner builds; carry SolutionDir only for solution opens. Add design-time, framework and solution-property regressions, then repeat review/tests before full verification.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Causal probe retained at evidence/freshness/csharp-import-probe.json: after restore MSBuildAllProjects names obj/Freshness.csproj.nuget.g.targets; rewriting only csproj with identical bytes switches the property to Freshness.csproj. The analyzer consumes this property as its imported-input inventory. SDK 11.0.100; no production fix in TASK-59.

Real SDK regression first failed on the previous inventory: NuGet targets disappeared after identical-byte project rewrite. Replaced the inventory with strict framed preprocessing, explicit imported-file snapshots and repeated inventory checks around metadata and compiler extraction. Legacy C# manifests now report unverifiable without mutating artifacts; empty C# target detection and TS/Java isolation are covered. Independent review and real CLI clean-reference audit are running before full verification.

First implementation passed baseline regressions, core tests and a nine-scenario real CLI clean audit. Independent review then proved a missing consumed DesignTimeBuild-only import: CallAtlas exported DesignOnly while default preprocessing omitted its external props input. Correcting the premise: two evaluation actors (normal preparation and Roslyn extraction) require an explicit context union. Pinned Roslyn source/installed NuGet commit 35d9211 identifies exact global properties, TFM selection and solution-only SolutionDir; no guessed configuration/platform mappings. Review also requires fresh preprocess output and exact path-line handling.

Second review pass rejected lossy context aggregation. Revised premise: normal build preparation and Roslyn extraction are two actors, with per-framework variants; one merged metadata result cannot own both classification and dependency meaning. Configured normal ownership must remain authoritative on overlapping files, while source/input scopes can be unioned. Runtime validation must retain exact import membership per context (not only the union) to detect normal/design membership swaps. Independent tester passed all six current real compiler regressions plus six core checks; review-specific regressions are being added before the next full gate.

Final independent review resolved ownership overwrite, per-context membership loss and literal MSBuild property injection. Eight real compiler regressions, eleven input/context tests and two ownership tests pass; Clippy and all modified-source CodeScene checks pass. Reviewer found no remaining demonstrated source blocker. The special-path native SolutionDir-expanded probe over-observes an unused input, documented as a native limitation rather than semantic equivalence; no blanket path rejection/upstream fix was introduced. Full verification and final public CLI audit now running; retained binaries support subsequent task diagnostics.

The first full verification attempt passed prerequisites, static/build/Clippy/ordinary tests and all completed release groups, but stage 24 exceeded the default 1800-second aggregate timeout during the C# integration group after two large private Maven cases. No assertion failed. Cleanup was reported unverified; a separate outside-sandbox process inventory found no retained descendants from that run. Retained full-timeout-results.json and restarted the entire full script with --timeout 3600; no runtime code was changed.

Full verification retry passed all 36 stages with --timeout 3600, including all eight new C# integration tests, complete release acceptance, benchmarks, packaging/signatures, installed CLI and independent MCP journeys. Final public CLI policy-v2 audit passed 18/18 independent clean comparisons across nine edit scenarios; invalid refresh recovery and generation mismatch checks passed. All ten modified sources scored Code Health 10 and the pre-commit safeguard passed. Sonar is inactive because this project has no .sonar/config.json. Evidence and the earlier timeout are retained under evidence/freshness/csharp-import-fix/verification.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed timestamp-sensitive C# import coverage with strict context-aware evaluated import inventories, snapshot rechecks, normal ownership authority and literal MSBuild properties. Legacy C# manifests remain readable and conservatively unverifiable. Real regressions and independent full-artifact comparisons pass; documented observed-input and native special-path limitations remain explicit. Research, review and verification evidence: evidence/freshness/csharp-import-fix/README.md. Full verification passed all 36 stages; commit on current branch follows, with no push.
<!-- SECTION:FINAL_SUMMARY:END -->
