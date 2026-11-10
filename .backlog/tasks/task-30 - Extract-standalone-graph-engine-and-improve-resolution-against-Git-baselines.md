---
id: TASK-30
title: Extract standalone graph engine and improve resolution against Git baselines
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-10T13:41:00Z'
updated_date: '2026-11-10T16:19:00Z'
labels: []
dependencies: []
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved roadmap: preserve merged baseline, extract independent engine, publish one project-local canonical graph and compare Git versions, then precise inference stages after equivalence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Foundation verified against master fb54f5f before resolution changes
- [x] #2 One deterministic .codegraph/graph.json per project and Git comparator
- [x] #3 Desktop invokes packaged standalone engine
- [ ] #4 Resolution stages verified in dependency order
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Complete foundation 0A-0F in separate commits, verifying frozen native payload equivalence before inferred relationships. Then implement evidence, observer channels, callable propagation, framework dispatch, HTTP/navigation and incremental inference.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Foundation completed and verified before any inference changes. Master baseline fb54f5f; extraction commits 8fff86a, 2d92b77, b663649, cf0d8fd. Proceed to evidence and resolution stages in dependency order.

Preparing a required bootstrap prerequisite for the approved framework stage: dependency patch verification must include patch-added upstream files without modifying the real upstream index or relying on unrecorded edits/stashes. Use a temporary Git index to obtain the complete diff; exact checked-in patch-byte matching remains the acceptance gate. Add real Git migration/idempotence/rejection tests before changing the helper.

Patch-bootstrap prerequisite implemented and independently reviewed: temporary copy of the real index includes patch-added files and staged additions, without real-index mutation. Tests first reproduced repeat-verification rejection, then found/fixed a staged-added omission in an earlier read-tree-based approach. Six dependency tests, existing54 Python root tests and new4 packaging tests pass; actual vendor verification passes with byte-identical normal/split indexes and literal/space/newline paths. Source reviews10.0. Repo-wide safeguard is temporarily pending the concurrent callable implementation cleanup; no commit yet.

Bootstrap prerequisite final gate passed: repo-wide CodeScene safeguard now passes15eligible files after callable cleanup; no regressions in the prerequisite source. Existing root54tests, six dependency tests, four packaging tests, real normal/split-index probes and actual three-vendor verification passed. Ready for the separately scoped bootstrap support commit.
<!-- SECTION:NOTES:END -->
