# Expanded-result contract checkpoint

The resumed work follows `docs/testing-strategy.md`. These are core unit tests
over real in-memory contracts and transformations; fixture input does not make
them compiler or application acceptance tests.

The first category run observed four expected contract failures. Review then
found child choices could refer to a different displayed parent call; two more
regressions failed before exact structural-address/full-target correspondence
was enforced. Both red checkpoints are indexed in `verification.json`.

Final categorized core verification passed 142 tests, including 17 expanded
contracts, with owned cleanup. Build, preparation, execution and cleanup timing
remain separate in the report. Per-call origins, exact source snapshots, typed
cleanup identities, explicit stop markers and result references are validated.
Ordinary chart and legacy serialized artifact bytes remain unchanged.

Both final CodeScene reports and safeguards passed. Earlier failed safeguard
reports remain intact; a concrete child-location argument replaced the excessive
argument list in the shared traversal. No second semantic walk was introduced.

This checkpoint does not implement expansion assembly, traversal, CLI or UI.
Input-IR membership and compiler eligibility remain assembly responsibilities.
Full verification belongs to completed TASK-22.7 and has not run for this unit.
