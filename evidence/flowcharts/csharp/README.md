# C# flow export verification

TASK-22.1 adds raw analyzer facts only. Rust ingestion, shared availability and
desktop C# flowcharts are subsequent tasks; exporter evidence alone does not
establish that those workflows are available.

## Before implementation

[pre-export-baseline.json](pre-export-baseline.json) records the exact previous
combined-patch checksum, the upstream test failure names and a real CLI fixture's
complete existing graph projection and source. The original suite had 291 tests:
262 passed and 29 failed. Those failures concern existing assembly-qualified
identity expectations; they are not newly passing or waived acceptance checks.
Compare failure names as well as counts after changes.

The fixture uses the production CallAtlas CLI before exporter changes. To compare
it afterward, recreate its recorded source/file at the recorded task-owned
temporary fixture root, run the recorded CLI operation and compare every existing
JSON field while excluding only
the checkout root and explicitly additive flow fields. This catches unintended
changes to nodes, edge counts/targets, entries, external/unresolved calls,
architecture and framework facts, and unrelated diagnostics. The existing raw
architecture facts include absolute source paths, so using another temporary root
without explicit path rebasing would introduce differences unrelated to flow
export. Verify the existing fixture contents before reusing that temporary root;
keep analyzer output outside the scanned fixture.

TASK-22.2 additionally supplies versioned compiler encoding metadata:
`sourceFactsVersion: "1"` and nullable `architectureFiles[].sourceCodePage`.
The raw acceptance helper validates that capability and code-page types, then
removes only those named additive metadata fields before comparing the original
graph projection. It verifies code page 65001 for the real UTF-8 solution fixture.
Existing architecture source text, paths and semantic facts remain compared;
the historical baseline JSON is unchanged.

## Reproducible patch installation

The previous exact combined patch is preserved as
`patches/callatlas-assembly-identity-pre-flow.patch`, with its checksum in
`analyzers.lock.json`. The current combined patch must include every upstream
source and test addition; an edited vendor checkout alone is not a deliverable.

`python3 -m unittest discover -s tools -p 'test_dependency*.py' -v` exercises
clean/repeated installation, migration from every recorded checkpoint and
preservation of unknown edits, as well as added-file and real-index invariants.
The CallAtlas checkpoint test checks the previous patch against the recorded
pre-export checksum. These tests use disposable repositories.

## Acceptance evidence

Tests-first exporter evidence is recorded in TASK-22.1 notes. The original red
run observed the missing capability/inventory and missing rejection behavior.

Final independent raw CLI acceptance passed on 2026-11-14 against the explicitly
rebuilt Release CLI. [raw-cli-acceptance.json](raw-cli-acceptance.json) records:

- All existing baseline graph fields remained equal: six nodes and six edges;
  six exported flow functions were deterministic across repeated analysis.
- A restored disposable copy of the actual collision solution preserved four
  assembly contexts, twelve flow functions and eight same-named target sites.
- Original UTF-8 bytes containing a BOM, Unicode surrogate pair and CRLF yielded
  the exact source slice `Leaf(s)`. Conditional/deep/wide cases remained opaque;
  each budget overflow produced one callable-level record.
- Intrinsic `nameof(Child.Value)` remains an atomic value without evaluating
  property operands. A real method call `@nameof(Hit())` evaluates `Hit()` first;
  its omitted native binding remains explicitly diagnosed in flow facts without
  inventing a graph edge. The probes use separate classes to avoid shadowing the
  intrinsic with the real method declaration.
- The complete upstream suite executed 319 tests: 290 passed and the same 29
  baseline failures remained. The failed test-name sets match exactly; all 28
  added cases passed. This is baseline-equivalent upstream evidence, not a claim
  that the whole upstream suite is green.

Reproduction commands (the upstream suite intentionally exits nonzero for the
recorded baseline failures):

```sh
dotnet test vendor/callatlas/tests/CallAtlas.Tests/CallAtlas.Tests.csproj -c Release --no-restore --logger 'trx;LogFileName=task22-nameof-suite.trx' --results-directory /private/tmp/task22-nameof-suite
dotnet build vendor/callatlas/src/CallAtlas.Cli/CallAtlas.Cli.csproj -c Release --no-restore
python3.13 tools/csharp_flow_acceptance.py --output evidence/flowcharts/csharp/raw-cli-acceptance.json --trx /private/tmp/task22-nameof-suite/task22-nameof-suite.trx
```

The helper is an explicit compiler acceptance command, outside ordinary Python
unit-test discovery. It refuses unexpected fixture contents, keeps outputs outside
scanned inputs, reports bounded subprocess diagnostics and verifies complete TRX
counts as well as failure names. Its final CodeScene review scored 10.0 with no
findings. Project-wide verification remains a separate gate.

## Final project verification

The final exporter patch passed all 28 stages of
`target/verification/full-38446p00/results.json`, including release acceptance and
CPU/GPU benchmarks, isolated profiling/storage, packaging, signatures, installed
CLI/MCP journeys, CodeScene and protected cleanup. The run used `--timeout 3600`
because the preceding measured real-project acceptance stage took almost 28
minutes. [verification.json](verification.json) preserves the stage outcomes.

The first full attempt failed in an existing observer test's setup: this host's
pipe stalled at 512 queued bytes while reporting writable, so the old 8 KiB
threshold never reached the EOF assertion. A controlled blocking socket stdout
fixture now proves actual sustained backpressure while preserving child-only
writes, real-pipe tests and the two-second EOF bound. Independent tests and review
passed; its Code Health reached 10. This test-only correction is commit `6071db0`.

A full run then passed before the final `nameof` semantic correction. Its result
is retained separately and does not stand in for the final-source pass above.
The current upstream patch and its preserved pre-flow checkpoint match their lock
checksums; all ten patch-installation/migration tests pass.

[Code Health evidence](code-health.json) includes every touched upstream source
and project helper. Focused responsibility splits raised new source/validation
modules to green scores and improved the existing call-graph builder from 4.91 to
5.20. Existing unrelated upstream reader/model findings remained stable. No PR
was opened, so PR change-set analysis is not yet applicable. Sonar is inactive
because this project has no `.sonar/config.json`.
