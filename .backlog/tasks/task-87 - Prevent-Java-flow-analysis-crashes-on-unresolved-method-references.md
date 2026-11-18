---
id: TASK-87
title: Prevent Java flow analysis crashes on unresolved method references
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-18T16:01:00Z'
updated_date: '2026-11-18T16:16:00Z'
labels: []
dependencies: []
type: bug
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Java source inventory aborts after successful Maven compilation when javac recovery supplies a method symbol without a declaring type for a reference to a Lombok-generated getter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An unresolved method reference yields a flow diagnostic without aborting inventory or weakening declaration identity.
- [ ] #2 Real Maven and Gradle Lombok cases cover the crash and preserve authored signatures and graph relationships.
- [ ] #3 The original Recurring Billing selection publishes and displays a graph with the fixed desktop adapter.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Extend the existing Lombok fixture and reproduce the failure with the deterministic verification selector.
2. Guard declaring-type lookup in SourceSignatures and retain the existing unresolved flow representation.
3. Run selected verification and CodeScene review, then validate the original desktop journey.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The previous packaged adapter reproduced the ClassCastException with the extended Maven fixture. The fixed adapter published the Maven fixture with the method reference source span, empty targets, and the declaring-type diagnostic. CodeScene scored the Java guard and both fixture/test files 10.0.
On an isolated copy of Recurring Billing, the fixed adapter published 11,442 symbols and 18,257 relationships with the existing Maven skipUi option and the desktop app rendered Graph ready. An unmodified Maven run hit a process-enumeration timeout during the frontend build. The user stopped the retry and requested a local commit. The combined Java and TypeScript selection was separately blocked by existing frontend type errors.
Static checks, Cargo check, Clippy, and unit tests passed. The selected verification plan is recorded under target/verification/changes-o0gotzd4 and was running integration when the commit was prepared.
<!-- SECTION:NOTES:END -->
