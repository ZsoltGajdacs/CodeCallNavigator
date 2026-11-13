---
id: TASK-62
title: Add opt-in engine-owned automatic graph refresh
status: To Do
assignee: []
created_date: '2026-11-13T08:06:00Z'
labels: []
dependencies:
  - TASK-60
  - TASK-61
references:
  - TASK-59
documentation:
  - docs/graph-freshness.md
type: feature
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Once saved changes are visible, users need automatic coherent graph updates without repeated manual refresh. Keep scheduling in an engine coordinator and use existing one-shot analysis subprocesses initially so per-refresh cancellation stays separate from monitor shutdown. Full compiler-backed extraction remains the correctness baseline.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Automatic refresh is explicitly opt-in, manual refresh remains available, and one active analysis plus one coalesced pending refresh bounds per-project work.
- [ ] #2 Debounce, invalid-code recovery and lock contention do not loop on unchanged failures or canonical/cache/build-output writes; input changes before publication retain the prior artifact and schedule the latest pending state.
- [ ] #3 Cancellation of one refresh, explicit user cancellation and monitor shutdown have separate outcomes and reap owned descendants without resetting the global cancellation flag.
- [ ] #4 Concurrent desktop/agent/manual writers use the existing publication lock; queries retain generation binding and consumers never combine generations.
- [ ] #5 Per-language publication budgets are derived from TASK-59 evidence and confirmed with at least 20 warm samples on each claimed supported workload; no unmeasured universal latency promise is made.
- [ ] #6 Real compiler concurrency/cancellation/edit-burst journeys, full verification and CodeScene verify failure recovery and process cleanup.
<!-- AC:END -->
