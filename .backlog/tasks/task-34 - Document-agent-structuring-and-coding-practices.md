---
id: TASK-34
title: Document agent structuring and coding practices
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T05:40:00Z'
updated_date: '2026-11-11T05:47:00Z'
labels: []
dependencies: []
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents need repository-specific guidance at the root and both member crate roots so future changes respect the existing desktop, shared-core and analyzer boundaries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root agent.md defines workspace ownership, coding practices and proportionate verification.
- [x] #2 Both member crates have agent.md guidance tailored to their current responsibilities and contracts.
- [x] #3 Existing AGENTS.md points to the guides, existing mandatory policies are preserved, and local references and commands are verified.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Inspect manifests and architecture contracts; write the three scoped guides and link them from AGENTS.md; verify references and the documentation diff.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Inspected manifests, public module roots and the standalone engine contract; planner/explorer and implementer roles produced three scoped lowercase guides. Added discovery links in AGENTS.md and verified prior policies remain verbatim. Validated all 17 relative links, Cargo package references and whitespace including untracked files. Used Python 3.13 after system Python lacked tomllib. Initial CLI creation could not write the .git lock in the sandbox; authorized elevated CLI retry succeeded. Documentation-only change: compiler, analyzer, packaging and source CodeScene checks are not applicable; Sonar is inactive because .sonar/config.json is absent.

Reviewer/tester inspection found no issues with ownership, contract guidance, links or commands. Parent independently read the guides and repeated link/package/whitespace validation. Changes remain uncommitted.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added agent.md at the workspace root and both member crate roots with responsibility boundaries, Rust practices, contract invariants and proportionate verification. AGENTS.md links the guides while retaining all prior policies. Review and documentation checks passed; no runtime/source changes, commit or push.
<!-- SECTION:FINAL_SUMMARY:END -->
