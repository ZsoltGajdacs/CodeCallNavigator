---
id: TASK-60
title: Expose typed CLI-owned saved-input freshness observations
status: Done
assignee:
  - '@codex'
created_date: '2026-11-13T08:05:00Z'
updated_date: '2026-11-13T13:19:00Z'
labels: []
dependencies:
  - TASK-59
  - TASK-63
references:
  - TASK-59
documentation:
  - docs/graph-freshness.md
type: feature
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Saved edits currently wait for desktop polling and freshness is reduced to a boolean. Deliver prompt verified observations while keeping graph loading and watcher ownership separate; saved files only. Extract typed core contracts, an engine observer and explicit capability conversion beside their first consumers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI exposes generation-bound match/stale/unverifiable observations and contextual failures without changing existing one-shot query guarantees.
- [x] #2 Measured supported workloads meet one-second p95 saved-edit stale detection or expose an explicit unsupported/limited mode; commands and at least 20 warm samples are retained.
- [x] #3 Add/delete/rename/atomic saves, missing inputs, linked roots and dropped-event reconciliation preserve input coverage and output exclusions without refresh loops.
- [x] #4 Artifact/index reuse remains query-executor owned; filesystem observation and verification reuse core manifests without desktop/MCP watcher implementations.
- [x] #5 Real compiler and filesystem journeys plus full verification and CodeScene pass, including backward-compatible capability negotiation.
- [x] #6 C# continuous-freshness support is released only after TASK-63 resolves or explicitly documents the measured cold/warm input-coverage discrepancy; unmapped TypeScript ancestor probes remain unverifiable rather than being dropped.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add typed generation-bound freshness observation and capability negotiation without changing legacy query envelopes or one-shot guarantees. Include a live-input revision distinct from artifact generation, conservative coverage and contextual failures.
2. Extend core observation beside verification: share manifest checks, observe actual bytes and membership, and include newly discovered files under recorded directory scopes using existing traversal/exclusions. Prove stale-to-stale and new-file repair revisions with failing regressions first.
3. Keep artifact loading/reuse in the query executor and make index construction lazy for observation-only requests. Extract an engine freshness observer that consumes immutable artifact/digest/roots, without its own graph cache.
4. Add negotiated freshness query plus CLI observe NDJSON stream. CLI owns sequential 250ms reconciliation, initial/change/liveness emissions, shutdown and broken-pipe handling. Publish verified_polling as an explicit limited mode; no native-event claim or universal one-second promise.
5. Test real compiler/filesystem edit, add/delete/rename/atomic-save/missing/linked-root/output-exclusion/recovery journeys and unchanged query/session behavior. Retain at least 20 warm save-to-verified-emission samples per claimed language workload; mark expensive or incomplete coverage limited.
6. Independent review and tester validation, CodeScene and full verification with a sufficient stage timeout; retain evidence, finalize and commit on the current branch before TASK-61.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-63 completed and committed as 3c6b619 after independent review, eight compiler regressions, 18 clean CLI comparisons and 36-stage full verification. Planner and explorer agree on CLI-owned continuous reconciliation and a separate live revision: directory membership hashes alone miss edits to newly added files while the graph is already stale. Initial delivery uses explicit verified_polling limited mode rather than claiming native watchers. No desktop/MCP scheduling policy is added in this task.

Measurement utilities now reuse ProcessMeasurement through a small resource_report interface, with edit orchestration separate from stream transport/reporting. All seven existing harness tests passed outside the sandbox, including actual TypeScript refresh/failure recovery and descendant cleanup; all three modified/new measurement sources scored Code Health 10. Independent measurement review is underway before running the production observer measurements.

Independent measurement review found two issues before collection: reused revisions needed causal sequence/receipt bounds across repetitions, and inherited stdout/closed stdin could defeat bounded shutdown. Corrected both: each edit and reset waits for a later event sequence, pre-save receipts are rejected, nonblocking output reads have explicit cancellation and owned descendants are cleaned by identity. Four real-process regressions pass; shared sampler close-input/resource-report extractions remain small explicit interfaces. Production feature regression was observed red: the prior CLI rejects freshness v1 as an unknown query variant.

Measurement re-review cleared causal sequencing and bounded lifecycle. Recorded reproduction command now includes supplied TypeScript module path and timeout. All eleven existing/new measurement regressions pass on the host (including real compiler refresh/recovery and inherited stdout); all four affected measurement/test files scored Code Health 10. No latency samples have been claimed before the production observer integration is ready.

