---
id: doc-7
title: Function flow semantics and delivery history
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:13:00Z'
---
# Function flow semantics and delivery history


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-flowcharts-readme-md"></a>

## evidence/flowcharts/README.md

Source: [evidence/flowcharts/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/README.md). Historical record; retained rationale, methods and limitations.

### Function flowchart verification

Verified on Apple Silicon macOS on 2026-11-09.

#### Behavior and regressions

The shared builder tests verify completion-preserving finally/cleanup, return
replacement, resource acquisition failures, labeled loop jumps, switch yields,
short-circuit evaluation, ordered effectful case tests, catch-all handling,
opaque boundaries, detached callbacks and stable Graphviz link identities.
Inventory tests cover composition across supported/unsupported languages, symbol
pruning and visible chart-size limits.

Production acceptance executes actual Java and TypeScript fixture programs and
checks that their observed marker traces follow chart paths. Early returns cannot
resume at statements after finally. Java checks include two-resource acquisition
failure and finally overriding a return. Reactor 3.8.7 executes real assembly and
subscription traces; charts keep eager argument evaluation outside deferred
callbacks and reject lookalike application methods.

Compiler tests also cover overload identity, duplicate contexts, ambiguous lambdas,
Unicode/CRLF byte ranges, metadata persistence and deeply nested source that must
produce bounded partial IR without rejecting the method graph.

Native runtime tests exercise inspector entry, actual target-button navigation,
source-row highlight movement, flow camera/history restoration, unchanged graph
navigation, stale generation rejection, source snapshots without disk files,
old-cache messaging and minimum-window bounds and actual source scroll restoration through A → B → A
visits. Review/testing found and fixed
lost labeled-block siblings, eager nested conditional calls, effectful switch
ordering, a fast initial-fit race, clipped dynamic footer controls and the C#
inspector layout regression. A final long-source test exposed shared scroll-state
ownership across visits; per-visit scroll identities and captured restore offsets
now keep each history entry independent.

#### Checks

- 250 Rust tests passed across regular and real-toolchain runs, with the two
  unavailable private fixtures excluded. This includes 11 shared semantic/layout tests, 3 flow
  inventory tests, 2 real TypeScript flow tests and 5 real Java flow tests.
- Two archived private-project checks were explicitly excluded because their
  `CODEGRAPH_COMPILED_METADATA` / `CODEGRAPH_RECURRING_BILLING` inputs are unavailable.
  These are the same existing private acceptance fixtures documented by earlier work.
- Python suite: 49 passed, including six real JVM inventory checks.
- TypeScript Node suite: 45 passed.
- All-target Clippy with warnings denied and package-scoped formatting passed.
- CodeScene individual source reviews prompted focused refactoring; the final
  safeguard passed without regressions. Type-only module wrappers have no score.
- SonarMCP remains inactive: no local `.sonar/config.json` is configured.

#### Performance and invariance

[TypeScript comparison](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/typescript-export-comparison.json) uses 200 methods and 200
callbacks, producing 602 nodes, 800 edges and 601 flows. Before/after node, edge,
entry-point and diagnostic JSON is identical. Median extraction increased from
92.1 to 155.2 ms; serialized output grew from 625,477 to 1,560,581 bytes, with
935,086 bytes of flow IR. This synthetic measurement describes export overhead,
not a whole-project refresh benchmark.

[Java comparison](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/performance/flow-java.json) uses the same real Maven reactor
paths, compiled artifacts and graph before/after changing only the source inventory.
All 10 nodes and three relationships are identical. The number of subprocesses
remains 14. SQLite grows from 110,592 to 118,784 bytes; flow IR is 9,183 bytes.
Warm refresh samples were approximately 5.9 s before and 5.6 s after; concurrent
machine load makes this unsuitable as evidence of a speed improvement.

Selected-chart results are cached independently of the graph. Source uses virtual
rows, while chart cards and connectors outside the camera viewport are culled.

#### Packaging and visual evidence

The existing Rune checkout has a pre-existing formatting diff and was preserved.
Packaging ran from the isolated staging root established for TASK-20 using the
clean pinned Rune revision, current CodeGraph source and current analyzer artifacts.
The unchanged provenance gate, release build, dependency/license bundling, ad-hoc
signing and strict signature verification passed. `dist/CodeGraph.app` and the ZIP
contain the resulting package; ordinary packaging against the dirty checkout is
not claimed to pass.

[Packaged smoke metadata](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/packaged-smoke.json) confirms real Maven analysis creates
three valid flow symbols and a cached source file. The packaged flow scene also
rendered with PATH restricted to system tools, exercising bundled Graphviz.

[Native captures and reproduction](<doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-function-flowcharts-readme-md>) cover light
and dark appearance at 1420×900 and 1050×680, plus a restricted-PATH packaged capture.
The scene is explicitly synthetic; real compiler and Reactor acceptance are separate.
Physical trackpad and VoiceOver validation were not performed.

#### C# delivery (2026-11-14)

