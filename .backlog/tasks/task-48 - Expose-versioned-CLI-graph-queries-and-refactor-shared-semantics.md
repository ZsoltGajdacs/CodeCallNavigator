---
id: TASK-48
title: Expose versioned CLI graph queries and refactor shared semantics
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T13:16:00Z'
updated_date: '2026-11-12T13:54:00Z'
labels: []
dependencies: []
references:
  - DRAFT-1
ordinal: 58000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Desktop and external agents need one authoritative process interface for analyzed graph information. Establish a reusable CLI contract before migrating consumers, keeping semantic queries separate from presentation and transport.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI independently exposes capabilities, graph status, search, symbols, graph selections, navigation, analyzed source and function flow through versioned JSON.
- [x] #2 Responses preserve graph identity, generation binding, source locations, relationship meaning, certainty/provenance, diagnostics and explicit query limits.
- [x] #3 Shared query ownership is refactored cleanly; queries remain read-only and analysis preserves existing publication and cancellation contracts.
- [x] #4 Real-graph acceptance, interactive latency measurements, documentation, full verification and CodeScene review pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Inventory desktop query needs and establish a shared versioned query request/result contract with stable wire IDs. 2. Add CLI capabilities/query handling backed by one canonical artifact read and generation-bound verification/hydration. 3. Expose search, snapshot, symbol/source/explorer, graph selections/neighbors and shared function flow through focused operations; extract semantic policy from desktop ownership. 4. Verify observable real-fixture behavior and deterministic bounded results, document contract/examples and measure interactive workloads. 5. Run full verification and CodeScene; finalize only against evidence before TASK-49 begins.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Approved three-task sequence recorded as TASK-48 -> TASK-49 -> TASK-50, all referencing DRAFT-1. Planner identified stable-ID search adapters, snapshot boot replacement and desktop-owned selection/flow semantics requiring extraction. Initial sandbox Backlog creation failed on .git lock; authorized escalation succeeded. Sonar is inactive: no .sonar/config.json.

Implemented CLI-owned persistent query session after measurements justified reuse. Session hashes actual current artifact bytes, retains parsed immutable generation, and rechecks observed source inputs. Added failing-then-passing session regression for changed inputs, graph replacement, expected-generation mismatch and EOF shutdown; existing verifier tests pass. Real 113MB Java graph measurements: one-shot search median 1864ms vs session 234ms, selection 1891ms vs249ms; snapshot remains startup-scale. Evidence: evidence/queries/README.md and billing-latency.json. Required full prerequisite checks passed with installed TS6/React19/Next16 modules, private Billing fixture and saved profiling workspace. Parent modified verifier/query/session/benchmark files reviewed at Code Health10.0.

CLI unit is ready for independent reviewer. Focused command tests: four ordinary query regressions pass; real TypeScript compiler-backed query test passes; session tests cover source freshness, graph replacement and pre-analysis live explorer; existing four analysis protocol/process cancellation regressions pass. Query processes now retain default signal termination since they own no build children; analysis retains cooperative cancellation. Live explorer can list configured roots before the first artifact exists, with null graph metadata. Explicit query filters require version2 to avoid legacy persisted-filter upgrade semantics silently changing requested relationship selections.

Independent reviewer found two interface issues: bounded overview would lose grouped desktop counts, and class search expanded unbounded member IDs. Both corrected: generation-required render_selection shares semantic selector but returns complete filtered membership; class search returns identity/member_count. Regressions above1500 nodes and limit1 on1502-member class pass; reviewer recheck reports no remaining actionable findings. Six ordinary query tests plus two session tests pass. Full suite next; tester is independently exercising real compiler query and cancellation.

Full runtime verification passed:502 ordinary Rust,587 release Rust incl ignored/private compiler acceptance and benchmarks,155 project Python,26 tool tests,112 TypeScript; storage/package/signature/packaged engine passed. Original final CodeScene stage failed on null module-only scores. Fixed conservative applicability reporting; independent review no findings, tester16quality+19orchestrator tests passed; test fixture builder split for LargeMethod finding and six query tests+Clippy rerun passed. Production unchanged after full runtime. Actual final quality rerun passed all19eligible files/no regressions. Original failed and recovery reports preserved under target/verification/task48-full-20261112; evidence/queries/README.md records composite evidence. No waived checks.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added independently usable versioned CLI queries with stable semantic IDs, generation/freshness metadata, bounded agent results and complete generation-bound render selections. Centralized graph selection/flow availability and added a measured CLI-owned query session. Real compiler/process regressions, full runtime/package checks and recovered final CodeScene gate pass. Action log and evidence: evidence/queries/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
