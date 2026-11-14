# C# flow ingestion acceptance

This directory tracks TASK-22.2's production-engine integration separately from
the raw CallAtlas exporter evidence in the parent directory.

## Pre-ingestion compatibility baseline

[pre-ingestion.json](pre-ingestion.json) was captured on 2026-11-14 using the
frozen packaged engine from TASK-22.1, before Rust flow ingestion. It records that
engine's SHA-256, exact commands and source encodings, native nodes/edges/entries,
canonical graph projections, and complete historical artifacts.

Three actual net11 projects succeeded through both native analysis and public
canonical publication:

- UTF-8 with a BOM, Unicode and CRLF.
- Valid UTF-16 with a BOM, decoded by the compiler.
- ASCII bytes with explicit compiler `CodePage` 28591 (Latin1), which cannot be
  distinguished from UTF-8 by byte validity alone.

The first two contain three methods and two relationships; the Latin1-configured
case contains two methods and one relationship. All capture commands used the
same verified frozen engine hash.

None of the baselines exported a function-flow inventory into the engine graph. This
proves that adding exact UTF-8 flow spans must preserve the existing UTF-16
graph/source workflow. Artifact source content can intentionally change for
BOM-preserving ingestion; the graph-equivalence checks compare native and
canonical nodes, edges and entries, rather than hiding source changes inside a
whole-artifact equality claim.

The recorded roots under `/private/tmp/task22-ingestion-baseline` are task-owned.
The acceptance helper checks existing source/project contents before reusing
them, preserves original source bytes, and keeps fault-injection outputs outside
the analyzed roots. The baseline JSON remains historical even when a later full
verification replaces the current packaged engine.

## Explicit production CLI checks

Run after rebuilding the intended engine:

```sh
python3.13 tools/csharp_flow_ingestion_acceptance.py --engine target/release/codecallnavigator-engine --output evidence/flowcharts/csharp/ingestion/cli-acceptance.json
```

This explicit integration command is outside ordinary Python unit-test discovery.
It checks generation-bound source-linked flows, overload identities/provenance,
exact BOM/Unicode/CRLF spans, linked files compiled under distinct `#if` contexts,
historical graphs without inventories, and cached source after modification or
deletion with `dotnet` absent from the query PATH. UTF-16 and explicitly
Latin1-compiled ASCII retain native graph equivalence and compiler-decoded source
with visible flow-unavailability reasons.
The plain linked fixture verifies two UTF-8 contexts with different active
conditional call targets. A separate mixed-configuration fixture compiles one
ASCII file under configured code pages 65001 and 28591. It captures unchanged raw
compiler facts from the same successful engine analysis, then checks each
context's flow/source availability against its actual `SourceText` encoding.
Configured and actual pages, actual membership counts and coverage limits are
recorded separately. The probe does not require or retry to force both actual
encoding groups to appear.

[linked-encoding-observation.json](linked-encoding-observation.json) preserves an
initial failed expectation that configured Latin1 always meant graph-only flow.
A subsequent raw metadata probe reported A/B at code page 28591 and C at 65001
despite the opposite configured grouping. That probe was a separate process,
so it is not presented as the exact metadata from the failed run. This observed
configured/actual difference may reflect linked-workspace text reuse; it does
not establish an ingestion defect. Final checks use same-analysis capture rather
than substituting project configuration for actual compiler facts. Uniform actual
encoding groups are reported honestly, with synthetic differing-metadata unit
cases and standalone UTF-16/Latin1 fixtures providing the other policy coverage.

Failure probes use real analysis. Invalid source must leave the previous artifact
unchanged. A task-local `dotnet` wrapper delegates to the real installed toolchain
and mutates only its completed raw map output, removing the inventory or corrupting
an owned callable identity. An out-of-project marker proves successful analyzer
execution and injection before a rejected refresh counts as evidence. A successful
passthrough run validates the wrapper environment; each injected failure must
report the expected missing-facts or absent-callable rejection. Canonical bytes
must remain unchanged for both failure cases.

SQLite, composition/pruning and fabricated raw-boundary validation belong to the
Rust regression suites; this helper does not create an alternate SQLite reader or
graph interpretation. Final CLI/full-verification outcomes are recorded when run.

The tests-first helper run against the frozen pre-ingestion engine failed as
expected with `CLI does not advertise C# flow support`. The helper's current
CodeScene score is 10.0 with no findings after ownership/state and validation
refactoring. Independent final helper review found no blocking issues.

## Independent CLI acceptance

[cli-acceptance.json](cli-acceptance.json) passed 35 real CLI operations against
the frozen explicitly rebuilt release engine. It records that engine's path and
SHA-256, the pinned CallAtlas revision and combined-patch checksum, requests and
generation bindings, process exit codes, and whether `dotnet` was available on
each command's PATH. The engine binary hash remained unchanged throughout the run.

Native and canonical nodes/edges/entries match all three pre-ingestion fixtures.
The UTF-8 source retains its actual BOM, Unicode and CRLF, and each call selects
the correct overload. Flow and analyzed-source queries return the cached snapshot
after current source is edited or deleted, report staleness, and leave graph bytes
unchanged while `dotnet` is absent. Historical inventory-free artifacts remain
graph-readable and give refresh guidance. Failed source analysis and both injected
raw-output failures preserve the prior published bytes, with verified successful
passthrough and rejection reasons.

UTF-16 and explicitly Latin1-compiled ASCII retain compiler-decoded source and
their graph behavior while detailed UTF-8 flow is unavailable with an encoding
reason. Two plain UTF-8 linked contexts preserve independent conditional targets.
The separate same-analysis mixed probe observed actual pages A=65001, B=28591,
C=65001, despite configured A/B=65001 and C=28591. A/C correctly returned flows;
B retained graph/source with an encoding-specific unavailable result. These are
the actual groups observed in this run, not a guarantee that future workspace
loads choose the same groups. The recorded configured/actual distinction is part
of the evidence rather than an inferred ingestion defect.

This is production CLI acceptance. Full project verification, SQLite/composition
tests and native desktop/package acceptance retain their separate evidence and
scope; they are not implied by the 35-operation result.

## Raw compiler source metadata

[raw-source-acceptance.json](raw-source-acceptance.json) passed against the frozen
rebuilt CallAtlas CLI after adding `sourceFactsVersion: "1"` and nullable compiler
code-page facts. The helper validates and removes only that named additive
metadata for comparison with the unchanged pre-export graph baseline. All six
nodes, six edges and existing semantic fields remain equal; six flow functions
are deterministic. Folder parsing truthfully reports unknown encoding, while
the real MSBuild UTF-8 collision solution reports code page 65001.

All prior raw collision/span/budget/`nameof` checks passed. The complete upstream
TRX executed 331 tests: 302 passed, and the same 29 baseline failures remained by
exact test name. No new failed test names appeared. This report is separate from
TASK-22.1's historical raw acceptance rather than overwriting it.

```sh
python3.13 tools/csharp_flow_acceptance.py --output evidence/flowcharts/csharp/ingestion/raw-source-acceptance.json --trx /private/tmp/task22-source-suite/task22-source-suite.trx
```
