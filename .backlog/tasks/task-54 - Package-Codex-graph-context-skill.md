---
id: TASK-54
title: Package Codex graph-context skill
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-12T21:01:00Z'
labels: []
dependencies:
  - TASK-52
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
  - evidence/agent-integrations/delivery-mcp.md
  - evidence/agent-integrations/codex-native-mcp.md
type: feature
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Package the canonical native-MCP graph-context workflow for Codex. The user selected MCP for the agent experience, superseding the direct terminal-query prototype. Use the independently installed codecallnavigator-mcp server with ccnavigator as its engine command; both resolve through PATH, with explicit project binding configured once in the host. Deliver self-contained canonical skill copies with drift detection, a local marketplace, host setup/removal guidance and actual native-tool conversation evidence. Bundle no server implementation, graph algorithms/cache, binaries or hooks. Validate explicit/implicit selection, generation/freshness/ambiguity, repeated contexts, unavailable flow/MCP and lifecycle preservation. Preserve CLI compatibility, normal host trust and precise unvalidated-surface limits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Codex CLI/desktop native-MCP support claims each have isolated installation, invocation and removal evidence or an explicit unvalidated-surface limitation.
- [x] #2 The self-contained package uses configured MCP tools backed by ccnavigator on PATH, preserves canonical guidance without silent drift, and has no hooks, automatic analysis or terminal graph-query fallback.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reuse verified deterministic packaging and migrate canonical guidance/metadata to native MCP tools.
2. Document explicit project-bound host MCP setup/removal using command names on PATH, without guessed cwd or binary paths.
3. Validate actual isolated package installation and native MCP tool conversations, including explicit/implicit selection and failure/freshness/repeat cases.
4. Review and run current MCP/compiler acceptance plus full packaging verification; retain superseded CLI evidence only as history.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-52 dependency is Done (ce39d9b). Claude package source/atomic shared builder is implemented and focused-tested; its final conversation retest and full integration gate remain pending. Starting Codex source next on the same branch. Both independent host tasks will close only after their own host evidence and the combined full packaging run.

Portable manifest/catalog installed successfully into isolated CODEX_HOME; installed skill bytes match actual artifact. Package source and metadata reviews pending final response; 11 both-host package tests and 21 verification tests pass, CodeScene 4/4 source/test files 10.0. Exact local-executor boolean validation regression failed before fix. Final combined full verification started at target/verification/agent-integrations-full; real CLI model journeys ongoing. Desktop UI cannot be isolated with available surfaces and remains explicitly unvalidated.

User reversed plugin transport to MCP for optimized native-agent use. Direct-CLI plugin prototypes and their model traces are retained as historical iteration only. Updating canonical skill, project-bound MCP setup, host-native tool acceptance and full verification. ccnavigator remains the independently installed PATH engine command; no executable path configuration is needed. Superseded CLI package full run was intentionally interrupted and is not reported as passed.

Final native-MCP package installed in isolated Codex configuration with cached skill hash matching canonical wrapper. Actual explicit/implicit invocation, unknown-binding clarification, actual truncation, real-match status-only repeat, unavailable flow, stale/missing/disabled fallback and explicit refresh passed. Disable/enable and target/catalog/server removal verified with new-session absence; source/graph hashes and eleven unrelated managed plugins preserved. Full verification all33 passed. Codex ambiguity prose omitted generation once while native arguments stayed generation-bound; disclosed as model-output limitation. Desktop UI acceptance remains explicitly unvalidated with no desktop support claim.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered the Codex local-executor skill-only plugin, local marketplace and deterministic archive with explicit native MCP project configuration using command names on PATH. Real codex-cli 0.166.0 acceptance and isolated lifecycle passed; no MCP implementation, hooks or automatic analysis shipped. Desktop/IDE/cloud/web and other platforms remain unvalidated. Full verification and CodeScene gates passed; evidence in evidence/agent-integrations/codex-native-mcp.md and delivery-mcp.md.
<!-- SECTION:FINAL_SUMMARY:END -->