The [C# desktop evidence](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-csharp-desktop-readme-md>) records real compiler/CLI
workflows, owned native light/dark windows at exactly 1050×680, relocated
analyzer/Graphviz resources, source highlighting and target/Back restoration.
Headless compatibility and controlled late-result tests are identified separately.
All 46 full-verification stages pass; all 20 touched source files score 10.
The [cleanup evidence](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-csharp-cleanup-readme-md>) and
[support matrix](<../../../docs/function-flowcharts.md#c-support>) retain the precise
synchronous subset and opaque/conservative boundaries.


<a id="source-evidence-flowcharts-comparison-desktop-catalog-publication-fix-readme-md"></a>

## evidence/flowcharts/comparison-desktop/catalog-publication-fix/README.md

Source: [evidence/flowcharts/comparison-desktop/catalog-publication-fix/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/catalog-publication-fix/README.md). Historical record; retained rationale, methods and limitations.

### Native catalog publication regression

The first final native attempt passed dark but failed light after the Changed
case: its observer read the previous catalog between worker settlement and UI
publication. Both runs certified query cleanup. This failed attempt is retained.

A runtime regression reproduced stale-query failure. The opt-in probe now awaits
the requested echoed query and offset0 before retaining its exact-ID assertion.
Matching-response missing-ID still fails. No workspace/client/default behavior
changed. Independent review and22 release cases passed; both CodeScene reports
passed two scores10 and safeguards. Environment failures are preserved.

The complete full115-stage run precedes this probe-only change. Final light99,
official packaging/signature and both native workflows on the final package
passed afterward, recorded in ../final-verification and ../native-final.


<a id="source-evidence-flowcharts-comparison-desktop-cleanup-reaping-readme-md"></a>

## evidence/flowcharts/comparison-desktop/cleanup-reaping/README.md

Source: [evidence/flowcharts/comparison-desktop/cleanup-reaping/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/cleanup-reaping/README.md). Historical record; retained rationale, methods and limitations.

### Cleanup termination and reaping checkpoint

The actual packaged engine reproduced a parent-owned zombie after verified SIGTERM.
The helper's `waitpid` returned `ECHILD`; the group disappeared only after its parent
exited. This establishes the ownership defect in the original cleanup ordering.
The first native attempt did not record process state, so this repro does not prove
that attempt's remaining member was a zombie.

Failure cleanup now records absent or all-zombie groups as terminated, with zombies
explicitly awaiting parent reaping. Stopped or unknown states do not qualify. After
stopping and waiting for the owned app, every leased group (including zombies) and
private query session must disappear before cleanup is certified. Stable signal
identity excludes transient scheduler state only; successful UI Cancel still requires
full reaping while the GUI is alive.

Independent testing passed all 17 Python cases outside the sandbox, with no skips;
all other 92 previously frozen paths were unchanged at that check. Both changed
files score 10 in both CodeScene versions and their safeguards pass. The actual
packaged-engine green audit confirms awaiting-parent-reap becomes cleared only after
parent cleanup and true absence. Independent review approved the change.

The original and green repros, tests, audits and raw quality results are preserved in
`raw-evidence.tar.gz`; `archive-members.json` gives SHA256 values after extraction.
JSON copies are unchanged. The native-testing documentation was then updated to
explain this two-stage contract and the unlocked/active-display prerequisite.
Native comparison/ordinary C# capture gates remain pending desktop availability.


<a id="source-evidence-flowcharts-comparison-desktop-final-verification-readme-md"></a>

## evidence/flowcharts/comparison-desktop/final-verification/README.md

Source: [evidence/flowcharts/comparison-desktop/final-verification/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/final-verification/README.md). Historical record; retained rationale, methods and limitations.

### Verification sequence

Full run full-tpvcnnp4 passed115/115 outside the sandbox, including real compiler
acceptance, all release ignored tests, CPU/GPU/storage benchmarks on an isolated
profiling copy, package/signatures and installed CLI/MCP journeys.

A final opt-in native observer race was then reproduced and corrected in two
files without changing workspace/client/default application behavior. Final
light-v4vqgrga passed99/99. Official packaging and strict signature verification
passed, followed by comparison and ordinary C# native runs in both themes on
the final package. All8 captures were visually inspected. The full run predates
that probe-only correction; it is not represented as a post-correction full run.

Earlier full-x584jt3l failed compilation and cleanup due to ENOSPC. Only native-
inspected idle generated debug profiles were reclaimed; sources, evidence,
packages, release profiles and global caches were preserved. Its failed result
and the actual recovery record remain here. The successful full and light runs
performed their configured cleanup. No private profiling data is committed.

The index hashes original local reports/logs; source-and-package.json binds all94
implementation files and final binaries. Earlier checkpoints remain historical.

The direct CodeScene MCP pre-commit safeguard also passed:90 checked/eligible
files, no issues. Its original tool response is retained as pre-commit-tool.json.


<a id="source-evidence-flowcharts-comparison-desktop-full-verification-readme-md"></a>

## evidence/flowcharts/comparison-desktop/full-verification/README.md

Source: [evidence/flowcharts/comparison-desktop/full-verification/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/full-verification/README.md). Historical record; retained rationale, methods and limitations.

### Historical full checkpoint

full-l9fmsdlu passed115/115 before the later native visual target-label correction.
Its original results and report index are retained unchanged.

Final evidence is in ../final-verification and ../native-final: a later full115,
then the isolated probe-only correction verified by final light99, official
packaging/signature, and both native workflows in light/dark.


<a id="source-evidence-flowcharts-comparison-desktop-native-final-readme-md"></a>

## evidence/flowcharts/comparison-desktop/native-final/README.md

Source: [evidence/flowcharts/comparison-desktop/native-final/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/native-final/README.md). Historical record; retained rationale, methods and limitations.

### Final native verification

Comparison version2 and ordinary C# version1 native workflows passed on the same
final official package, in light and dark at1050x680 content size. All six
comparison and two ordinary captures were visually inspected: bounded explicit
ellipsis replaces clipped target text; controls, provenance and source remain
readable. Comparison passed26 checks per theme and certified query cleanup.

Cases cover changed/unchanged/added/removed, missing historical source, manual
correspondence/reset, independent highlights/scroll/cameras, target/Back, moved
ref mismatch/Reload, cancellation and a fresh request afterward. Input passes
through actual runtime handlers and Drawn callbacks; OS input delivery is not
measured. Max readiness-to-Drawn was593.12ms dark and588.18ms light.

Original reports, inputs, native traces, owned window leases, cleanup audits and
PNGs are archived byte for byte. The index records hashes. A temporary owned wake
assertion was stopped after verification; no system preference changed.


<a id="source-evidence-flowcharts-comparison-desktop-native-label-red-readme-md"></a>

## evidence/flowcharts/comparison-desktop/native-label-red/README.md

Source: [evidence/flowcharts/comparison-desktop/native-label-red/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/native-label-red/README.md). Historical record; retained rationale, methods and limitations.

### Native visual regression before label correction

The preserved reviewed candidate passed comparison version2 and ordinary C#
version1 native harnesses in both themes after the desktop became active.
The comparison used1050x680 content viewports, actual Drawn frames and owned
window captures. Query cleanup was certified cleared in both themes.

Visual inspection of all six comparison and both ordinary captures nevertheless
found the long target label clipped horizontally in the narrower comparison
inspector. The restored-pair captures show this in both themes. The ordinary
wide target row remained readable. Task22.6 visual acceptance was kept open;
a shared-row regression/fix and final native verification are required.

The archive preserves original reports/configurations/traces/window leases,
cleanup audits and PNGs byte for byte. Runtime harness status is deliberately
unchanged: passing automated checks did not establish visual acceptance.
Its index records sizes and SHA-256 hashes.

The comparison observed readiness-to-Drawn maxima of628.30ms dark and589.67ms
light; these are pre-correction measurements and exclude OS input delivery.


<a id="source-evidence-flowcharts-comparison-desktop-target-label-fix-readme-md"></a>

## evidence/flowcharts/comparison-desktop/target-label-fix/README.md

Source: [evidence/flowcharts/comparison-desktop/target-label-fix/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/target-label-fix/README.md). Historical record; retained rationale, methods and limitations.

### Bounded call-target labels

Two new runtime regressions observed clipping in the actual text child: a1456-point
label inside a324-point control. The shared target row now constrains width and
uses middle ellipsis while retaining the complete accessible label, help, disabled
policy and exact follow target. No vendor change was needed.

Independent review and27 distinct release UI tests passed:13 ordinary,14 comparison,
including both new cases in both themes at1050x680 with native titlebar inset.
Tests inspect presented label/control/inspector bounds and execute navigation.
Both CodeScene reports passed three scores of10; safeguard checked90 eligible
files of261 total. Clippy passed with11 existing Rune warnings.

The three source hashes stayed frozen. Final full/package/native verification is
recorded separately; this checkpoint alone does not establish it. Raw reports/logs
are archived byte for byte, with sizes and hashes in the index.


<a id="source-evidence-flowcharts-comparison-desktop-unit3b-readme-md"></a>

## evidence/flowcharts/comparison-desktop/unit3b/README.md

Source: [evidence/flowcharts/comparison-desktop/unit3b/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/unit3b/README.md). Historical record; retained rationale, methods and limitations.

### Comparison desktop Unit 3B checkpoint

Independent verification passed 63 distinct tests: 48 comparison client/controller/UI
cases, 12 ordinary flow controls, and three real compiler-backed C# cases in release
mode. No tests were ignored or skipped. The [independent report](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/unit3b/independent-unit3b.json)
records commands, prerequisites, engine identity and the 56 source hashes checked
before and after execution. Independent source review approved the same frozen scope.

Comparison tests exercise actual Rune controls backed by Git artifacts and the packaged
CLI: catalog/search/paging, explicit manual counterparts/reset, moved refs requiring
Reload, missing/unavailable/partial states, cancellation and owned query cleanup,
endpoint-local selection/highlight, local zoom coordinates, target/Back history,
rendered source scroll and first-fit camera preservation. Git fixtures use explicit
tag refs; a later unrelated HEAD commit cannot change the saved after endpoint.

The minimum-window checks assert full control and visible rectangles at 1050x680
in both themes with a simulated native titlebar inset. These are headless UI checks;
actual desktop rendering and OS pointer delivery are not established by this checkpoint.
Native comparison acceptance and full verification belong to later task gates.

Both directly regenerated CodeScene reports contain 39 reviews scoring 10 and a
passing safeguard over all 56 modified source files. Scoped Clippy with `-D warnings`
passed; the logs retain existing warnings from the vendored Rune dependency.

The first independent C# attempt ran inside the sandbox and failed three analysis
setups because the MSBuild host could not connect to its pipe. Its bounded run
unwound normally; all known owned descendants were absent before retry. The three
cases then passed outside the sandbox in 27.35 seconds. Failed checkpoint and backend
outputs are retained to distinguish this prerequisite failure from feature assertions.

Red/green logs preserve the Back/source-scroll mount-order regression, camera
remeasurement, ordinary target-help compatibility and disabled unavailable Fit actions.
Temporary tracing was removed before freezing. Whitespace at the end of copied text
logs is normalized; fixture/source bytes and JSON reports are copied unchanged.


<a id="source-evidence-flowcharts-comparison-desktop-unit4-readme-md"></a>

## evidence/flowcharts/comparison-desktop/unit4/README.md

Source: [evidence/flowcharts/comparison-desktop/unit4/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/unit4/README.md). Historical record; retained rationale, methods and limitations.

### Native-harness functional checkpoint

Independent review approved the same 94 source/fixture/document hashes recorded in
[freeze.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/unit4/freeze.json). Independent tests passed 20 Rust native-probe cases and
14 Python gate/protocol/process cases outside the sandbox, with no ignored tests or
skips. The [independent report](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/comparison-desktop/unit4/independent-unit4.json) includes a fresh real .NET
fixture preparation, exact source and mutation checks, ref rebinding and process cleanup.

The fixture covers changed/unchanged/added/removed functions, historical missing
source, explicit manual pairing and a recorded project target. Original compiler
artifacts prove Before differs only by the declared Legacy cached-source row, while
After remains compiler-produced. Cached source matches the actual original BOM/CRLF
input bytes. This host normalizes Git source blobs through `core.autocrlf=input`;
that diagnostic does not change the saved-source contract or cause reconstruction
from Git. A later empty commit retains artifact bytes while changing the comparison
binding. Live C# files and the working graph are removed before native boot.

Both raw CodeScene reports pass: 34 scored reviews at 10, one declaration-only module
without a score, and a passing safeguard over 90 eligible source files (94 changed
files total). Scoped Clippy and static checks pass. Existing vendor Rune warnings
remain in the logs. Version 1 native configuration/report behavior is preserved;
ordinary app launches return before probe setup.

Failure regressions cover terminal report writes, canceled acknowledgments, owned
Git/query cleanup, reused/inconclusive identities and macOS signal-zero permission
behavior. Process status now uses checked numeric process-group listings; failed
inspection cannot certify absence or authorize an unverified signal.

`raw-evidence.tar.gz` preserves the original bytes of every archived output.
`archive-members.json` lists their SHA256 values; the producer's original
`evidence-sha256.json` validates its ten bound members after extraction. JSON copies
are unchanged. Readable logs under `logs/` have only end-of-file whitespace normalized
and should not be checked against the original-byte hashes.

This checkpoint does not establish actual native rendering, capture inspection,
packaging or full verification. Those gates are recorded separately after integration.


<a id="source-evidence-flowcharts-csharp-readme-md"></a>

## evidence/flowcharts/csharp/README.md

Source: [evidence/flowcharts/csharp/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/README.md). Historical record; retained rationale, methods and limitations.

### C# flow export verification

TASK-22.1 adds raw analyzer facts only. Rust ingestion, shared availability and
desktop C# flowcharts are subsequent tasks; exporter evidence alone does not
establish that those workflows are available.

#### Before implementation

[pre-export-baseline.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/pre-export-baseline.json) records the exact previous
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

#### Reproducible patch installation

The previous exact combined patch is preserved as
`patches/callatlas-assembly-identity-pre-flow.patch`, with its checksum in
`analyzers.lock.json`. The current combined patch must include every upstream
source and test addition; an edited vendor checkout alone is not a deliverable.

`python3 -m unittest discover -s tools -p 'test_dependency*.py' -v` exercises
clean/repeated installation, migration from every recorded checkpoint and
preservation of unknown edits, as well as added-file and real-index invariants.
The CallAtlas checkpoint test checks the previous patch against the recorded
pre-export checksum. These tests use disposable repositories.

#### Acceptance evidence

Tests-first exporter evidence is recorded in TASK-22.1 notes. The original red
run observed the missing capability/inventory and missing rejection behavior.

Final independent raw CLI acceptance passed on 2026-11-14 against the explicitly
rebuilt Release CLI. [raw-cli-acceptance.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/raw-cli-acceptance.json) records:

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

#### Final project verification

The final exporter patch passed all 28 stages of
`target/verification/full-38446p00/results.json`, including release acceptance and
CPU/GPU benchmarks, isolated profiling/storage, packaging, signatures, installed
CLI/MCP journeys, CodeScene and protected cleanup. The run used `--timeout 3600`
because the preceding measured real-project acceptance stage took almost 28
minutes. [verification.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/verification.json) preserves the stage outcomes.

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

[Code Health evidence](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/code-health.json) includes every touched upstream source
and project helper. Focused responsibility splits raised new source/validation
modules to green scores and improved the existing call-graph builder from 4.91 to
5.20. Existing unrelated upstream reader/model findings remained stable. No PR
was opened, so PR change-set analysis is not yet applicable. Sonar is inactive
because this project has no `.sonar/config.json`.


<a id="source-evidence-flowcharts-csharp-cleanup-readme-md"></a>

## evidence/flowcharts/csharp/cleanup/README.md

Source: [evidence/flowcharts/csharp/cleanup/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/README.md). Historical record; retained rationale, methods and limitations.

### C# cleanup and conservative control flow

TASK-22.3 uses existing shared version-1 IR for a bounded synchronous subset.
The [support matrix](<../../../docs/function-flowcharts.md#c-support>) separates
structured lowering, conservative exception paths and opaque constructs. Native
identities, relationships and core production completion semantics stay intact.
Flow collection records compiler declaration owners to withhold ambiguous bodies.

#### Focused evidence

[cli-acceptance.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/cli-acceptance.json) records 32 real .NET cases and 202
observed events: 23 supported observable path projections and nine explicitly
partial probes. All 40 ordinary forbidden traces and both filtered-region
missing-finally traces are rejected. Observed operation outcomes constrain
exception edges; observed MoveNext/continue booleans constrain their decisions.
The matcher preserves an observed pending exception through normal cleanup.

Genuine C# lock syntax is checked with gate-evaluation events, Monitor.IsEntered
inside the body, another thread's release probe, and source-owned structural
checks for capture, Enter dominance and guarded Exit. Monitor operations are not
claimed as directly instrumented runtime calls. Null/nullable guards are verified
as conditional structure, without a general runtime-value interpreter. Partial
async/filter/goto/guard/budget probes do not claim complete execution simulation.

The frozen native suite has 355 tests: 326 pass and the same 29 recorded historical
failures remain. Three core completion proofs pass. Four ignored production CLI
C# tests pass, including successful publication of a valid cross-file partial
method whose body cannot safely bind the retained native declaration location.
Raw graph/source/collision/budget/nameof parity is recorded in
[raw-cli-acceptance.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/raw-cli-acceptance.json).

#### Tests-first and review checkpoints

- Six initial native cleanup tests failed against the preceding exporter.
- Three async-boundary reproductions failed before the suspension fix.
- [red-cli.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/red-cli.json) shows the actual using trace rejected by the old
  opaque chart; [first-green-cli.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/first-green-cli.json) shows it matched by
  structured cleanup and rejects a trace that omits disposal.
- [provenance-red.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/provenance-red.json) preserves real raw CLI outputs that
  concatenated bodies under aliased local/generic identities. Compiler-owner
  proof now withholds those flows. Valid-source ownership regressions assert
  zero compiler body errors and empty declaration diagnostics; the intentionally
  invalid accessor case separately preserves its compiler diagnostic.
- [catch-matcher-red-cli.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/catch-matcher-red-cli.json) records a test-matcher
  issue with normal disposal preserving an earlier exception into a catch.
- Typed-fixture conversion and constructor-negative checkpoints are retained in
  [typed-filter-matcher-red.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/typed-filter-matcher-red.json) and
  [typed-constructor-negative-red.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/typed-constructor-negative-red.json).
  Explicit typed identities and preconstructed finally markers remove fixture
  ambiguity without broadening the matcher or changing producer semantics.

The initial safeguard caught loose telemetry arguments and duplicated foreach
fixtures. Typed event/fault/resource identities and shared fault-parameterized
source bodies resolve those findings while preserving real break/continue syntax.
All project-owned source files are 10.0. The first full run exposed a missing
numeric score for a top-level-only fixture; moving its real exit-report hook into
that executable as a local function preserves the lifetime test and makes the
file scored. The exact corrected quality gate and packaged 32-case acceptance
pass. Existing upstream findings remain stable or improve. [code-health.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/code-health.json) records source hashes and findings.

#### Repeat the acceptance

```sh
python3.13 tools/csharp_cleanup_acceptance.py --engine target/release/codecallnavigator-engine --output /private/tmp/csharp-cleanup.json
```

The helper copies its checked-in fixture to a disposable root, builds with the
ordinary .NET 11 SDK, and uses production CLI generation-bound queries. It needs
no frozen historical engine for routine execution. Full verification runs it
explicitly after building the release engine and retains a per-run report.

[verification.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/verification.json) records the full-verification status.
Full verification `full-bkd4h8bc` passed all 34 stages outside the macOS sandbox
with an isolated profiling snapshot. The integrated [final report](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/cleanup/full-cli-acceptance.json)
records all 32 cases against the final patch identity. The manual final report predates a test-only validity-audit patch change;
producer DLL hashes stayed identical, and the integrated full gate records the
final patch identity.


<a id="source-evidence-flowcharts-csharp-desktop-readme-md"></a>

## evidence/flowcharts/csharp/desktop/README.md

Source: [evidence/flowcharts/csharp/desktop/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/README.md). Historical record; retained rationale, methods and limitations.

### C# desktop workflow acceptance

TASK-22.4 uses the production CLI-backed workspace, with no separate desktop
language gate or UI-side inference about missing flow inventories.

#### Verified focused behavior

- Tests-first presentation checkpoint: 9 passed and two expected failures
  (unlocated selection reset source scroll; a known unavailable body kept an
  enabled action). After the bounded fixes, all 11 pass.
- Three release-mode real-CLI desktop workflows pass. The genuine .NET fixture
  preserves BOM, CRLF and Unicode; checks cover source spans, branches, implicit
  cleanup, exact overload targets, possible interface dispatch, target/Back and
  independent camera/scroll history, cached source after edit/deletion, partial
  async and unavailable abstract flows, older inventories and minimum-window
  light/dark layout. These are headless Rune tests, not displayed native windows.
- The real CLI epoch test proves immediate cancellation. Three controlled
  publisher tests additionally prove that late Ready/Error results actually land
  and are rejected for obsolete serial, epoch or symbol without changing the
  current session/cache; a matching-result positive control publishes.
- Eleven native-probe support tests pass, including: strict launch guards, split-frame
  production pointer dispatch, and screenshot acknowledgment validation.
- Independent review approves the implementation and evidence. All 20 touched
  source files score 10 in [code-health.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/code-health.json). The default
  verifier CLI is older than MCP and initially flagged two probe functions;
  cohesive reply/assertion refactors preserve the checks and now pass both
  versions. The exact corrected gate is [retained](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/quality-corrected-cli.json).

#### Measurements

[refresh-overhead.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/refresh-overhead.json) records three alternating warm
refresh samples after one warmup per engine, using isolated copies of the named
fixture and a rebuilt pre-feature `6071db0` CLI with its pinned analyzer patch.
Baseline median: 6.767 seconds and 17,288 artifact bytes. C# flow candidate
`bff3d27`: 7.772 seconds and 24,169 bytes. Existing canonical graph nodes,
relationships and entry points match. The delta includes the whole C# feature
set; it does not isolate exporter CPU cost or establish statistical significance.

#### Native and relocated evidence

[The final native run](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/native-minimum-managed/result.json) passes in both themes
at an observed content viewport of exactly **1050 by 680**. Owned-window images
were inspected: native title-bar buttons are clear, the selected `Leaf(7)` step
highlights source line 116, overload/certainty details remain visible, source
scroll does not move the chart camera, and target/Back/graph restoration pass.
The C# fixture has no unlocated step, so that check is explicitly null here;
the separate unlocated UI regression proves unchanged scroll and target access.

The opt-in probe uses normal saved-descriptor boot and production CLI queries.
Only actual native Drawn frames advance the assertions. A background worker
handles startup/capture acknowledgments and report files. The harness relocates
the complete app, clears analyzer overrides, removes .NET from the GUI PATH,
checks bundled engine/Graphviz paths, and captures only its verified window.
Input is synthetic through production Runtime handlers in a real native window;
OS mouse delivery is not measured.

The successful run records queued pointer-press to first ready Drawn frame:
root charts 136.4/119.6 ms and followed targets 54.8/53.0 ms (dark/light).
These are two observed samples, including CLI/layout work, not latency guarantees.
First-ready sampling occurs during gesture settling instead of imposing its
100 ms delay on the measurement.

AeroSpace initially resized the window after creation. The external harness now
queries only its owned PID and verifies the exact native window ID before using
AeroSpace's per-window floating command. Both final audits record `h_tiles` to
`floating` for their own window and cleanup after the owned app exited. No global
rules/preferences or existing windows are changed. The probe issues one resize
only after setup acknowledgment, waits for 250 ms of matching Drawn sizes, and
rejects subsequent size or window-ID changes. Native flow headers consume Rune's
existing title-bar inset. Nine Python ownership/cleanup tests and eleven probe
support tests pass.

Earlier failures remain available: `native-first` could not render while every
display slept; `native-awake` woke displays but the session was locked.
[native-prerequisites.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/native-prerequisites.json) records timestamped SDK
results. After the user unlocked the desktop, `native-unlocked` passed functional
checks at a larger viewport; it is not minimum-size evidence and its older timing
included gesture settling. `native-minimum` correctly rejected AeroSpace's later
resize. The final managed-window run supersedes those attempts without rewriting
history.

[Relocated offscreen evidence](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/relocated-offscreen-final/verification.json) also
passes the real C# CLI-backed UI test using the relocated engine and Graphviz,
with independently inspected light/dark minimum renders. It is labeled headless
GPU rendering, separate from the native proof. The first extra offscreen attempt
is recorded as not run because verifier cleanup had already removed its binary.

Repeat with an unlocked desktop and a fresh output directory:

```sh
/usr/bin/caffeinate -u -d python3.13 tools/csharp_desktop_acceptance.py --app dist/CodeCallNavigator.app --output /private/tmp/csharp-desktop-new-run
```

The display assertion ends with the command. Inspect both owned-window images.
Full verification `full-0tid44xs` passed all 46 stages outside the macOS sandbox
with isolated profiling. [verification.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/desktop/verification.json) retains the final
results and earlier failed/corrected checkpoints.


<a id="source-evidence-flowcharts-csharp-ingestion-readme-md"></a>

## evidence/flowcharts/csharp/ingestion/README.md

Source: [evidence/flowcharts/csharp/ingestion/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/ingestion/README.md). Historical record; retained rationale, methods and limitations.

### C# flow ingestion acceptance

This directory tracks TASK-22.2's production-engine integration separately from
the raw CallAtlas exporter evidence in the parent directory.

#### Pre-ingestion compatibility baseline

[pre-ingestion.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/ingestion/pre-ingestion.json) was captured on 2026-11-14 using the
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

#### Explicit production CLI checks

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

[linked-encoding-observation.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/ingestion/linked-encoding-observation.json) preserves an
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

#### Independent CLI acceptance

[cli-acceptance.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/ingestion/cli-acceptance.json) passed 35 real CLI operations against
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

#### Raw compiler source metadata

[raw-source-acceptance.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/csharp/ingestion/raw-source-acceptance.json) passed against the frozen
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


<a id="source-evidence-flowcharts-expansion-acceptance-map-md"></a>

## evidence/flowcharts/expansion/acceptance-map.md

Source: [evidence/flowcharts/expansion/acceptance-map.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/acceptance-map.md). Historical record; retained rationale, methods and limitations.

### TASK-22.7 acceptance evidence

The delivered API is shared-core `flow::expand`. CLI and desktop expansion
controls remain TASK-22.8. Existing single-function and comparison workflows
retain their behavior.

| Criterion | Proof |
| --- | --- |
| 1. Exact occurrence, source and relationship ownership | [Result contracts](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit3-readme-md>), [assembly](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4b-readme-md>), [nested occurrences](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit5a-readme-md>), and [real compiler consumers](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit6-readme-md>). Structural IR paths, call slots, cleanup routes and complete targets distinguish occurrences; source bytes and digests remain owner-bound. |
| 2. Supported synchronous call/return and completion | [Builder refactor](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4a-readme-md>) and [assembly](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4b-readme-md>) cover caller continuation, callee and caller cleanup, exceptions, returns and unsupported transfers. [Compiler consumers](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit6-readme-md>) prove actual SDK facts reach expansion through cached artifacts. |
| 3. Active-path cycles, independent repeats and explicit choices | [Nested occurrence matrix](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit5a-readme-md>) covers direct/mutual recursion, repeated siblings, diamonds, candidate selection, request permutation and blocked descendants. |
| 4. Validated bounds without silent truncation | [Budget matrix](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit5b-readme-md>) covers exact and one-under limits, late caller/cleanup work, deterministic repeated demotion, source-owner deduplication, oversized children, mandatory-root failure and cancellation. |
| 5. Conservative unsupported execution | [Assembly](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4b-readme-md>), [nested matrix](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit5a-readme-md>), and [final negative matrix](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit6-readme-md>) cover callbacks/deferred regions, references, HTTP/navigation, async/deferred bodies, missing bodies and exceptions/cleanup. Full choices survive visible stops; no child body or unused child source is invented. |
| 6. Compatibility and immutable inputs | [Invocation contracts](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit1-readme-md>) retain legacy serialization; [ordinary chart goldens](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4a-readme-md>) remain exact. [Compiler consumers](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit6-readme-md>) assert unchanged cached artifact bytes. Expansion does not modify input graph/IR inventories. |

The [final full report](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-final-readme-md>) records current verification after the
two lint cleanups and the independently reproduced [publication-lock lifetime
fix](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-lock-lifetime-readme-md>). Earlier checkpoints are historical evidence, not
substitutes for the final full run. All category scripts use the new inventory
and owned-workspace cleanup paths; no blanket serialization or shortened recovery
timeouts were introduced. The final full run selects all 1,447 registered cases
and eleven standalone gates, with no registered cases or gates left unrun.

CodeScene reviews and safeguards are retained with each implementation unit.
The final full run also checks the complete changed source set. SonarMCP is
inactive because this checkout has no `.sonar/config.json`. No pull request was
opened, so the before-PR change-set check does not apply.


<a id="source-evidence-flowcharts-expansion-desktop-readme-md"></a>

## evidence/flowcharts/expansion/desktop/README.md

Source: [evidence/flowcharts/expansion/desktop/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/README.md). Historical record; retained rationale, methods and limitations.

### Optional call expansion desktop delivery

TASK-22.8 exposes shared bounded call-site expansion through an opt-in CLI query
and desktop controls. Ordinary single-function Flow remains the default.

TASK-22.8 is verified on the corrected source: focused compiler/CLI/Rune,
packaged native acceptance, both Code Health variants and the mandatory full
run passed. The first full attempt remains recorded as a Clippy failure.
See the [acceptance map](<doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-desktop-acceptance-map-md>) for requirement-level proof.

[Focused checkpoints](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/focused-checkpoints.json) preserve successful stages,
expected tests-first failures and the separate owned-cleanup recovery. Raw logs
remain in the named local output directories. Counts describe selected cases;
registrations and partial runs are not claimed as passing executions.

[Source hashes](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/source-sha256.json) bind the corrected executable/test scope.
The [default](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/quality-old.json) and [current](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/quality-modern.json) CodeScene
reports each cover 68 source reviews: 66 score 10 and two contain declarations
only. The extra comparison `pair.rs` review is unchanged source; the five-file
correction is described in [its record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/clippy-correction.json). Both safeguards
passed. The prior [native source freeze](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/native-source-sha256.json) and raw
`native-quality-*.json` reports remain separate. Documentation and final
verification evidence are recorded separately from these source freezes.

[The first full attempt](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-first-results.json) records 77 passed stages,
one Clippy failure and 40 not run. Runtime categories did not execute. Its
[lint log](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-first-clippy.log) and [private input cleanup](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-first-private-input.json)
remain unchanged. Optional expansion storage was boxed at its owning Session,
and four test slices now borrow their input; strict all-target Clippy and 20
affected pure cases then passed with owned cleanup.

#### Native acceptance and measurements

The [focused native result](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/native-result.json), [independent audit](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/native-independent-audit.json)
and [dark](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/native-dark.json)/[light](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/native-light.json) reports pass all 15 checks
per theme in an actual native Metal window at 1050 by 680 points. The harness
analyzes the public C# fixture, checks ordinary cached-source queries, removes
the copied source files, then proves expansion reads the immutable artifact.
Repeated call slots remain distinct; recursion stops visibly; collapsing one
child keeps its sibling; target/Back and collapse-all restore selection, camera
and source scroll. Artifact bytes and generation remain unchanged.

Root inspected both owned-window captures. Native title-bar controls and the
two toolbar rows are clear; the selected helper call highlights cached
`Helpers.cs` line 55, and the explicit target/expand/collapse details fit inside
the viewport. [Inspection scope](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/visual-inspection.json) and screenshot hashes
are recorded. PNGs remain under the local output path in the audit rather than
being duplicated in Git. Input is synthetic through production Runtime handlers;
this checks a real displayed workflow, not OS mouse delivery.

| Measured work | Observed values |
| --- | --- |
| Initial C# analysis | 17.877 s |
| Package stage, including compilation | 90.566 s (compiler: 56.22 s) |
| Native gate, both themes | 26.244 s |
| Query plus layout edits: queued pointer press to first new Ready Drawn frame | 75.15–109.82 ms |
| Collapse-all: queued pointer press to first restored Ready Drawn frame | 17.65–17.73 ms |
| CLI query, including cold negotiation and client teardown | 30.63–53.44 ms |
| Graphviz layout | 22.97–47.94 ms |
| Category cleanup | 2.075 s |

These are six readiness observations per theme, not latency guarantees or a
statistical benchmark. Timing is sampled before gesture-settling delay, and
reports retain step/link counts (14/13 to 41/45). Query/layout are absent for
collapse-all because it restores the retained ordinary chart.
[Native cleanup](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/native-category-cleanup.json) reclaimed 1,363,498,077 bytes of
owned outputs and retained only the allowed prior-package artifacts; query
process cleanup also passed in both themes.

#### Final full verification

`verify-full.sh --base HEAD` passed **118/118 stages** outside the macOS sandbox
with isolated profiling inputs and a disposable private Maven project copy.
[Raw stages](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-results.json), [timings/selection](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-summary.json) and the
[independent audit](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-independent-audit.json) are retained. The full command
and input cleanup are in [the wrapper record](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-private-input.json).

All 1,476 unique selected test registrations and 12 gates were covered by
successful stages, with zero not-run selections. These are selection counts,
not an inferred total of framework-reported passing tests. Compiler-backed
integration, CLI/MCP/installed-agent acceptance, ordinary/comparison/expansion
native journeys, performance, packaging and CodeScene all passed.

| Full recorded stage time | Seconds |
| --- | ---: |
| Compilation | 357.488 |
| Setup | 10.432 |
| Test/gate execution | 1,457.376 |
| Cleanup | 48.670 |
| Other static/bootstrap/package work | 127.507 |
| Total recorded stages | 2,001.473 |

Package work includes its own compilation; the raw stage log and focused
package breakdown above retain that cost rather than treating it as test time.
The summary keeps every category's compile/setup/execution/cleanup separately.
No ordinary tests were repeated in debug and release; release profiling and
strict all-target static linting have separate purposes.

All five category/check workspaces passed cleanup and their removed paths are
absent. Reports are retained for [checks](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-checks-cleanup.json),
[unit](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-unit-cleanup.json), [integration](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-integration-cleanup.json),
[acceptance](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-acceptance-cleanup.json) and [performance](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-performance-cleanup.json).
Only allowed prior-package artifacts were retained. The private original's
fingerprint is unchanged; its owned copy was removed after inactivity checks.
All 72 [full-run source hashes](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-source-sha256.json) match before/after/current
bytes, including the intentional deleted encoder. The 71-path reviewed source
freeze additionally includes unchanged comparison `pair.rs`.

The final boxed-Mode package passed native expansion again:
[result](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-native-result.json), [dark](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-native-dark.json) and
[light](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-native-light.json). First-ready edits measured 96.92–118.71 ms,
collapse-all 16.80–17.82 ms, query work 43.03–58.40 ms and layout 23.20–32.69 ms.
These are twelve observed readiness samples across both themes, with the same
representative 14–41 steps; they are not worst-case performance guarantees.
Root inspected six final ordinary, expansion and restored-comparison captures;
[visual observations and hashes](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/full-native-visual.json) are retained.

SonarMCP is intentionally inactive because this checkout has no
`.sonar/config.json`. No required behavioral/quality gate was waived. The final
[direct MCP pre-commit safeguard](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/pre-commit-safeguard.json) passed after staging
the source: 67 eligible files checked, no issues. Only task/evidence metadata
changed afterward. PR change-set analysis does not apply to this local commit delivery.


<a id="source-evidence-flowcharts-expansion-desktop-acceptance-map-md"></a>

## evidence/flowcharts/expansion/desktop/acceptance-map.md

Source: [evidence/flowcharts/expansion/desktop/acceptance-map.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/desktop/acceptance-map.md). Historical record; retained rationale, methods and limitations.

### TASK-22.8 acceptance map

| Requirement | Observable proof and boundary |
| --- | --- |
| Opt-in, versioned, generation-bound CLI; ordinary Flow compatibility | Core wire/capability unit tests; real engine `expansion_cli_is_generation_bound_and_keeps_ordinary_flow_unchanged` and `expansion_cli_reports_stale_choices_limits_and_escaped_transport_bounds`. The process tests check cached ownership, invalid structural addresses/slots, stale generation, limits and complete escaped transport bounds. |
| Explicit target and occurrence expand/collapse, source ownership and stops | Pure controller/core fixtures distinguish candidates, siblings, recursive markers and shared stops. The real C# compiler/CLI/Graphviz/Rune journey proves assembled controls and visible file/span highlighting. Native reports additionally prove two distinct root slots, repeated child expansion and visible cycle stops. |
| Background cancellation, stale replies, exact ordinary restoration | Publication tests bind serial, epoch, symbol and generation, with matching-result positive controls. Captured-action tests reject obsolete Ready/result/selection bindings. Initial Cancel and pending-history cases retain the correct previous state. Real Graphviz process probes prove blocked-work cancellation and reaping; native collapse-all checks exact retained ordinary state. |
| Navigation, camera and scroll | Three cheap actual Rune tests prove same-source unrealized-row centering, changed-source centering without offset overwrite and strict minimum-window actual/target control geometry. Native target/Back proves selected occurrence, camera and source scroll restoration, sibling-preserving collapse and graph restoration. |
| Bounds, semantic limits, Code Health and measurements | Product and CLI docs describe defaults/ceilings, immutable-source ownership and conservative execution meaning. Raw source reviews/safeguards, native first-Ready Drawn/query/layout measurements, visual inspection and category cleanup records accompany this delivery. Corrected full verification passed all 118 stages with isolated inputs and no not-run selections. |

Pure tests use actual core expansion from explicit in-memory flow IR, not mocked
successful analyzer output. The compiler-backed fixture separately verifies the
real C# path. Rune integration tests prove layout/input transactions; the smaller
native journey proves the packaged application and actual Drawn frames. Native
input is synthetic through production handlers, not OS event injection.

The first full attempt stopped at Clippy before runtime suites. Its failed stage,
not-run stages and successful cleanup remain separate from the corrected run.
The optional expansion state is now boxed at its owning Session to preserve the
ordinary/comparison footprint; borrowed single-item test slices remove needless
clones. The existing comparison representation remains unchanged.


<a id="source-evidence-flowcharts-expansion-final-readme-md"></a>

## evidence/flowcharts/expansion/final/README.md

Source: [evidence/flowcharts/expansion/final/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/final/README.md). Historical record; retained rationale, methods and limitations.

### Final expansion verification

The final mandatory full run, `task22-final-full-r4`, passed all **133 stages**. The independent audit checked its actual result records, four category selections and all 11 corresponding gate stages. The selections contain 1,447 unique inventory registrations, not an asserted actual passing-test count. Raw subprocess/self-test output can repeat test names; it is not counted as extra independent coverage.

All 108 recorded source hashes match before, after and the current files. Compilation, setup, execution and cleanup timings remain separate in `verification.json` and the raw summary. Every checks/category cleanup passed; every removed or absent owned build/tmp/profile path is currently absent. Only the documented newest previous packages remain retained by policy. The separate real private Maven copy was reclaimed (270,438,581 bytes), and its original checkout fingerprint remained unchanged.

The archive retains the first two full failures at Clippy, their two narrow fix/quality checkpoints and the all-target Clippy diagnostic; the third full failure at cache-clear lock contention; and the successful fourth run. Earlier failed runs are not relabeled, and their unexecuted later stages remain not run. The deterministic lock regression red/green evidence is separately indexed in `../lock-lifetime/`.

Both native desktop acceptance gates passed using public compiler fixtures in light and dark themes. Root inspected the captures; this report audits their terminal records and does not claim another native run or independent screenshot inspection.

`reports.tar.gz` contains only raw JSON/log/stderr evidence with per-member SHA256 and byte counts in `archive-index.json`, verified by reading the archive back. Private graph/source/artifact/profile data, binaries and packages are excluded. Final pre-commit checks, task closure and commit remain Root-owned.


<a id="source-evidence-flowcharts-expansion-lock-lifetime-readme-md"></a>

## evidence/flowcharts/expansion/lock-lifetime/README.md

Source: [evidence/flowcharts/expansion/lock-lifetime/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/lock-lifetime/README.md). Historical record; retained rationale, methods and limitations.

### Explicit project-lock lifetime verification

Mandatory full-r3 exposed a second consecutive cache-clear call returning `LockBusy`. The original full results and runtime log are preserved in the archive. This is a bounded prerequisite repair, not a full-verification pass.

Two real-filesystem regression tests retain a cloned locked descriptor across guard drop. Before the fix, both fail immediate reacquisition; the original cache-clear failure also reproduces (57 passed, 3 failed). Later test artifacts are not run after that failing library artifact.

The acquired-only `LockedFile` guard explicitly unlocks on drop. Failed contenders cannot unlock the owner; double-Busy controls, old-clone closure preserving a new owner, immediate reacquisition, and independent lock types remain covered. No retry or broad serialization masks contention.

The green core integration category passes all 111 distinct selected tests with zero failures or ignores. All five stages and owned cleanup pass; source SHA is identical before and after. Timings and cleanup bytes are in `verification.json`. Both raw CodeScene versions score the modified file 10 and pass safeguards.

`category-reports.tar.gz` contains only raw JSON/log/stderr reports for the red/green categories and the bounded original full failure. Its per-member byte counts and SHA256 values are in `category-report-index.json`, verified by reading the archive back. No binaries, packages, or retained build workspaces are archived. The real descriptor proof does not identify which historical fork retained a descriptor; final full/package/native verification remains separately owned by Root.


<a id="source-evidence-flowcharts-expansion-resumed-testing-coverage-map-md"></a>

## evidence/flowcharts/expansion/resumed-testing/coverage-map.md

Source: [evidence/flowcharts/expansion/resumed-testing/coverage-map.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/resumed-testing/coverage-map.md). Historical record; retained rationale, methods and limitations.

### Resumed testing coverage map

Read-only audit on 2026-11-15 after the `d792304` testing-strategy change. This
records the current source and inventory, not a passing verification of the
in-progress Unit 3 implementation. No builds or tests were run for this audit.
Only this evidence document was written.

#### Capability-test migration

The two engine libtest identities remain unchanged:

- `analyzers::typescript::tests::stale_missing_and_future_typescript_adapters_require_matching_resources`
- `analyzers::typescript::tests::current_typescript_adapter_is_accepted_with_unchanged_prepare_contract`

Previously, `prepare_adapter` created a temporary `analyze.js` that fabricated
successful compiler metadata, wrote that metadata to the requested output, and
kept a Node process alive on stdin. Each invalid-version combination launched a
replacement worker and called production `prepare`. This was engine integration
using a business-result double, not a successful real TypeScript journey or a
necessary protocol fault probe. Its unique intended assertion was resource-version
policy; the synthetic configuration and worker provided no additional compiler
evidence.

The replacement tests call production `validate_capabilities` with explicit
in-memory JSON. Production `prepare` calls that same function immediately after
decoding actual worker metadata. There is one policy implementation, no copied
predicate and no replacement compiler/worker. The tests are now ordinary,
unignored engine **unit** tests with an empty requirements list in
`tools/verification/test-inventory.json`.

| Original defect/assertion | Retained direct proof | Boundary proof retained |
| --- | --- | --- |
| Missing, stale `"6"`, future `"8"`, numeric `7`, or null adapter accepted | The first identity supplies each value with valid `boundary_rules:"1"`; each must return an error containing rebuild guidance. Removing the adapter-version guard makes `expect_err` fail. | Real CLI TypeScript analysis consumes actual metadata through `prepare`, then publishes/query-checks the resulting artifact. |
| Matching adapter rejected | The second identity requires `adapter:"7", boundary_rules:"1"` to succeed. | Real current adapter and compiler are exercised by the public CLI journeys and Node compiler fixtures. |
| Boundary capability weakened | The second identity additionally rejects missing, null, numeric `1`, and unsupported string `"2"` boundary versions. | Existing TypeScript preparation/configuration/compiler fixtures remain registered as engine integration. |

Observed original red evidence is retained at
`/private/tmp/codecallnavigator-task22.7/target/unit2c-verification/capability-red.log`:
the stale/missing test failed with **“Stale/unknown adapter must be rejected”**,
0 passed / 1 failed. Its hash is recorded in
`evidence/flowcharts/expansion/unit2c/report-index.json` as
`780d030a943fa942061e2edf1d07d7ce95f0790d97c53dfbff2b868c400cac72`.
The new unit assertions exercise the precise missing version-check failure,
rather than relying on matching test names. After the initial preparation audit,
the parent ran the engine unit category. Independently inspected
`target/verification/task22-resumed-engine-unit/04-unit-test-rust.log` contains
both exact identities passing, unignored. All six category stages passed in
`results.json`, including category cleanup. This is an independently inspected
parent-run green, not an additional tester execution or mutation trial.

Historical cost, not a current benchmark: the old two-case release run recorded
1.04 seconds of libtest execution; its independent batch recorded 5.328 seconds
wall time including Cargo work. These observations are in the retained Unit 2C
independent report and `independent-capability-release.log`. There is no measured
isolated replacement-pair timing, so this audit claims no measured speedup. The
new category report separates compilation 10.636s, preparation 0.958s, Rust test
stage 2.351s, Node unit stage 1.012s and cleanup 1.078s (prerequisites 0.902s).
Those whole-stage figures include other selected tests and cannot be compared
directly with the old two-case execution figure.

#### Retained coverage and classification

| Coverage | Component/category | Requirements and reason for this layer |
| --- | --- | --- |
| The two capability-policy identities above | Engine / unit | In-memory JSON and the actual production validator; no Node/compiler/process requirement. |
| `real_typescript_facts_survive_stable_ids_cached_queries_and_excluded_targets` in `engine/tests/typescript_facts.rs` | Engine / acceptance | Inventory requires Node and TypeScript; Cargo builds the actual CLI. A cheaper layer cannot prove analyze → canonical publication → public cached query. |
| `real_typescript_analysis_is_queryable_without_reanalysis` in `engine/tests/query.rs` | Engine / acceptance | Inventory requires CLI, Node and TypeScript. Retains the existing complete query journey. |
| `tools/typescript/adapter.test.js` and `flow-export.test.js` real source/compiler fixtures | Engine / integration | Node and installed TypeScript. Real compiler declaration ownership, overloads, configuration inputs and source evaluation need compiler boundaries. |
| Shared invocation-fact/legacy serialization checks | Core / unit | Explicit IR/artifact fixture inputs prove strict facts, uncertainty and exact legacy bytes; fixture loading does not make these integration tests. |

The new CLI journey analyzes the actual fixture using
`CARGO_BIN_EXE_codecallnavigator-engine`; it asserts synchronous execution,
isolated/direct invocation and stable owned IDs, prunes a real target while
preserving invocation facts, deletes `main.ts`, and queries with PATH restricted
to `/usr/bin:/bin`. It checks cached Unicode source, returned facts and unchanged
artifact bytes. These are real outputs, not fabricated successful services.

Retained actual compiler fixture identities include:

- `type errors reject analysis and config preparation reports engine inputs`
- `compiler bindings retain anonymous callback ownership and overload implementations`
- `runtime import aliases resolve one authored overload implementation`
- `simple ordinary function calls prove only their isolated final invocation`
- `generator parameter defaults are a synchronous prefix before a deferred body`

Protocol/process failure probes elsewhere are not replaced or presented as real
successful journeys by this migration. No test is removed solely to meet a time
aspiration, and no correctness/recovery timeout is shortened.

#### Commands and readiness

After freeze, use the shared category execution paths, without also repeating the
same ordinary tests in release:

```sh
./verify-unit.sh --component engine
./verify-integration.sh --component engine
./verify-acceptance.sh --component engine
./verify-unit.sh --component core
./verify-full.sh --dry-run
```

The first selects the two migrated policy tests; integration retains real Node
compiler fixtures; acceptance retains both public CLI journeys. Exact selected
registrations, requirements, builds, setup, execution and cleanup must be read
from the post-freeze category reports. Full remains required for the overall
cross-component/analyzer change and is owned by the parent task.

At this audit checkpoint, unit-engine, acceptance-engine and full **dry runs were
unavailable**: inventory discovery referenced the not-yet-created
`crates/codecallnavigator-core/tests/flow_expanded_contract/identity.rs` during
the active Unit 3 test split. Each returned exit 2 before any build/execution or
evidence creation. This was a transient preparation observation, not a waived gate;
the later parent engine-unit run successfully selected and executed the policy
tests. Repeat the remaining selection checks after final freeze.
`verify-unit.sh --help` successfully confirms
component/suite/dry-run/profile/timeout/keep-artifacts options.

Current shell inspection (no secrets or private project contents read):

- Cargo/Rust, Node, Java/javac, dotnet, Maven, Graphviz, codesign, xcrun, lsof and
  cs-mcp resolve on PATH. The local verification Python exists. Tool presence is
  not toolchain/version/native-access preflight success.
- `CODECALLNAVIGATOR_TS_NODE_MODULES`, `CODECALLNAVIGATOR_RECURRING_BILLING` and
  `CODECALLNAVIGATOR_VERIFY_PROFILE_SOURCE` are unset; no conflicting
  `CODECALLNAVIGATOR_TEST_TYPESCRIPT_ROOT` is set.
- The previously used modules candidate
  `/path/to/node_modules` exists;
  its package metadata reports TypeScript 6.0.3, React 19.3.1 and Next 16.4.2.
- Prior full `full-tpvcnnp4/results.json` explicitly supplied profile source
  `/path/to/Application Support/CodeCallNavigator`; that path
  and `catalog.sqlite3` exist. No private catalog contents or active-graph
  readiness were inspected. Future performance tests must use the runner's copy.
- The exact historical private Recurring Billing path was not recovered from the
  prior configuration inspected. The parent subsequently supplied the current
  fixture candidate `/path/to/private-maven-project`; its `pom.xml`
  exists. This is not proof of repository access or successful compilation.
  Supply the required environment explicitly and let the category own an isolated
  fixture when building could modify the checkout.
- `.sonar/config.json` is absent. CodeScene remains mandatory; no new quality
  verdict is claimed by this read-only audit.

Full preflight also requires Apple Silicon macOS/Metal and process inspection,
Node 24+, JDK 25, .NET SDK 11, Maven, Graphviz and signing tools. Run outside the
sandbox. No native/display/package/performance readiness is established here.

Documentation discrepancy flagged to the parent: `docs/testing-strategy.md` and
`tools/verification/suites.py` disable Cargo automatic cache cleaning with
`CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never`, while `docs/reference.md` still says
age-based downloaded-cache management remains enabled. No out-of-scope doc edit
was made.

#### Unit 3 red checkpoint remains separate

`target/verification/task22-unit3-red/04-unit-test-rust.log` records four expected
expanded-contract failures: structural owner address validation, visible call
origin serialization, full-address cardinality/distinctness, and explicit visible
stop markers. This is tests-first evidence, not a current implementation green.
The category cleanup report says passed, reclaimed 534,543,924 bytes, retained
zero, with no errors. Its timing report separates compilation 9.7s, setup 1.744s,
test 11.369s and cleanup 1.049s. No unrelated failure or retained-output cleanup
claim is inferred from that checkpoint.

#### Independent final Unit 3 source/result audit

The producer's frozen result-contract implementation was independently reviewed
without executing the category again. Exact evidence:

- `target/task22-unit3-contracts/freeze-sha256.json`: all ten file hashes matched
  on two audit reads. Manifest SHA-256:
  `4941b72136687e42f366038d7e71f0c24d699735f0c8dbd45e6aa7f95d2a8acf`.
- `target/verification/task22-unit3-final/results.json`: all five category stages
  passed, with exit code zero.
- Actual libtest summaries and test-name lines independently total **142 distinct
  passed / 0 failed / 0 ignored**, matching 142 selected and zero not-run entries.
  The **17 expanded-contract cases are included in 142**, not additional passes.
- `unit/category-cleanup.json` reports passed, 537,849,438 bytes reclaimed,
  zero retained, no errors and no unmeasured paths. All paths recorded as removed
  were independently confirmed absent. No test process or build was started by
  this audit.
- Timings remain separated: prerequisites 0.891s, preparation 0.950s, compilation
  9.808s, tests 10.175s, cleanup 1.059s. These are category-stage observations,
  not a cold/warm benchmark or an isolated 17-test timing claim.

The tests exercise production public contracts with explicit in-memory inputs:

| Observable contract | Assertions inspected |
| --- | --- |
| Compatibility and serialization | A real ordinary `flow::build` chart retains exact golden chart bytes inside a root-only result; full result serialization/deserialization is stable. |
| Owned source interning | Exact BOM/Unicode/CRLF bytes and digest preserved; same owner/content deduplicates, distinct owner stays distinct, conflicting content under one owner rejects. |
| Stable structural sites | Equal spans across branches, nested expressions, deferred regions and catch/finally/cleanup blocks yield distinct typed owner paths and slots; static enumeration invents no execution copies. |
| Cleanup/repeated occurrence identity | Full original scope, pending route, outer-to-inner context, target certainty/provenance and repeated choices remain distinguishable through hashing and serialization. These tests prove identity semantics, not cleanup execution or assembly. |
| Visible source/origin integrity | Missing or duplicate full call origins, invalid IDs/links/regions, bad source digest/owner/index, reversed ranges and UTF-8 splits reject at `ExpandedFlow::validate`. |
| Stop/boundary integrity | Stopped work needs a visible nonblank marker and explanation; cannot claim assembled entry/return/throw boundaries or unrelated owners. |
| Result-local child correspondence | Both stopped and expanded children must match an exact displayed parent site and full target. Mutating certainty, provenance, relationship kind, call slot or explicit call boundary rejects; a valid setup is validated before each negative family. |

These are core **unit** contracts, not mocked compiler services. Mutating a valid
serialized result and observing the actual validator's rejection exercises the
external-data boundary directly. A shared fixture constructs data and calls real
ordinary chart building; it does not reproduce validation decisions. No unique
missing behavior was found within this unit's result-only scope, so no additional
tests or source changes were proposed.

The stronger child-correspondence red is separately retained at
`target/verification/task22-unit3-correspondence-red/04-unit-test-rust.log`
(15 contract passes / 2 failures), before the final green. Both final CodeScene
raw reports (`quality-final-old.json`, `quality-final-modern.json`) pass their
safeguards: nine scores are 10 and the declaration-only `expanded.rs` is null,
not an invented numeric score. Earlier failed quality reports remain preserved.

The engine-unit evidence still contains both pure TypeScript policy identities
passing. The original fabricated-worker source backup remains intact at
`/private/tmp/codecallnavigator-task22.7/crates/codecallnavigator-engine/src/analyzers/typescript_tests.rs`,
SHA-256 `df6b95c846b89123ee828e8a145b82758ec84ad09bdbe861c19e7e284d081e76`.
The private backup's Unit 2A/2B/2C manifests (22/19/16 files) still match. Its
earlier Unit 1 `flow.rs`, `model.rs` and `parts.rs` hashes were intentionally
superseded by partial Unit 3; this audit does not claim the whole old Unit 1 tree
is unchanged.

Scope remains limited: no expansion API, callee assembly, immutable-input IR
membership/eligibility validation, new consumer execution, native acceptance or
full verification is established by these result-contract units. Later assembly
must enforce input-backed eligibility and source ownership independently.

#### Unit 4B single-call expansion audit and root-only seam

The original frozen Unit 4B was independently audited from source and
`target/verification/task22-unit4b-final`, without repeating its category merely
to obtain another count. Its actual log contained **155 unique passes, zero
failures/ignored**, matching 155 selected and zero not-run registrations. The 12
expansion cases were included, not added again. All five stages passed.
Preparation/prerequisites totaled 1.844s, compilation 9.522s, test execution
13.646s and cleanup 1.153s. Cleanup reclaimed 554,738,835 bytes, retained zero,
reported no errors/unmeasured paths, and all removed paths were absent.

All 18 source hashes matched the producer freeze during that audit. Its manifest
SHA-256 was `b54e4ca37c7535c2ccb0fe879909b87e2d11617e861a6fd4dbb40ac05f23e1b1`.
The three legacy invocation-fact IR/chart/artifact goldens still match Unit 1.
The eight-chart ordinary builder golden remains 17,644 bytes, SHA-256
`32fcd5279b59b32f42e48dc80cf5491af7c23c29effbada5c4a5638cd1d4a6e6`.

Observable topology assertions inspected:

- Child entry/body is reachable; removing the child entry prevents reaching the
  caller continuation, and child return boundaries resume the caller's original
  return or decision. The resumed decision has no duplicate call metadata.
- Child throw and unknown leaf failure reach child cleanup before caller cleanup,
  with an outer catch path. Pre-body call failure independently reaches the caller
  while bypassing the child. A throwing finally suppresses child return and the
  caller's otherwise normal continuation.
- Cleanup-copy selection identifies the original resource/finally/unpaired scope
  and semantic pending route, and expands exactly one matching copy.
- Missing/unknown/deferred/reference eligibility produces a visible stop, retains
  the caller continuation and does not retain rejected child source snapshots.
  Virtual/candidate selections preserve full uncertain target facts.
- Coincident spans and multiple calls do not manufacture isolation; stale target
  evidence or nonexistent cleanup context errors, and cancellation rejects.

`reaches` observes the returned graph using a visited-node set. `labels_from`
enumerates simple paths through these small explicit fixtures with a per-path
visited set, preventing cyclic revisits. It can be combinatorial on arbitrary
graphs and is not presented as a large-graph algorithm or runtime trace oracle.
Neither helper simulates lifetimes, eligibility, values, exception unwinding or
compiler decisions: those behaviors are asserted on actual production chart
topology, boundaries, source ownership and call metadata.

Both review defects are preserved in
`target/verification/task22-unit4b-review-red/04-unit-test-rust.log`: 10 expansion
passes and two failures. The final green proves a reference-only leaf does not
invent a callee throw boundary, while empty/mixed/unknown targets remain
conservative and pre-body failure remains possible. It also proves valid legacy
Action/implicit cleanup selection produces an ineligible marker rather than a
stale-selection error. Both final raw CodeScene reports pass; 17 scored files are
10 and the declaration barrel is null. Earlier failed/intermediate results remain
preserved by the producer.

##### Unique missing seam found and covered

All 12 original expansion cases selected a root call. `ExpansionRequest::new`
also allows no selection, while the older root-only contract test wraps ordinary
`flow::build` directly. It therefore did not exercise the new expand → record →
remap → validate seam. This was a concrete coverage gap, not a demonstrated
production bug or a reason to duplicate existing tests.

The parent authorized and registered one new **core unit** case, with no external
requirements:

`codecallnavigator-core/test:flow_expansion::root_only::unselected_expansion_preserves_ordinary_cleanup_chart_and_owned_call_origins`

The test calls actual `expand` with no selection and only an immutable root input.
Its input includes acquisition/cleanup/finally, return/fallthrough branches and
multiple calls whose callee bodies are unavailable. It requires byte-identical
ordinary chart output, exactly one root occurrence and source, exact source bytes
and ownership, root-owned origins and complete call-origin cardinality. This
proves no eager callee lookup/assembly is required without an explicit selection.
It uses explicit input data and public production APIs, not copied builder rules
or a fabricated successful compiler. Only
`tests/flow_expansion/root_only.rs` and its module declaration in
`tests/flow_expansion.rs` were changed; both received CodeScene MCP scores of 10.

Because this added unique coverage, one category execution was justified:

```sh
./verify-core.sh --suite unit --output target/verification/task22-unit4b-independent-root-only
```

This ran outside the sandbox and passed all five stages: **156 distinct actual
passes, zero failures/ignored, 156 selected, zero not run**. Thirteen expansion
cases are included. Prerequisites took 0.894s, preparation 0.945s, compilation
10.740s, tests 11.942s and cleanup 1.152s. Cleanup reclaimed 555,263,299 bytes,
retained zero, had no errors/unmeasured paths; removed paths were confirmed absent.
No full/native/package or compiler run was added.

The original 18-file manifest differed only at the authorized test barrel before
and after this category; all production files remained unchanged. New source
identities after the run:

- `tests/flow_expansion.rs`:
  `87bfd21ce257fff575dcd223bb86b02af3eeb11c12404212b86802f3b71af7a8`
- `tests/flow_expansion/root_only.rs`:
  `ec089d3e07fcd1624afbded24617da04ece0e75e4fbae0fd0e3f8d53184a127e`

The category report and logs retain these actual test results and cleanup. Parent
quality/freeze integration follows the test-only addition. Scope remains one
selected root call or no selection: no recursive/multi-selection expansion,
configurable budgets, query/UI journey or native acceptance is claimed.

#### Unit 5A nested-selection independent audit

The frozen nested-selection source/tests and existing final category were audited
without rerunning the same category. All **23** hashes in
`target/task22-unit5a-nested/freeze-sha256.json` matched on the audit reads;
manifest SHA-256:
`03e5c597c56353e4068de9815f19c4a733ea2816864de8b71db71ef08ce316c8`.

`target/verification/task22-unit5a-final` contains **167 distinct executed names
and passes, zero failures/ignored**, matching 167 selected and zero not-run entries.
The **22 expansion** and **19 result-contract** cases are included in that total.
All five stages passed. Prerequisites took 0.835s, preparation 0.917s, compilation
9.604s, tests 10.904s and cleanup 1.164s. Cleanup reclaimed 562,326,288 bytes,
retained zero, reported no errors/unmeasured paths, and removed paths were
independently confirmed absent. There is no additional tester execution or
isolated timing/performance claim.

Reviewed observable coverage:

- Nested selection yields the exact two-choice occurrence path, three separately
  owned sources, leaf reachability and return to the child's continuation.
  Nested cleanup also retains the semantic finally/return replay identity.
- Reversing both request order and immutable-input order yields byte-identical
  results. Diamond visits to a common callee and repeated sibling invocations
  remain distinct occurrences while sharing source snapshots. Possible target
  certainty remains possible. Direct/mutual cycles stop only on the active path.
- Stopped unknown or missing ancestors produce typed `blocked_selections` tied to
  a real stopped occurrence, with explanatory count, rather than synthetic child
  bodies/sources. Result validation rejects ghost call-bearing steps and rendered
  children under stopped parents.
- A nested stage ending in unresolved break rolls back child/leaf occurrences,
  request statuses and borrowed pending sources; no leaf body survives, only the
  caller source and an explanatory blocked request remain.
- Duplicate/orphan/stale selections, invalid unused input owners, duplicate symbol
  inputs, conflicting physical source bytes, malformed blocked choices, excessive
  count/depth and streamed encoded-identity size reject through production APIs.
- Two selected unproven slots share one original marker in original call-slot
  order with byte-identical ordinary chart output. Stopped conditions preserve
  the original single decision and exact ordinary literal-`false` routing.

These core unit tests use explicit immutable IR/source data and actual `expand`
and `ExpandedFlow::validate`; they do not fabricate successful compiler results
or duplicate eligibility/rollback decisions. The graph observation helper bounds
described above still apply. Source inspection additionally confirms pending
source content is borrowed (`&str`), original indexes are shared with `Rc`, and
owned source text is materialized only for a successful final result. The rollback
checkpoint contains small statuses and vector lengths, not cloned source bodies.
This is an ownership/code-path audit, not an allocation benchmark.

No unique missing boundary behavior was identified within 5A's scope. Runtime
reds are retained and correctly distinguished: invalid unused owner (19 expansion
passes / 1 failure), malformed request below unavailable ancestor (19 / 1), ghost
stopped body (18 contract passes / 1 failure), and shared multislot/condition
markers (20 expansion passes / 2 failures). The final green includes all these
identities. Both final raw CodeScene reports pass: 22 numeric scores are 10 and
the declaration barrel is null; safeguards pass. Earlier quality results remain
preserved. Legacy IR/chart/artifact and the exact 17,644-byte builder golden still
match their earlier hashes.

##### Failed cleanup and later recovery remain separate evidence

The first `task22-unit5a-api-red` failed honestly at the missing-API compilation
step. Its test stage was **not run**, and its cleanup was also **failed** because
native lsof returned inconclusive nonempty PID/file data with exit 1. It retained
**498,850,158 bytes**. This historical run is not relabeled passed by later work.
Original result/log/cleanup/workspace records are preserved under
`target/task22-progress/cleanup-unit5a/original/`.

The separately recorded root inspection in `inspection.json` shows the exact
previously reported PIDs absent (`ps` exit 1 with empty output) and native lsof
with empty stdout/stderr. The later owned `recovery-cleanup.json` reports passed,
reclaimed exactly 498,850,158 bytes and retained zero, with no errors/unmeasured
paths. Its recorded removed build path is absent. The final category's successful
cleanup is a third, distinct observation. No cleanup rule was weakened or old
failed report silently replaced by this audit.

Remaining limits: no configurable expansion budgets or whole-attempt retry,
CLI/query/UI integration, compiler journey, package/native or full verification
is established by this nested core unit checkpoint. Only this audit document was
edited; no source/inventory write or build/test execution was performed.

#### Unit 5B complete-assembly budget audit

Independent source/result inspection, without a redundant category rerun, confirms
all **14** frozen hashes in `target/task22-unit5b-budgets/freeze-sha256.json`.
Manifest SHA-256:
`abdcfd78a85cf1e8f6b6eb482bdc1788a41c2dd807be6bd81f66f502517443af`.

The actual final log under `target/verification/task22-unit5b-final` contains
**174 unique names/passes, zero failures/ignored**, matching 174 selected and zero
not-run registrations. All five stages passed. **29 expansion cases include the
seven budget cases**; these are not counted again. Prerequisites took 0.900s,
preparation 0.958s, compilation 9.190s, tests 11.729s and cleanup 1.089s. Cleanup
reclaimed 563,382,545 bytes, retained zero, had no errors/unmeasured paths, and
removed paths were independently confirmed absent. The producer records no
compilation warnings; this audit does not claim a new compilation run.

The tests assert actual public `expand` results, rather than reenacting budget
accounting:

| Boundary/risk | Observable assertion |
| --- | --- |
| Zero depth or reached-request allowance | Correct typed `Limit` reason, no child source, exact collapsed ordinary chart bytes. |
| Inclusive graph/source admission | Exactly measured complete graph/source allowances reproduce the full result bytes; steps or links independently one below the complete size demote the child and preserve the caller chart. |
| Work after an accepted graft | A later caller suffix or duplicated finally cleanup exceeding the allowance removes the earlier child graft, returns a typed Steps stop and preserves exact collapsed caller bytes/source count. |
| Nonmonotonic graph size | Full always-throwing B removes the suffix; demoting B restores the larger suffix, requiring another demotion of A. Both typed stops and exact collapsed chart are required, and reversed input/request order gives identical complete result bytes. No maximal expanded-subset claim is made. |
| Unique owner and oversized borrowed source | Same-owner root/child charge one source; a child longer than the old 16 MiB hard limit produces SourceBytes stop and blocked descendant with only the root source. The same oversized mandatory root returns an error. |
| Rejected nested work still costs requests | Child and leaf evaluations occur before rejection; later sibling is a Requests stop, with rolled-back leaf source/body and blocked metadata. Rejection does not refund already reached evaluation work. |
| Failure distinctions | Invalid ceilings/positive minima and exhausted mandatory-root allowances return errors. Entry cancellation remains an error containing cancellation context, not an apparently successful limit stop. |

Source audit confirms private `BudgetExceeded` is the only error type accepted by
retry downcasting. Malformed data and cancellation propagate normally. One
all-stopped assembly proves mandatory root fit; at most selection-count-plus-one
subsequent speculative attempts can add fresh demotions. The demotion set grows
monotonically; graph size is explicitly allowed to grow after demotion. A failure
without a new eligible candidate propagates instead of retrying forever. The
same assembler handles root proof and later attempts, avoiding a copied size
estimator or a second completion algorithm.

Each attempt resets its reached-work counter; rollback deliberately does not
restore that counter inside an attempt. Blocked descendants remain request
metadata and do not activate their missing/unbuilt bodies. `Prepared` shares
immutable indexes through synchronous `Rc` and scoped `RefCell` borrows; it does
not retain a mutable borrow across recursive assembly. SourcePool admission
checks the prospective unique byte count before adding a borrowed `&str` entry.
Failed attempts drop borrowed pools and private graphs. Only complete successful
materialization constructs owned source text.

The source-limit test proves absent leaked source entries and preserved caller
output. The stronger no-copy conclusion is limited to the inspected ownership
path, not an allocation/RSS measurement; the test's large input string itself is
allocated as explicit input. No synthetic successful compiler or mock business
service is involved. No unique missing required-scope case was identified.

The first API red is honestly a missing `ExpansionLimits`/`with_limits` compile
failure, not a manufactured runtime failure: the test stage was not run and owned
cleanup passed, reclaiming 506,450,506 bytes with zero retained. Both current raw
CodeScene reports pass, including safeguards: 13 numeric scores are 10 and the
declaration barrel is null. The exact 17,644-byte ordinary builder golden remains
unchanged. Historical 5A cleanup failure/recovery described above is unaffected.

Only these evidence notes were changed. No source/inventory edits, builds/tests,
compiler/CLI consumer, native GUI, package or full verification were performed by
this audit. The currently locked native session remains a separate prerequisite;
these core results do not establish native acceptance.

#### Final negative eligibility matrix and focused SDK checkpoint

The parent identified one acceptance-criterion gap in the existing negative
matrix: explicit HTTP/navigation relations and deferred activation/callee facts
were not supplied, although production guards were already present. The same
test identity now supplies those four variants. Each requires an Ineligible stop,
no child body/source, exact full selected choice and preserved target metadata on
the original marker. Deferred activation uses valid partial facts
`{"timing":"deferred"}`, without a false isolation proof. No production change
or artificial runtime-red claim was made.

The first `task22-final-negative-contracts` core category passed 174 cases and
cleanup. Modern CodeScene scored the file 10, but the old CLI's global review
still found the enlarged assertion function too long. That failed verdict remains
preserved. With parent authorization, a cohesive `assert_ineligible_choice`
expectation helper extracted only assertions, without reproducing eligibility
decisions. Both old and modern scoped raw reviews now score 10, stored at
`target/task22-progress/negative-contracts-quality-{old,modern}.json`.

The source change justified the single subsequent core category
`target/verification/task22-final-negative-contracts-refactored`: **174 passed,
zero failed/ignored**. All five stages passed; prerequisites 0.900s, preparation
0.949s, compilation 10.336s, tests 11.670s, cleanup 1.107s. Owned cleanup reclaimed
563,500,018 bytes, retained zero. Final test source SHA-256 is
`c44183ba87175ee448e87c29acf79bcd03a32a9f69fe5fb67ed11cda34e134a7`.
The test identity did not change; scoped rustfmt/diff checks passed.

The first focused SDK attempt is retained at
`target/verification/task22-unit6-focused`. It used production inventory selection,
Runner/Step/exclusive locking, SuiteContext release environments, prerequisite
checks, exact Rust registration/execution and owned category cleanup. It selected
only one Java integration identity and two C#/TypeScript acceptance identities,
with all other registrations/gates explicitly not run. It is not whole-category
verification and never used fabricated compiler results.

- Java passed. Its setup/compilation/test/cleanup stages took
  0.091s/22.255s/1.993s/1.108s, plus prerequisites 0.151s. Cleanup reclaimed
  175,370,532 bytes and retained zero.
- The shared acceptance build compiled both targets once (24.688s), but the
  TypeScript test failed before expansion: the helper selected by CallSite label
  `alias()` instead of actual callee-expression label `alias`, finding zero sites.
  The production runner stopped at that artifact failure, so C# was **not run**.
- Acceptance cleanup passed and removed its owned build/temp paths. Its reported
  178,727,250 retained bytes are the two existing previous-app rollback packages
  deliberately preserved by cleanup policy, not leaked category workspaces.
- Source hashes were unchanged across the failed attempt. Full logs, exact
  selection/source snapshots, timings and `independent-observation.json` remain
  preserved; the failed attempt is not relabeled passed.

Read-only inspection of retained actual compiler output resolves the fixture
expectation: exact call span 104..111 is `alias()`, its label is `alias`, and its
full target is the implemented `leaf.ts|74|function`, with signature
`(value?: number): number`, `body_unavailable=false`, and one matching owned
synchronous FunctionFlowIR (span 74..118). The current fixture source hash equals
the saved source hash. This demonstrates an expectation/selector error rather
than an observed overload-to-body binding defect. Full observation is in
`target/task22-progress/typescript-callsite104-target-observation.json`.

The producer owns the authorized shared-helper correction: select an exact
compiler-owned span whose **cached** source slice equals known fixture call text,
then independently assert the real CallSite label and precise callee metadata.
Uniqueness/facts/source/connection assertions are retained. Because that shared
helper and Java Pair changed, the parent requires all three exact workflows on
one refreshed four-file freeze at a fresh `task22-unit6-focused-r2` output. That
repeat is justified by changed shared test source; it is not a parser/counting
rerun or a whole engine SDK suite. Its result is pending at this audit checkpoint.

##### Final focused SDK retry

`target/verification/task22-unit6-focused-r2` passed **all three exact real SDK
identities** and all ten stages. Java integration passed one test; the separately
owned acceptance workspace passed TypeScript and C# tests built together once.
No ignored/skipped case was silently counted as executed. The inventories record
247 other integration test registrations/one gate and 80 other acceptance
registrations/five gates as not run. This remains focused-subset evidence.

Prerequisites/setup/compilation/test/cleanup respectively took
0.262/0.083/24.418/1.973/1.047s for integration and
0.188/0.077/24.861/14.450/1.212s for acceptance. Both cleanups passed and removed
their owned build/temp paths. Integration reclaimed 175,373,894 bytes and retained
zero. Acceptance reclaimed 234,048,959 bytes; its 178,727,250 retained bytes are
the two existing rollback app packages, deliberately preserved by policy.

All 45 reviewed source hashes and the exact inventory hash matched before/after
the retry. The four compiler-test source manifest has SHA-256
`609da4e53ec9a11d5dbb6dbaef29f0e15d5e61ee3cd145e3c4e890a8fa559271`.
Both final raw CodeScene reports pass four scores of 10 and their safeguards.
The completed archive is `evidence/flowcharts/expansion/unit6/`, with exact
selection, source checkpoints, commands, timing/cleanup records, both expectation
corrections, original failure and final negative-core 174 evidence. All 84
reports/logs in its tar archive were independently hash-verified after writing;
no retained binaries or packages were archived.

These are the sources at the focused SDK checkpoint. Root subsequently reported
the mandatory full run stopped on a core `calls.rs` collapsible-if Clippy finding;
cargo-check passed and later runtime stages were not run. Root owns that small
correction and full retry. The focused archive does not assert later entire-task
source immutability or a full/native/package pass.


<a id="source-evidence-flowcharts-expansion-unit1-readme-md"></a>

## evidence/flowcharts/expansion/unit1/README.md

Source: [evidence/flowcharts/expansion/unit1/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit1/README.md). Historical record; retained rationale, methods and limitations.

### Invocation-fact contract checkpoint

Tests first observed four failures: new facts were discarded, invalid enums were
ignored, and inconsistent isolation/direct-dispatch claims were accepted. Typed
facts now default to uncertainty and omit unknown/default values during serialization.
The shared owner-aware walk validates positive claims at publication, source, checked
chart and direct builder boundaries. Dispatch survives target filtering without promotion.

Independent review approved all 17 frozen hashes and the unchanged staged baseline.
Independent tests passed 209 distinct cases (151 core, 18 engine comparison, 40 desktop
consumers), with no failures or ignored cases. The 16 new fact cases are counted only
within the core group. Exact legacy IR/chart/canonical-artifact bytes remain unchanged.
Workspace compilation and core Clippy pass; existing Rune warnings remain.

Both raw CodeScene reports pass: 13 scored files at 10, one declaration-only barrel,
and passing safeguards. `verification.json` discloses that the initial sandbox EPERM
log was overwritten by the definitive outside-sandbox run; no replacement raw failure
log was invented. Other original outputs are preserved in `raw-evidence.tar.gz`, with
hashes in `archive-members.json`; JSON copies are unchanged.

This unit does not populate exporters or implement expanded charts. Full verification
belongs to the completed task; CLI/desktop expansion delivery remains TASK-22.8.


<a id="source-evidence-flowcharts-expansion-unit2a-readme-md"></a>

## evidence/flowcharts/expansion/unit2a/README.md

Source: [evidence/flowcharts/expansion/unit2a/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2a/README.md). Historical record; retained rationale, methods and limitations.

### C# execution facts checkpoint

Unit2A passed independent source review and 176 distinct tests: 86 native Flow,
25 engine, 4 explicitly executed ignored engine, 5 release C# integration,
16 shared facts/compatibility, and 40 Python dependency/bootstrap/package contracts.
Two existing Java-resource checks were skipped; full/package/native GUI gates
were not run for this checkpoint. See [independent.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2a/independent.json).

Fresh old/current analyzer output preserved 12 native nodes, 5 edges and all
11 flow bodies apart from additive execution/invocation metadata. The protected
reference DLL, all 22 Unit2A files, 17 Unit1 files and staged baseline stayed unchanged.
The combined patch exactly matches the upstream checkout, with an exact prior
checkpoint and pinned checksums. Historical canonical flow inventory stays version 1;
fresh C# resource capability is version 2.

Both raw CodeScene review verdicts remain failed for existing findings:
FlowExpressions 9.38 and FlowExport 9.68, identical before and after. Both safeguards
passed; source review found no correctness or maintainability blocker.

The archive preserves original reports/logs/TRX byte for byte. The index records
their sizes and SHA-256 hashes. Test-first failures and intermediate assertion
corrections are disclosed in [verification.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2a/verification.json).


<a id="source-evidence-flowcharts-expansion-unit2b-readme-md"></a>

## evidence/flowcharts/expansion/unit2b/README.md

Source: [evidence/flowcharts/expansion/unit2b/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2b/README.md). Historical record; retained rationale, methods and limitations.

### Java execution facts checkpoint

Unit2B passed independent review and 84 distinct tests: 40 Python, 27 Java engine,
1 release real-compiler normalization/projection and 16 shared compatibility.
Python had no skips. Twelve broader Rust Java cases remain ignored in the normal
unit group; the selected real compiler case ran explicitly. No full/native GUI
or package command was run. See [independent.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2b/independent.json).

The v9 source helper adds compiler-owned execution/invocation facts. Typed compiler
errors suppress only new facts in affected files; unowned errors suppress all new
facts. Ambiguous javac recovery and valid-neighbor regressions exercise this.
Historical v8 source data and canonical inventory version 1 remain readable; fresh
analysis requires v9 and packaging checks both new helper classes.

Native declarations, call IDs, line/ownership tables and mapped graph/source
projection match the immutable v8 helper. Raw source records differ only in the
header and optional facts. Earlier manifests, staged baseline and helper JAR
hashes stayed unchanged. Both CodeScene versions passed: 18 files scored 10 and
safeguards checked 126 eligible files of 140 total without regressions.

The archive retains original reports/logs, raw source rows and the five compared
native tables from each helper byte for byte. Its index records sizes and hashes.
Compiler binaries and unrelated generated analysis files are not duplicated.


<a id="source-evidence-flowcharts-expansion-unit2c-readme-md"></a>

## evidence/flowcharts/expansion/unit2c/README.md

Source: [evidence/flowcharts/expansion/unit2c/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2c/README.md). Historical record; retained rationale, methods and limitations.

### TypeScript execution facts checkpoint

Unit2C passed independent review and 140 distinct tests: 120 JavaScript adapter,
2 real preparation gates in release mode, one new real CLI facts/pruning/source
case, one existing CLI query and 16 shared compatibility cases. Definitive groups
had no skips. Clippy was clean. No full/native GUI/package command was run.
See [independent.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2c/independent.json).

The version-7 helper proves bounded ordinary calls from compiler bindings and
explicit evaluation order, retaining conservative unknown facts for unsupported
forms and reassigned bindings. Generator defaults are an immediate prefix before
the deferred body, so their combined IR remains unknown. Historical canonical
inventory stays version 1; fresh preparation rejects old/missing/future helpers.

The final immutable-v6/relocated-v7 comparison preserves 16 native nodes, one edge
and 11 function IR bodies apart from execution/invocation and adapter version.
All 72 copied files, including 70 JavaScript files, match frozen source. Earlier
checks covered 61 production JavaScript files only; a stale test-copy checkpoint
and its mismatched file remain preserved. Final full-copy equality uses
[projection-relocation-frozen.json](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit2c/projection-relocation-frozen.json).

Both final CodeScene reports passed: 14 files scored 10, safeguards checked
140 eligible files of 156 total. Original test-walker findings remain in the
archive; the focused 22-case rerun and independent full suite verify its refactor.
The independent report also records initial ignored/default and debug-targeted
preparation attempts; only definitive release results count for those gates.

All current and prior manifests, staged baseline and immutable helper hashes
stayed unchanged. The archive retains original reports/logs/maps/probes and the
stale test file byte for byte; its index records sizes and SHA-256 hashes.


<a id="source-evidence-flowcharts-expansion-unit3-readme-md"></a>

## evidence/flowcharts/expansion/unit3/README.md

Source: [evidence/flowcharts/expansion/unit3/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit3/README.md). Historical record; retained rationale, methods and limitations.

### Expanded-result contract checkpoint

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


<a id="source-evidence-flowcharts-expansion-unit4a-readme-md"></a>

## evidence/flowcharts/expansion/unit4a/README.md

Source: [evidence/flowcharts/expansion/unit4a/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit4a/README.md). Historical record; retained rationale, methods and limitations.

### Completion-aware builder refactor checkpoint

Eight explicit in-memory IR cases pin complete ordinary chart serialization
before refactoring. The deliberate empty baseline capture mismatch records
17,644 bytes from the pre-refactor production builder; it is not evidence of a
production defect. The pinned baseline passed before production changes.

The refactor extracts typed completion and function/region assembly, encapsulates
existing call exception tracking, and returns one prune mapping. The ordinary
chart bytes, completion grouping order, deferred exclusions and exception paths
remain unchanged. No expansion request or callee composition is introduced.

Final categorized core verification passed 143 tests and owned cleanup. Both
CodeScene reports and safeguards passed; eight Rust files scored 10. Independent
review approved the implementation. All ten Unit 3 source hashes remain exact.
Reports retain compilation, preparation, execution and cleanup timings separately.

These are core unit checks. Full verification belongs to the completed task and
has not run for this checkpoint.


<a id="source-evidence-flowcharts-expansion-unit4b-readme-md"></a>

## evidence/flowcharts/expansion/unit4b/README.md

Source: [evidence/flowcharts/expansion/unit4b/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit4b/README.md). Historical record; retained rationale, methods and limitations.

### One selected synchronous callee checkpoint

Core accepts borrowed immutable function/source pairs and one explicit root-site
choice. Compiler invocation/body facts gate expansion; an explicit candidate
choice retains its full uncertainty and provenance. Unsupported, deferred,
reference-only or missing bodies stay visible stops. Valid legacy and implicit
cleanup Actions are ineligible rather than falsely stale.

A separately staged child preserves its own exception frame and source origins.
Normal/Return resumes the caller; escaping Throw traverses child cleanup before
caller protection. Pre-body invocation failure stays separate. Conditions resume
the caller's decision after successful child completion. Original-reference
indexing and typed cleanup routes distinguish equal spans and cleanup copies.

The final independent categorized core run passed 156 distinct tests, including
13 expansion cases, all five stages and owned cleanup. It added a unique public
root-only expansion check after producer verification. Ordinary and legacy chart
bytes remain exact. Final production reviews passed; both new root-only test
files score 10 and the independent MCP safeguard passed all 77 eligible files.

Missing-API compile red, cleanup-selection runtime red, both review regressions
and intermediate compilation failures remain indexed/archived. Those are not
passing runs. Full task verification has not run. Nested selections, global
configurable budgets, CLI and desktop controls remain later checkpoints.


<a id="source-evidence-flowcharts-expansion-unit5a-readme-md"></a>

## evidence/flowcharts/expansion/unit5a/README.md

Source: [evidence/flowcharts/expansion/unit5a/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit5a/README.md). Historical record; retained rationale, methods and limitations.

### Explicit nested expansion checkpoint

Nested selections retain full caller occurrence paths, original call-site and
cleanup-route identities, and exact candidate records. Active-path cycles stop;
repeated siblings and diamonds remain independent. Builder order determines
occurrence/source IDs regardless of input or request ordering.

Blocked descendants are explicit unevaluated request metadata linked to a real
stopped ancestor. They do not invent body steps, sources or compiler facts.
Rejected staging restores small request/occurrence metadata and borrowed-source
pool length; source text is materialized only after complete successful assembly.
Prepared original-reference indexes are reused through local `Rc` ownership.

Every selected legacy slot is consumed, even when several share one Value.
Ineligible slots share the original call-bearing marker without losing separate
choice identities. Stopped conditions preserve the original decision and literal
branches. All-stopped ordinary chart bytes remain exact.

Final categorized core verification passed 167 distinct tests: 22 expansion and
19 result-contract cases are included. Independent source/results review verified
all 23 frozen hashes, complete selection/execution and owned cleanup. Both final
CodeScene reports/safeguards passed; the earlier old-CLI complexity failure is
retained with the cohesive validation refactor that resolved it.

The initial API-red cleanup failed closed and retained 498,850,158 bytes. Separate
native inspection later proved the reported PIDs absent and no open files; the
unchanged owned cleanup tool then reclaimed that exact workspace. Original
failure and recovery are both archived, never relabeled as one passing run.

Configurable budgets and bounded whole-attempt retries remain Unit 5B. Full
verification, final compiler boundaries, CLI and UI delivery have not run here.


<a id="source-evidence-flowcharts-expansion-unit5b-readme-md"></a>

## evidence/flowcharts/expansion/unit5b/README.md

Source: [evidence/flowcharts/expansion/unit5b/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit5b/README.md). Historical record; retained rationale, methods and limitations.

### Global expansion work budgets

Validated defaults are depth 8, 64 reached requests, 1,500 assembly steps, 6,000
links and 16 MiB of unique-owner source bytes. Hard depth/request ceilings are
32/128. Zero depth or request allowance produces typed stops; graph/source
allowances must be positive. The request identity envelope remains independent.

The same real assembler first proves that the complete all-stopped caller fits.
Private attempts then either complete or demote an active/latest-root-graft
selection and retry. The demotion set grows monotonically; graph size need not.
This policy is deterministic and does not promise a maximum expanded subset.
Later caller work and duplicated cleanup cannot leave a silently truncated chart.

Source admission checks before mutation, deduplicates exact owners, and holds
borrowed content until complete success. Rejected nested work does not refund
reached evaluations within an attempt. Cancellation remains an error and cannot
become a limit marker or a partial published result.

Final categorized core verification passed 174 distinct tests, including 29
expansion cases and seven budget cases. All stages and owned cleanup passed.
Independent review/audit confirmed the 14 frozen hashes, exact ordinary goldens,
meaningful complete-assembly assertions and both passing CodeScene reports and
safeguards. Oversized-child tests prove bounded result/source outcomes; there is
no allocation benchmark or native acceptance claim here.

Real compiler consumer checks and full verification remain before task completion.


<a id="source-evidence-flowcharts-expansion-unit6-readme-md"></a>

## evidence/flowcharts/expansion/unit6/README.md

Source: [evidence/flowcharts/expansion/unit6/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/expansion/unit6/README.md). Historical record; retained rationale, methods and limitations.

### Compiler-backed cached expansion consumers

The final focused checkpoint passed **three distinct real SDK workflows**: Java
compiler integration, C# CLI acceptance and TypeScript CLI acceptance. All ten
prerequisite/setup/compile/test/cleanup stages passed. This is an exact inventory
subset, **not a whole engine category or full-verification pass**. Other tests and
gates are explicitly recorded as not run in the archived selections.

Each workflow consumes actual compiler output and the saved artifact's Ready
caller/callee views, verifies synchronous/direct/isolation facts, expands through
the production core API and checks connected call/return boundaries, source
ownership, exact bytes/digests and unchanged artifact bytes. C#/TypeScript also
retain their real public CLI cached-query journeys. No new public expanded-flow
CLI operation or native-window behavior is claimed.

Both owned categories used release workspaces and the production verification
selection/environment/prerequisite/execution/cleanup APIs. The acceptance targets
compiled together once. Integration compilation/test/cleanup took
24.418s/1.973s/1.047s; acceptance took 24.861s/14.450s/1.212s. Setup and prerequisite
times are separate in `verification.json`. All 45 reviewed source snapshot hashes,
including the four-file compiler-test freeze, matched before/after this run.

Owned builds and temporary workspaces were removed. Integration reclaimed
175,373,894 bytes and acceptance reclaimed 234,048,959 bytes. Acceptance's reported
178,727,250 retained bytes are the two previous-app rollback packages deliberately
preserved by cleanup policy, not leaked category outputs. No packages or binaries
are included here.

The first focused run remains failed historical evidence: Java passed, TypeScript
failed selecting a CallSite by the wrong label, and C# was not run after that
artifact failure. Both cleanups still passed. The corrected helper selects exact
**cached** source text at the compiler-owned span, then asserts the separate real
label. The precise TypeScript implementation signature was corrected from `leaf`
to the observed `(value?: number): number`; the latter diagnosis used retained
actual compiler data before reaching that assertion at runtime. The target was
an available implemented body, not an observed overload-binding defect. All three
workflows were rerun because their shared test helper changed.

The final negative core matrix kept the same test identity and now covers HTTP,
navigation, valid deferred activation and Deferred callee bodies with visible
ineligible stops, no child source/body and preserved full choices. Its refactored
category passed 174 distinct tests with owned cleanup. Old/modern scoped reviews
both score 10 after assertion-helper extraction. Earlier passing runtime and
failing old-CodeScene results remain preserved rather than relabeled.

`verification.json` is the final independent focused report;
`producer-verification.json` preserves the earlier producer freeze awaiting those
checks. Current and prior raw quality reports retain their original verdicts.
`category-report-index.json` hashes all 84 entries in `category-reports.tar.gz`;
every archived entry was read back and verified. The archive contains reports and
logs only, including first failure, final retry, both core checkpoints and the
actual TypeScript target observation.

This is a **prior source checkpoint**. A subsequent mandatory full run stopped at
a core `calls.rs` Clippy finding after cargo-check passed; later runtime stages
were not run in that attempt. Root owns the small correction and full retry.
These focused results do not claim that later whole-task sources are unchanged
or that full/native/package verification has passed.


<a id="source-evidence-flowcharts-revisions-readme-md"></a>

## evidence/flowcharts/revisions/README.md

Source: [evidence/flowcharts/revisions/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/flowcharts/revisions/README.md). Historical record; retained rationale, methods and limitations.

### Bound saved-revision flow queries

TASK-22.5 adds the optional version-1 public CLI `flow_comparison` operation.
The [wire contract](<../../../docs/cli-queries.md#saved-revision-function-flows>)
describes catalog paging, selected before/after details and explicit manual
counterparts. These checks exercise real Git repositories and CLI subprocesses;
desktop comparison presentation belongs to TASK-22.6.

The unit reports preserve focused evidence from an isolated checkout:

- `first-unit-verification`: portable artifact flow ownership, cached source,
  ordinary Flow parity and Code Health.
- `second-unit-verification`: strict revision inspection, unknown schema,
  exact-byte provenance and unchanged legacy Compare bindings.
- `third-unit-verification`: tests-first contract/CLI failures followed by 163
  passing focused tests, all-target Clippy and both installed CodeScene versions.
  One existing SDK-dependent ordinary-query test was ignored in this focused run.

Catalogs use exact symbol identity. Incompatible endpoint membership is unknown;
it never establishes a removal or addition. Ready details use endpoint-local
source/targets. Global strict artifact validation remains in place; selected IR
conversion is lazy only after that validation. The implementation avoids
hydrating two graphs or constructing two GraphIndexes.

WORKTREE freshness retains existing read-only live-input observation. Tests
modify and delete a live file, observe stale freshness, and assert identical
cached-source detail results. Committed endpoints require no linked checkout.
Tests also verify moved refs, changed saved bytes, cancellation, no fetch or
checkout, unchanged canonical/cache files, complete-envelope byte limits and
legacy query/session compatibility.

The independent final audit adds five CLI cases: 168 focused tests passed, with
the same existing SDK-dependent test ignored only in the focused run. Its
`third-unit-verification/independent.json` distinguishes fixture corrections from
production defects.

Integrated root full verification passed all **62/62 stages** in
`target/verification/full-k7cvs5hs`: workspace compilation/Clippy, ordinary suites,
TypeScript tests, real compiler fixtures, all ignored acceptance tests in release
mode, isolated profiling/benchmarks, package/signature checks, installed CLI/MCP
workflows, CodeScene and cleanup. See `verification.json` and
`full-verification/results.json`; raw logs remain in the named local run folder.
No new native UI behavior is claimed for this CLI/core task. TASK-22.4's separate
native C# workflow evidence remains applicable to the unchanged desktop.
