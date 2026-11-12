---
id: TASK-49
title: Migrate desktop graph queries to the public CLI interface
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T13:17:00Z'
updated_date: '2026-11-12T16:29:00Z'
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
- [x] #1 Desktop obtains search, selections, neighbors, details, analyzed source, explorer graph information, function flow and freshness through the CLI interface.
- [x] #2 Direct artifact and database graph-query paths are removed; presentation caches and navigation/settings remain desktop-owned.
- [x] #3 Transport and operation layers are maintainable, asynchronous, cancellable and reject obsolete generations.
- [x] #4 Desktop behavior parity, visual acceptance, responsiveness measurements, full verification and CodeScene pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add immutable loaded graph query context (project, linked roots, byte generation) independent of edited root; persist successful context in session, boot/successful analysis obtain snapshots through CLI and remove graph SQLite/artifact backend paths. 2. Separate persistent CLI query transport/cancellation from typed desktop operations; retain asynchronous latest-job generation/epoch rejection and isolate explicit scene fixtures. 3. Route search, exact multiedge neighbors, paged live explorer/current source, symbol data and freshness through CLI; class search activation obtains full class membership explicitly. 4. Feed CLI semantic render selections and cyclic/filter metadata into presentation projection, removing local production semantic selection while retaining grouping/layout/history/camera. 5. Consume CLI-built flow charts with geometry defaults and add CLI architecture query for config/classification required by existing layer UI; desktop owns layout/highlighting/polling only. 6. Verify actual process-backed desktop parity, saved/editing root context, stale/failed/cancelled results, filters/source/flow/explorer and large grouped views; measure responsiveness and canvas performance, independent review/tester, full verification and CodeScene before MCP starts.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-48 is Done with full runtime/package evidence and recovered CodeScene gate. Starting desktop migration only after CLI task acceptance. Preserve live explorer/current hover source versus immutable flow source, generation-bound query context independent of edited project text, and full filtered render membership for grouped counts.

Planner identified additional existing semantic desktop work that must use the CLI: architecture configuration/classification and cyclic/filter summary metadata. These are required migration support, not new user features. Explicit scene fixtures may prepare shared pure query results, but missing production context must report an error rather than silently falling back in-process.

CLI-support unit implemented and independently reviewed: generation-bound uncapped Inspect with exact multiedge snapshot offsets; complete render selections include full-graph cyclic/hidden metadata and optional layer scope/rule signature; live source by path; typed semantic flow charts exclude geometry and source span/UTF8/16MiB validation moved into CLI. Architecture supports current-rule observation, cached classification and explicit configuration-root overrides. Reviewer caught implicit review-config persistence and alternate-root rendering mismatch; fixed with shared load_readonly/Observe policy and same root propagation. Five real-process CLI support regressions pass, including1602node inspector, parallel edges, no query writes, rule changes, root-cache switching and invalid source span; reviewed core/engine files score10.0. Root desktop integration compiling separately.

Desktop migration and final independent review complete; findings fixed with red/green regressions: blocked pipe writes now bounded/nonblocking/cancellable; incompatible capabilities/nonzero exit rejected; corrupt legacy navigation salvages raw project root; reopening search cancels pending class activation and reissues cancelled same query; stalled explorer pages error rather than looping. Independent tester:6transport+12search+1boot+7CLI-support+32projection-layer tests and actualCLI-backedRune journey pass. Ordinary desktop suite217pass8ignored; workspaceClippy passes; CodeScene safeguard72eligible/no regressions. Actual native Metal dark/light flow captures reviewed clean after frame settling. Complete filtered large graph4977nodes5604edges rendered through CLI ~306ms median before metadata-size bounds; final measurements/full gate next. Global diagnostic/freshness query summaries now bounded with explicit omission metadata; snapshot and verify retainfull data.

Final compatibility unit independently verified: successful CLI snapshots retain a write-only profiling cache with separate source/cache generation metadata; full physical query descriptor prevents reuse across relocated checkouts with identical portable artifacts. Raw settings survive corrupt scalar/navigation recovery and subsequent saves. Snapshot decoding preserves saved and fresh analyzer provenance. Independent tester passed 3cache+1boot+4Python snapshot tests; implementation real TypeScript acceptance10 and Clippy passed. Full verification now running at target/verification/task49-full-20261112. MCP remains explicitly deferred by user; desktop will be committed before stopping.

Verification complete: full runtime run target/verification/task49-full-final-20261112 passed 524ordinary Rust tests and611release tests including all ignored acceptance and CPU/GPU benchmarks; Python160project+26tool and TypeScript112passed; storage targets, package, strict signature and packaged engine passed. Final full CodeScene source review found only argument-parser complexity9 while safeguard79/79eligible passed. Extracted option handling without behavior changes; four parser tests, desktop all-target Clippy and independent review passed. Exact final package/signature/packaged-engine/all-source CodeScene stages reran successfully at target/verification/task49-post-review-20261112; final quality79/79eligible, no findings/regressions, cleanup passed. Missing retained-cache-file reviewer regression also red/green: metadata-only availability check rebuilds cache through CLI snapshot. Evidence docs/desktop-cli.md and evidence/ui/cli-backend-2026-11-12/README.md include clean native dark/light captures and honest latency/frame metrics. No MCP code or dependencies created; task50 remains To Do by explicit user direction.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Desktop graph workflows now consume the generation-bound public CLI through asynchronous typed transport. Search, semantic render selections, relationships, source/explorer, architecture, flow and freshness share CLI semantics; rendering, navigation and derived presentation caches remain desktop-owned. Real-process/native workflow, compiler acceptance, performance, recovery/cancellation and packaging verified. Full runtime checks plus final focused parser and package/CodeScene reruns pass. Finish with local desktop commit; MCP explicitly deferred.
<!-- SECTION:FINAL_SUMMARY:END -->
