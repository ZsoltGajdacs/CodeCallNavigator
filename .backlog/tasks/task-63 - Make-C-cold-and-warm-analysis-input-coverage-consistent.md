---
id: TASK-63
title: Make C# cold and warm analysis input coverage consistent
status: To Do
assignee: []
created_date: '2026-11-13T08:21:00Z'
updated_date: '2026-11-13T08:24:00Z'
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
- [ ] #1 A real .NET fixture reproduces and explains the clean/warm generated-NuGet-target input discrepancy with retained before/after manifests.
- [ ] #2 The owning analyzer has one documented evaluated-input policy that preserves every consumed relevant build input and handles generated restore outputs without false freshness or refresh loops.
- [ ] #3 Independent clean and warm analyses satisfy the documented full-artifact/input-coverage contract while retaining equal graph semantics and uncertainty/provenance.
- [ ] #4 Source/build-input changes and failed refresh recovery remain covered by real compiler acceptance, full verification and CodeScene.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Causal probe retained at evidence/freshness/csharp-import-probe.json: after restore MSBuildAllProjects names obj/Freshness.csproj.nuget.g.targets; rewriting only csproj with identical bytes switches the property to Freshness.csproj. The analyzer consumes this property as its imported-input inventory. SDK 11.0.100; no production fix in TASK-59.
<!-- SECTION:NOTES:END -->
