---
id: TASK-52
title: Define shared graph-context workflow and journey acceptance
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T21:01:00Z'
labels: []
dependencies: []
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
  - integrations/shared/graph-context.md
  - integrations/shared/journeys.md
  - evidence/agent-integrations/delivery-task52.md
  - evidence/agent-integrations/delivery-mcp.md
  - evidence/agent-integrations/native-mcp-protocol.json
type: feature
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Deliver one canonical graph-context workflow using native CodeCallNavigator MCP tools, backed by the CLI. Preserve status-first generation-bound queries, ambiguity handling, bounded selections and summaries, certainty/provenance, stale/unverifiable fallback, repeated-context suppression and explicit-only refresh. The user selected MCP for the plugin agent experience after the direct CLI iteration. Keep the already verified ccnavigator terminal command on PATH as the MCP engine; no executable paths are required. Configure the independent MCP server explicitly per project rather than guessing project binding from plugin cwd. Retain direct CLI compatibility coverage and add real compiler/MCP and native host evidence; old CLI-only host evidence is historical.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The canonical MCP journeys have reproducible compiler-backed protocol outcomes and real native-tool host evidence, including repeat suppression and stale/unverifiable fallback.
- [x] #2 The workflow preserves generation/project binding, ambiguity, certainty and explicit refresh, with no implicit terminal graph-query or analysis fallback.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Replace direct terminal graph guidance with native MCP operations while retaining the public ccnavigator CLI and compatibility checks.
2. Use separate explicit project-bound stdio configuration with codecallnavigator-mcp and ccnavigator resolved from PATH.
3. Exercise actual MCP protocol/compiler journeys and native Claude/Codex tool conversations separately from installation checks.
4. Review, run applicable full verification and finalize only MCP-backed criteria with current evidence.
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

User changed plugin transport back to MCP. Reopening shared workflow delivery for MCP tool guidance and acceptance; the committed ccnavigator PATH entry point remains valid. Direct-CLI host evidence remains historical, not proof of current MCP behavior.

Native MCP source/protocol helper passes 19 real tools plus listing via PATH. Actual Claude MCP probe connected and returned complete generation/freshness/counts; natural implicit caller prompt also succeeded with native tools. No MCP Rust/query-contract extension needed. MCP plugin packages rebuilt and fresh Claude install bytes verified. Superseded CLI package full run target/verification/agent-integrations-full was intentionally interrupted after user scope change; cleanup was reported unverified, not passed. Current full validation started at target/verification/agent-integrations-mcp-full.

Current native-MCP compiler acceptance passed 19 tool operations plus listing. Final Claude explicit/implicit/disabled traces independently reviewed in evidence/agent-integrations/native-mcp-hosts-initial.md. Source-only fallback made no terminal graph queries or artifact content reads; filename enumeration is disclosed. Original direct-route completion summary cleared while current native host/full verification is pending.

Current native MCP delivery complete. Full verification passed all 33 stages at target/verification/agent-integrations-mcp-full, including 552 ordinary Rust, 642 release acceptance, 175 project Python, 26 tool Python and 112 TypeScript tests, package/signature checks, 20 standalone CLI operations and 19 native MCP operations. Both real hosts passed native explicit/implicit queries, status-only real-match repeat, flow_unavailable, stale/missing/disabled fallback, explicit refresh/new generation, unknown-binding clarification and actual truncation. Eight CodeScene source reviews at 10.0; safeguard checked eight eligible files with no issues. Evidence records model prose limits and host surface constraints.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered the canonical native-MCP graph-context workflow and real compiler/host acceptance, retaining ccnavigator as the PATH-resolved engine. Project binding stays explicit; analyzed tools are generation-bound, ambiguity and uncertainty retained, summaries bounded and unchanged repeats suppressed. Ordinary queries never refresh, use terminal graph fallbacks or read graph storage. All 33 full verification stages passed; current evidence and reproducible transcripts are linked in evidence/agent-integrations/delivery-mcp.md. No hooks or API extensions shipped.
<!-- SECTION:FINAL_SUMMARY:END -->
