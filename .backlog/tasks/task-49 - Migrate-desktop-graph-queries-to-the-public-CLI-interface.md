---
id: TASK-49
title: Migrate desktop graph queries to the public CLI interface
status: To Do
assignee: []
created_date: '2026-11-12T13:17:00Z'
labels: []
dependencies:
  - TASK-48
references:
  - DRAFT-1
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use the desktop as the first consumer of the public CLI query contract so agents and the UI receive the same semantics. Preserve presentation and asynchronous interaction while removing local semantic-query and artifact/database backend paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Desktop obtains search, selections, neighbors, details, analyzed source, explorer graph information, function flow and freshness through the CLI interface.
- [ ] #2 Direct artifact and database graph-query paths are removed; presentation caches and navigation/settings remain desktop-owned.
- [ ] #3 Transport and operation layers are maintainable, asynchronous, cancellable and reject obsolete generations.
- [ ] #4 Desktop behavior parity, visual acceptance, responsiveness measurements, full verification and CodeScene pass.
<!-- AC:END -->
