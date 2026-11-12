---
id: TASK-52
title: Define shared graph-context workflow and journey acceptance
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T19:40:00Z'
labels: []
dependencies: []
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
  - integrations/shared/graph-context.md
  - integrations/shared/journeys.md
  - evidence/agent-integrations/delivery-task52.md
type: feature
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents need one portable graph-context workflow before host packages ship. Deliver integrations/shared/graph-context.md with status-first, generation-bound direct JSON CLI queries, ambiguity handling, directed depth-one bounded selections, 20-hit searches, 8KiB rendered summaries preserving omissions/provenance, explicit analysis and stale/unverifiable fallback. The user selected direct terminal invocation under the public command ccnavigator available on PATH, replacing the research-era MCP route. Add a thin installed ccnavigator entry point sharing engine command policy while preserving desktop/MCP contracts, plus real compiler CLI and host conversation evidence. No skill-side graph reader, algorithms or persistent cache. Use real observed graph states and record representative model limitations honestly; full verification and CodeScene are required for executable changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All documented journeys have reproducible expected results against the real direct CLI, including recorded host repeat suppression and stale/unverifiable fallback.
- [x] #2 The canonical workflow never implicitly analyzes, combines generations, chooses an ambiguous symbol silently or claims possible calls were observed execution.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Deliver canonical host-neutral graph-context instructions with direct ccnavigator JSON query examples, generation/freshness handling and explicit-analysis policy.
2. Provide the public ccnavigator PATH command sharing existing engine entrypoint policy and bundled standalone resources, preserving desktop/MCP contracts.
3. Exercise compiler-produced graph journeys through the actual CLI and record representative Claude/Codex conversation behavior separately from backend assertions.
4. Review source, run full verification and relocated packaged-CLI acceptance, document limitations and finalize with evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Started on codex/agent-integrations. Planner confirmed sequential TASK-52 then TASK-53/54, with no hooks. Initial compiler fixture can report unverifiable freshness; acceptance must preserve that distinction and not equate it with fresh repeat suppression.

User explicitly chose direct CLI, named ccnavigator, available on PATH without binary path setup. Earlier MCP baseline is supporting prior evidence only; direct CLI acceptance replaces it for delivery.

Superseded MCP verification run target/verification/task52-full was interrupted during ordinary Rust tests when user changed delivery to direct ccnavigator CLI. It is not a pass; runner reported interrupted/cleanup-unavailable. Full verification will be repeated against the direct CLI implementation. Public command regression added first to observe missing PATH command before implementation.

Direct CLI reviewer found no blocking source defect; independent tester passed 20 real ccnavigator operations through PATH. Added exact roots array-of-pairs and selection JSON examples after an initial real Codex trial guessed the roots shape incorrectly; trial retained as diagnostic evidence, corrected workflow retested.

Direct PATH regression first failed with ccnavigator not found, then passed after adding a thin Cargo binary target sharing entrypoints. Initial src/bin module path did not resolve nested modules; switched to explicit src/ccnavigator.rs binary target. CodeScene found run() length regression from help-name handling; extracted print_help and re-review is 10.0. All five engine/packaging source reviews now clean. Public compiler acceptance tester passed 20 CLI operations. Real Codex and Claude shared-skill conversations now prove ambiguity, generation binding, match freshness, repeated-section suppression and flow_unavailable; final Codex repeat after instruction refinement emits only a reference plus status. Extra scenarios cover stale/truncation; read-only explicit-refresh denial is retained and a writable authorized-refresh trial is running.

Independent tester reviewed actual host JSONL transcripts: status-only repeats, expected_generation on analyzed requests, real flow_unavailable, no implicit analysis for stale/missing, and checked outputs below 8KiB. Authorized-refresh initial status omitted the test root mapping before recovery; evidence discloses this. Source-fallback misattribution is disclosed, with final refined wording to be retested against the host package.

Final full verification passed all 28 stages at target/verification/task52-direct-full: 552 ordinary Rust, 642 release acceptance, 164 Python project, 26 Python tool and 112 TypeScript tests; benchmarks, package signatures, independent MCP installation and cleanup passed. Newly packaged ccnavigator independently passed all 20 real CLI journey operations via PATH, proving bundled resource discovery. CodeScene source reviews all 10.0 and safeguard 7/7 clean. Sonar inactive: no .sonar/config.json. Host evidence/limitations in evidence/agent-integrations/delivery-task52.md.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered canonical direct-ccnavigator workflow, exact CLI examples and journey expectations, real compiler acceptance tooling, public PATH command sharing engine policy, and standalone packaging. Actual Claude/Codex conversations verify ambiguity, bounded generation-aware queries, unsupported flow, status-only repeats and explicit refresh/fallback behavior. All full verification stages and packaged CLI acceptance passed; host-package install/implicit-selection/removal remains TASK-53/54. No hooks shipped.
<!-- SECTION:FINAL_SUMMARY:END -->
