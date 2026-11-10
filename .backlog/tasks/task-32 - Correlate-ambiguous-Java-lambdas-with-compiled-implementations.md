---
id: TASK-32
title: Correlate ambiguous Java lambdas with compiled implementations
status: To Do
assignee: []
created_date: '2026-11-10T20:49:00Z'
labels: []
dependencies: []
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Java classfile line metadata cannot safely associate every lambda with source syntax, especially same-line, overlapping/nested or missing-debug-line cases. The current exporter uses syntax identities only for proven one-to-one mappings and warns when it retains native compiler identities, which can change after unrelated lambda insertions. The user explicitly deferred exact compiler/source correlation from TASK-31.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Same-line, nested and missing-line Java lambdas receive stable source-derived identities through verified compiled/source correlation without guessing by names or ordinal positions
- [ ] #2 Insertion and reordering preserve unchanged anonymous identities; body edits and identical siblings remain correctly distinguished
- [ ] #3 Compiler-bound graph and flow relationships remain unchanged, with ambiguity controls and real javac regression tests
- [ ] #4 Documented fallback warnings are removed only for cases with proven correlation; analyzer and CodeScene checks pass
<!-- AC:END -->
