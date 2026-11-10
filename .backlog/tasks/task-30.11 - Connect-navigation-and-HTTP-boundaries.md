---
id: TASK-30.11
title: Connect navigation and HTTP boundaries
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T19:23:00Z'
labels: []
dependencies:
  - TASK-30.10
parent_task_id: TASK-30
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Next navigation and literal requests resolve unique owned routes with explicit kinds
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Freeze framework checkpoint b7207e2 and commit new navigation/HTTP fixture source plus pre-boundary graphs first. Add shared versioned route/use facts and explicit flow target kinds in core; retain unsupported route declarations as blockers instead of inventing unique matches. Recognize compiler-bound Next Link/router/redirect and built-in fetch; support safe root-relative literal URLs and common single-segment parameters, preserving default static precedence only within one application. Extend qualified Spring and ASP.NET attribute extraction with source sites, method/path/case/condition information and unsupported-pattern blockers. Match after composition. Automatic links stay within one Next owner; cross-language HTTP uses optional project-local .codegraph/boundaries.yml version1 client-to-server build mapping. Verify the configuration as an input including missing-file creation; no new graph destination/protocol flags. Ambiguous scopes, constraints, order and rewrites remain unresolved. Land navigation then HTTP separately with exact graph/flow boundary kinds, negative controls, relocation/reordering, native-edge preservation, performance and packaging checks. No route configuration execution or general fuzzy matching.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Framework dependency completed and frozen at /private/tmp/codegraph-stage4-b7207e2. HTTP application mapping is required because selected builds do not establish shared origin; this narrow shared configuration avoids connecting unrelated apps. Planner primary-source review covers Spring mapping conditions, ASP.NET case/order and Next static-versus-dynamic routing precedence.

Fixture baselines committed separately at dc32e17 and 9a6d5b6 using frozen b7207e2 resources. Shared boundary contracts/config/canonical/store/flow tests pass; actual TypeScript graph-flow acceptance passes with native edges preserved. Compiler-backed negative controls added for Pages Router, Link as, router writes/escapes, global fetch structural writes/escapes, and deferred class initialization. Java HTTP baseline and extended fixtures pass; ASP.NET extraction and independent selector review remain in progress.

Completed: explicit Next navigation/builtin-fetch and qualified Spring/ASP.NET route facts; unsupported constraints retained as blockers; explicit boundaries.yml application mapping. Frozen fixture checkpoints dc32e17/9a6d5b6. Exact node/native-edge/architecture/translated-flow preservation verified for TS/Spring/MediatR/mixed. Added4 standalone TS and6 mixed boundary edges with graph-flow agreement. Packaged app+engine and all4 workload repeat/reorder/relocation checks pass. All98 TS tests,56 project Python,19 tool tests, workspace all-target/strict Clippy and real language/UI checks pass. Median runtime deltas +1.45%/+2.37%/-0.58%; sampled RSS increases below5%. Full upstream262pass/29fail: identical29 prior failure IDs,5newHTTPtests pass. Evidence evidence/engine/stages/boundaries.md; upstream modifications captured in checksum-pinned patch and pre-HTTP migration checkpoint.
<!-- SECTION:NOTES:END -->
