---
id: TASK-21
title: Add source-derived function flowcharts
status: To Do
assignee: []
created_date: '2026-11-09T19:28:00Z'
updated_date: '2026-11-09T19:45:00Z'
labels: []
dependencies: []
references:
  - /Users/zsolt.gajdacs/Projects/interactive-review/src/flow.ts
  - /Users/zsolt.gajdacs/Projects/interactive-review/shared/types.ts
  - /Users/zsolt.gajdacs/Projects/interactive-review/web/src/flowLayout.ts
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A method call graph does not explain branching within a method. Explore reuse of interactive-review flow.ts, shared flowchart types and flow layout for a separate source-derived detail view. Keep parsing, flow construction and rendering separate from the inter-method graph.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Selected Java and TypeScript methods open source-linked flowcharts with branches, loops, switches, exception handling and returns.
- [ ] #2 Call links use CodeGraph analyzer identities and preserve uncertainty and overload distinctions.
- [ ] #3 Validation covers finally, early exits, callbacks and Reactor pipelines; unsupported constructs are identified.
- [ ] #4 Document language coverage and limitations.
<!-- AC:END -->