Independent core tester passed 10 verification and 11 fingerprint tests plus external real-filesystem probes for repairs, nested additions, restoration, custom outputs/explicit generated inputs, linked roots, symlink cycles and cancellation. Found one gap: retargeting a broken symlink between missing destinations did not change revision while complete=true because link identity was skipped for Missing content. Assigned owning-module regression and fix before accepting the core checkpoint.

Core review identified four additional concrete blockers before stream integration: new links to device/FIFO targets could block hashing/cancellation; global union of per-entry exclusions omitted new-file content under a second inclusive scope; Maven generated-only .mvn marker was not excluded consistently; directory-name filtering skipped ordinary files named build/target. Revised ownership premise: every manifest entry owns its logical scope and exclusion policy, so recursive observation must retain that pair and reuse canonical output rules. Implementer paused stream work for red real-filesystem regressions and focused core fixes, including cancellable regular-file hashing and nonregular-target rejection.

Core corrections independently cleared: reviewer passed 15 verification tests (nine filesystem regressions plus six existing); tester passed the same 15 plus 11 fingerprint tests and reran external probes. Broken missing-link retargets, mixed-scope exclusions, repeated new-file edits, Maven output markers and ordinary build/target files are now tracked correctly. Device/FIFO inputs return incomplete coverage without blocking; cancellation interrupted active sparse 8GiB regular-file hashing immediately after a 20ms flag. Full core release suite passes 88 tests, all modified core source Code Health 10. Stream work resumed after both independent roles cleared the checkpoint.

Production observe stream is now runnable; parent real TypeScript smoke (first plus one warm body edit) passed final-revision/sequence binding and clean EOF, not treated as a latency distribution. Public one-shot freshness regression is green, lazy index keeps observation-only queries out of hydration/index construction, and legacy query/session focused checks pass. Real stdout-backpressure test filled the pipe and confirmed stdin EOF terminates within two seconds. Whole wire/stream review and independent process validation now running; excluded node_modules directory-link coverage is being checked before larger measurement collection.

Whole-stream review found an unbounded successful-write acknowledgement channel; replaced it with constant-space latest acknowledged sequence while preserving bounded final-error delivery. The TypeScript smoke also exposed unnecessary incomplete coverage for an excluded node_modules directory symlink; red-first regression and directory-symlink policy fix preserve ordinary build/target files and real nonexcluded directory-link limitations. Core now passes 89 release tests; focused stream six/query six/session two tests pass. Final whole-patch review and independent process journeys remain before full measurements and full verification.

Production measurements complete: 756 raw records (36 first +720warm), four compiler routes x nine scenarios x20warm. Every scenario met the one-second p95 objective on these small fixtures; worst per-route p95 TS262.6ms/C#264.0ms/Maven264.9ms/Gradle264.7ms. Known unmapped inputs remain unverifiable; no large-project/universal threshold claimed. All scopes in the measured run were completely observed; every stream exited cleanly with no sampling/cleanup errors. Raw records compressed losslessly and aggregate assertions checked. Evidence: evidence/freshness/observations/README.md. Required full verification now running with 3600-second stage limit and retained diagnostics.

The first full attempt stopped at static Rust formatting in query/contract.rs before runtime checks. Applied rustfmt only to the 20 touched project-owned Rust files (skip_children=true), with no semantic changes; measured behavior/binary evidence remains valid. Retained the static failure report and restarted the full script at target/verification/task60-retry with the same 3600-second stage limit.

Full verification retry passed all50stages: complete release acceptance/benchmarks, packaging/signatures, installed CLI, independent MCP and agent journeys, CodeScene. Pre-commit safeguard passed24eligiblefiles with no regressions. Sonar remains intentionally inactive (no .sonar/config.json). Evidence/commands/raw756records and verification reports retained under evidence/freshness/observations/. Independent reviewer and tester cleared all source, wire and lifecycle findings before measurements/full gate. Local commit on current branch follows; no push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered typed generation-bound one-shot freshness and continuous CLI-owned observations with explicit limited verified_polling mode, content-derived live revisions, bounded transport and preserved legacy query guarantees. Extracted shared verification/traversal, lazy executor-owned index and small measurement interfaces; fixed independently reproduced filesystem/output/cancellation/transport issues. Four compiler fixture routes x nine edit scenarios x20warm met one-second p95 detection (worst264.9ms); larger workloads remain unclaimed/limited and unmapped inputs remain unverifiable. Independent review/tests,50-stage full verification and CodeScene safeguards passed. Durable action log and evidence: evidence/freshness/observations/README.md.
<!-- SECTION:FINAL_SUMMARY:END -->
