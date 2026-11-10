---
id: TASK-30.10
title: Resolve qualified framework event dispatch
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-10T13:42:00Z'
updated_date: '2026-11-10T17:12:00Z'
labels: []
dependencies:
  - TASK-30.9
parent_task_id: TASK-30
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved standalone graph engine roadmap; execute only after the preceding checkpoint is verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Spring and MediatR qualified dispatch preserve conditional and asynchronous semantics
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Freeze the completed callable distribution, create real Spring6.2.5 and pinned MediatR fixtures, and commit their current graph baselines before enabling inference. Land Spring first: a focused javac pass computes compilerqualified publisher/listener pairs with Types assignability and UTF-8 sites inside each source context; normalize native identities and attach matching possible targets to Java flow sites. Preserve condition/transaction/async evidence and diagnose unsupported erased/generic/cross-context wiring. Then land MediatR: separate Roslyn-bound framework facts without changing native CallAtlas edges; exact closed request/notification contracts, argument ordinal mapping, compiler-visible assembly scope, explicit/inherited Handle resolution and anonymous-site deferral. Preserve upstream changes as complete checked-in patches with hash/migration/bootstrap verification. Validate each language on positive/negative real toolchains, prior graph invariants, repeated/relocated output, performance and CodeScene; C# flow inventory remains unavailable rather than fabricated.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Dependency completed in a22e651; frozen matching executable/resources at /private/tmp/codegraph-stage3-a22e651, rebuildable from commit. Bootstrap added-file prerequisite273cd2c passed independent review and exact patch verification. No framework source changes yet.

Real framework fixtures compiled before inference: Spring Maven test-compile succeeds offline (9 source files); MediatR12.4.1 locked public-only restore and Release build succeed (.NET10.0.403, zero warnings/errors). Frozen a22e651 graphs recorded: Spring19nodes13edges; MediatR25nodes2edges; expanded mixed119nodes76edges. No Spring/MediatR inferred edges present. Fixtures cover source-bound positives, type/API collisions, erasure, async/transaction conditions and unrelated application scopes. Fixture reviews10.0/no eligible findings, pre-commit safeguard passed13eligible source files.

Stage4A Spring verified:3realcompiled acceptance tests,6inventory tests,5existing Javaflow tests,28engineunit tests, five common evidence tests and Runeinspector qualifier test passed. Independent review reproduced raw/inherited/enclosing generic unchecked matches; actual Spring6.2.5 runtime also rejects direct Box<String> where javac assignability accepts. Guards defer erased generic payloads while retaining concrete StringBox metadata. Runtime requires matching v5 capability. Listener catalog persists9entries after publishers removed with0inferrededges. Corpus adds exact16edges23source sites; prior nodes/edges unchanged; Spring19/29 and mixed119/92. Repeated/relocated/reordered Spring+mixed artifacts byte-identical. Final3run median2.631550958s baseline vs2.660514959s candidate (+1.10%); sampledRSS227704832 vs212303872bytes. Structured opaque qualifiers survive SQLite/canonical/Git comparison and inspector, including reserved dictionary keys. All source reviews10.0; strictworkspaceClippy and precommit safeguard21eligible files passed. MediatR remains pending; its draft9target/6deferral check passes outside vendor checkout, with actual production test red before integration.
<!-- SECTION:NOTES:END -->
