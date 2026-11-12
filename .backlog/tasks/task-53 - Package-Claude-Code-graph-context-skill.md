---
id: TASK-53
title: Package Claude Code graph-context skill
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
  - evidence/agent-integrations/claude-native-mcp.md
  - evidence/agent-integrations/native-mcp-hosts-initial.md
type: feature
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Package the canonical native-MCP graph-context workflow for Claude Code. The user selected MCP for the agent experience, superseding the direct terminal-query prototype. Use the independently installed codecallnavigator-mcp server with ccnavigator as its engine command; both resolve through PATH, with explicit project binding configured once in the host. Deliver self-contained canonical skill copies with drift detection, a local marketplace, host setup/removal guidance and actual native-tool conversation evidence. Bundle no server implementation, graph algorithms/cache, binaries or hooks. Validate explicit/implicit selection, generation/freshness/ambiguity, repeated contexts, unavailable flow/MCP and lifecycle preservation. Preserve CLI compatibility, normal host trust and precise unvalidated-surface limits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Isolated Claude Code install, explicit/implicit native-MCP skill journeys and uninstall have recorded evidence with no production configuration changes.
- [x] #2 The package preserves canonical MCP workflow and drift checks, documents project-bound MCP setup/removal without executable paths, and ships no hooks or graph/server implementation.
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
TASK-52 completed and full verification passed. Starting actual Claude package and shared packaging support on codex/agent-integrations. Public invocation is ccnavigator from PATH; no MCP setup. Local self-root catalog format was validated in temporary configs before implementation.

Focused package tests 8/8 and verification tests 21/21 passed; independent review found no blocking source defect. Actual isolated install matches archive/canonical bytes; explicit and implicit Claude sessions succeeded, including real Skill invocation on implicit prompt. Real conversations exposed manual symbol-ID re-encoding retry and fallback search touching graph storage; refined canonical instructions to use parsed IDs and explicitly exclude .codecallnavigator, synchronized/rebuilt v2 for retest. Refined filename-preserving fallback correctly attributed the interface on first package trial.

User reversed plugin transport to MCP for optimized native-agent use. Direct-CLI plugin prototypes and their model traces are retained as historical iteration only. Updating canonical skill, project-bound MCP setup, host-native tool acceptance and full verification. ccnavigator remains the independently installed PATH engine command; no executable path configuration is needed. Superseded CLI package full run was intentionally interrupted and is not reported as passed.

Final native-MCP package installed in isolated Claude configuration with cached skill hash matching the canonical wrapper. Actual explicit/implicit invocation, unknown-binding clarification, actual bounded truncation, real-match status-only repeat, unavailable flow, stale/missing/disabled fallback and explicit refresh passed. Disable/enable, package/catalog/server removal and new-session absence passed; source/graph hashes unchanged, downloaded cache retained as Claude behavior. Full verification all33 passed. Evidence: evidence/agent-integrations/claude-native-mcp.md and delivery-mcp.md.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered the Claude Code skill-only plugin, local marketplace and deterministic package build with explicit per-project native MCP setup using command names on PATH. Real Claude Code 2.1.306 acceptance and isolated lifecycle passed; no server implementation, hooks, global configuration or automatic analysis shipped. Other platforms/surfaces remain unvalidated. Full verification and CodeScene gates passed.
<!-- SECTION:FINAL_SUMMARY:END -->
