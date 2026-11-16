---
id: doc-4
title: Analyzer behavior and regression acceptance
type: other
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:10:00Z'
---
# Analyzer behavior and regression acceptance


Historical internal records consolidated on 2026-11-16. Source revision: `13b0de13f3ddf904145f93119816c33b8a5b3e55`. Results, versions, counts and limitations below are dated observations, not current guarantees. Reproduction commands may require the recorded toolchain. Current execution policy is in the internal testing strategy; public behavior is documented in docs/. Raw captures remain local and in source history; they are not new tracked inputs.


<a id="source-evidence-analyzers-acceptance-md"></a>

## evidence/analyzers/ACCEPTANCE.md

Source: [evidence/analyzers/ACCEPTANCE.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/ACCEPTANCE.md). Historical record; retained rationale, methods and limitations.

### Analyzer acceptance evidence

#### C#: CallAtlas

Revision: `916c2419c0cdb91b5b86b84df4842dadb17bb6b4`.
Release build passed under .NET SDK 10.0.403. The pinned assembly-identity
serialization patch in `patches/` changes no call resolution. A four-assembly
collision fixture retains 26 declarations and 20 relationships without false
cross-project calls; duplicate AssemblyName projects are explicitly rejected.

The two-project .NET 8 fixture reports 12 nodes and 12 relationships with zero
compiler/body diagnostics and unresolved calls after restoring reference packs.
`tools/check_callatlas.py` verifies overloads, cross-project direct calls,
construction, generics, recursion, disconnected declarations, method-group
references and delegate/dispatch uncertainty.

PacificFate solution analysis reports 20,494 nodes and 75,296 raw relationships (75,881 normalized
relationships including reported possible targets),
including 7,355 linked Core declarations and 130 direct application-to-Core
calls. There are zero reported compilation/body errors and unresolved calls.
`MainGameController::_Ready()` remains visible.

Manually checked against source: `CampaignPersistenceController.CreateDocument`
at `scripts/main/ui/CampaignPersistenceController.cs:93` calls
`CampaignCheckpointCodec.Encode` at
`core/campaign/persistence/CampaignCheckpointCodec.cs:9`.

Limitations: cross-assembly interface dispatch retains the compiler-bound static
target but does not report implementation candidates, including in the fixture
with two implementations. Most source-generator output is excluded by the
upstream analyzer. Framework entry invocations, arbitrary reflection and DI
registrations are not reconstructed.

Machine-readable results: `callatlas-summary.json` (tracked). Large raw graph
files are ignored and can be regenerated with the commands in the README.

#### Java: Eclipse JDT LS

Pinned distribution: 1.63.0, SHA-256 recorded in `analyzers.lock.json`.
Java fixture: 15 declarations, 10 authored methods, 11 relationships, including
overload bindings, recursion, disconnected methods and a method reference.

StoryEngine uses Gradle 9.4, Java 25, Spring WebFlux, Lombok and MapStruct, with
generated OpenAPI source roots. Native `./gradlew compileJava compileTestJava`
passed without running tests or launching the application.

The initial reused JDT workspace with Lombok 1.18.44 failed its workspace build
with 2,356 diagnostics. A fresh workspace with Lombok 1.18.48 reduced that to four
diagnostics, but still failed on Reactor generic inference in
`LlmModelDbService`. Both results are explicitly incomplete. Neither may be
published as a complete application graph. The second collection was interrupted
once the failed build was confirmed; its partial graph is not an acceptance pass.

JDT also omits standalone initializer blocks from document symbols, and its
outgoing call protocol does not classify relationship certainty or distinguish
callback references. `tools/check_jdt.py` checks these results without relaxing
the failed real-project gate.

#### Accepted Java replacement: java-callgraph2

JDT failed the mandatory real-project gate. Before UI work proceeded, existing
java-callgraph2 4.1.0 was validated with BCEL 6.13.0 for Java 25. The adapter exports
Gradle build metadata, compiles every configured Java source set and normalizes
engine output. It does not resolve calls or infer Spring/DI connections itself.

Java 21 and Java 25 fixtures each retain 13 compiled declarations and 21 raw
relationships (14 project-owned relationships). Checks cover overloads,
construction, generics, two interface implementations, recursion, disconnected
methods and method references. Frame limits are zero. StoryEngine supplies 1,403
classes and 13,987 compiled method declarations, with exact BCEL metadata equality,
and 65,014 raw relationships before external exclusion. Normalization retains
14,504 symbols (including 517 explicit body-unavailable inherited/reference
placeholders) and 22,326 project-owned relationships. Generated OpenAPI, Lombok and
MapStruct output are included from actual compilation roots.

Manually checked source chains in AdvancedExtractionJobFlow include `execute`
to `executeSharedStep` (engine VIR) and `execute` to the `logCompletion` method
reference (engine _LM). The latter is a reported reference, not proof of callback
execution. `bytecode-summary.json` records exact signatures and engine call IDs.
The real-compiler regression also checks an authored `start -> lambda -> leaf`
path survives the default production filter.

Limitations: engine-reported inherited endpoints can lack declared bodies; these
remain labeled placeholders, never name-resolved edges. Spring/DI, reflection and
framework callback ordering are not inferred. JVM static receiver targets and
possible implementations preserve separate engine kinds. Source columns are
unavailable. See `native-pipeline.md` for ownership, freshness and cancellation
checks. The combined application pipeline successfully published both real builds
as 34,998 symbols and 98,207 relationships in one SQLite generation.


<a id="source-evidence-analyzers-java-duplicates-md"></a>

## evidence/analyzers/java-duplicates.md

Source: [evidence/analyzers/java-duplicates.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/java-duplicates.md). Historical record; retained rationale, methods and limitations.

### Java duplicate-class analysis

TASK-12 adds source-set compile contexts for Maven and Gradle. Evaluated own
class outputs precede the evaluated compile classpath; each context selects the
first definition of each JVM class name. A Java helper inventories directory
and archive inputs, handles Java 25 multi-release archives, and stages unique
winning definitions with original provenance and byte-comparison diagnostics.

Declarations retain build/module/source-set identity, including identical
copies in separate modules. Only a source set's own method bodies contribute
outgoing edges. Visible project dependencies provide source-backed declarations
and target lookup. External archives participate only in class-name visibility
and shadow detection: their bytecode is not passed to the call-graph engine and
their declarations and relationships never enter the graph. Pure project
archives map back to their source set; mixed archives are not assigned a
whole-source-set alias.

The combined graph uses each caller's owning compile context. Paths may cross
incompatible consumer contexts and are not a model of one runtime classpath.

#### Source-focused engine scope

The user clarified that relations between actual project source files are the
priority, rather than whole-dependency analysis. The proposed reduced-precision
retry was removed. Engine failures and frame-snapshot limits still fail analysis;
there is no precision downgrade or retry policy.

Verbose upstream expression-exclusion logging is disabled because it repeats
the project class-name filter for each excluded method. Structured CodeGraph
visibility diagnostics remain enabled. Successful context scratch directories
are deleted immediately to bound disk consumption; a failing production
context retains its files for diagnosis.

#### Reproduction

With bootstrapped artifacts, JDK 25, Maven and Gradle available:

```sh
cargo test --all-targets -- --test-threads=1
cargo test --lib analyzers::java::tests -- --ignored --test-threads=1
cargo test --test java_builds -- --ignored --test-threads=1
cargo test --test java_duplicates duplicate_classes -- --ignored --test-threads=1
cargo test --test packaged_java -- --ignored --test-threads=1
python3.13 -m unittest discover -s tests -v
cargo clippy --all-targets --no-deps -- -D warnings
python3.13 tools/package.py
```

The duplicate fixture checks independent differing definitions, identical copies,
main/test/custom-source-set shadowing, reversed dependency precedence, external
winners, mixed unconsumed Gradle archives, deterministic refresh, and SQLite
round trips. Shared fixture setup is extracted into tests/support/java.rs.
Real-JVM Python regressions cover provenance escaping, Java 25 multi-release
selection versus ordinary directories, standalone adapter inheritance behavior, filtered staging that excludes external
winners, retained equality methods, and valid methodless contexts. Module-info-only
source sets are compiled successfully and skipped for graph analysis.
An additional snapshot test verifies that a previously absent compile directory
becoming populated invalidates the graph's fingerprints.

#### Validation record

Verified on Apple Silicon macOS with JDK 25 on 2026-11-09:

- Rust default suite: 99 passed; compiler-backed callback/entry checks: 2 passed.
- Existing real Java builds: 8 passed; duplicate Maven/Gradle builds: 2 passed.
- Packaged-resource checks: 2 passed; Python suite: 32 passed.
- Clippy with warnings denied passed; modified Java/Rust/Python sources scored
  CodeScene 10.0; safeguard passed for 37 eligible files.
- Signed local app/ZIP packaging passed with deep/strict signature verification.
- The actual recurring-billing Maven build succeeded. Replaying its successful
  compiled metadata through the final source-focused pipeline completed all 13
  contexts in 126 seconds: 20,581 declarations and 50,014 relationships. Every
  declaration has a source location within the selected project; there are no
  dependency-source locations or precision downgrades. Both impl and test-utils
  retain their 10 generated message/notice classes (170 declarations each).
- The verification build's two tracked frontend generation changes were inspected
  and restored; recurring-billing and the vendored Rune tree remain unchanged.

The initial full-dependency experiment exhausted disk through repetitive upstream
exclusion logs. Source-only staging supersedes that experiment; no dependency
bodies are analyzed. Completed formatting comparison copies and an inactive,
rebuildable Rust incremental cache were reclaimed during verification.

SonarMCP is inactive because this project has no .sonar/config.json. CodeScene cannot score the Gradle init-script extension;
Java and Rust source reviews cover the implementation on either side of it.


<a id="source-evidence-analyzers-java-lambda-correlation-md"></a>

## evidence/analyzers/java-lambda-correlation.md

Source: [evidence/analyzers/java-lambda-correlation.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/java-lambda-correlation.md). Historical record; retained rationale, methods and limitations.

### TASK-32: Verified Java lambda identities

Implemented on `codex/standalone-graph-engine`, 2026-11-10. No upstream
dependency changes, merge or push are needed.

#### Behavior and proof

- The `kind_v8` inventory snapshots source lambda facts before lowering. A
  processor-free, memory-only javac replay captures source offsets and exact
  LambdaMetafactory implementation handles.
- A shared BCEL executable-class model checks class version/ownership/hierarchy,
  field and method descriptors/flags, generic signatures, constants, symbolic
  instructions, exception handlers and expanded bootstrap arguments. It rejects
  duplicate members. Debug attributes and constant-pool positions are excluded
  from this proof. Both debug modes must agree when both match the actual output.
- Existing singleton source keys are unchanged. Complete shared implementations
  receive one tagged, sorted member collection; membership/body changes can
  change that identity. Indistinguishable separate peers retain deterministic
  occurrences without individual continuity claims after identical swaps/inserts.
- Identity assignment and source-flow binding are separate. Shared implementations
  never acquire an arbitrary member flow. Native nodes and full native edge
  records are unchanged by binding, including absent SourceFile/line metadata.
- Verified and rejected owners cannot use legacy line fallback. Unavailable
  compiler replay retains conservative legacy attribution and specific reasons.
  Local/anonymous owners retain compiler-native portable-identity warnings.
- The inventory version participates in cache keys and analyzer provenance.
  Runtime helpers require v8; packaging rejects incomplete/legacy adapter jars.

#### Verification

| Check | Result |
| --- | --- |
| Regression-first real JVM and full analyzer checks | Old v7 lacked correlation records; same-line flow count was 1 instead of 3 |
| Workspace tests (`cargo test --workspace --locked`) | All unignored tests passed |
| Strict workspace all-target Clippy | Passed |
| Focused Java flows, lambda identities and correlation | 12 passed |
| Maven/Gradle, duplicate classes, HTTP, Lombok, Spring, packaged resources | 20 public acceptance tests passed |
| Public real-JVM Java engine-library checks | 9 passed |
| Python suite | 72 passed |
| Final adapter `installDist` | Passed; compiler-internal exports included in helper launchers |
| Engine-only packaging | Passed; installed and packaged Java adapter jars match |
| Final packaged engine process | Passed: one merged native lambda, one grouped identity, no arbitrary source flow |
| Independent reviewer | No actionable findings |
| CodeScene file reviews | All 22 supported modified/new source files scored 10.0 |
| CodeScene pre-commit safeguard | Passed: 22 of 22 eligible source files, no regressions |

The real-javac controls cover `-g`, `-g:none`, `-g:vars`, target release 17,
same-line and nested lambdas, captures, serializable lambdas, field/static/
constructor initializers, method references, merged nested/sibling implementations,
source/output immutability, stale source, and tampered bootstrap and method
descriptors. Rust boundary checks reject incomplete/duplicate/conflicting groups,
cross-owner/source-file evidence and contradictory known file locations.

End-to-end checks cover exact singleton spans/targets without debug metadata,
unchanged keys after unrelated insertion/reordering, body edits, group membership
edits, deterministic repeats, checkout relocation and export/hydrate/export.
Existing inventory cancellation, cleanup and atomic publication regressions pass.

#### Limits and execution notes

- The private recurring-billing fixture was unavailable and explicitly skipped.
- No `.sonar/config.json` exists, so SonarMCP is inactive.
- CodeScene does not support `.kts`; its installation verification passed 5/5.
  The Gradle script was verified by the actual adapter build instead.
- Maven initially hit a sandboxed global cache lock. Offline execution with
  `-Daether.syncContext.named.factory=rwlock-local` resolved it. Gradle acceptance
  required access to existing tool caches outside the sandbox.
- One test batch overlapped an adapter reinstall and failed class loading. With
  resources held stable, the complete affected batches passed. Tests and final
  package verification use the same final adapter resources.
- Unsupported compiler/processor configurations retain diagnosed fallback;
  compiler-native fallback IDs can still be renumbered by javac.


<a id="source-evidence-analyzers-java-lambda-owner-refresh-md"></a>

## evidence/analyzers/java-lambda-owner-refresh.md

Source: [evidence/analyzers/java-lambda-owner-refresh.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/java-lambda-owner-refresh.md). Historical record; retained rationale, methods and limitations.

### Java lambda owner refresh regression (TASK-33)

#### Cause and correction

During processor-disabled javac attribution, an unresolved generated-getter method
reference after a stream filter can make an expression resolve to an error
TypeElement named `java.util.stream.Stream$map`. Treating any ancestor TypeElement
as the lambda owner assigned lambdas from two TaxReduction files to that type,
triggering the strict Rust cross-file check.

`LambdaOwner` now resolves enclosing class declarations only. Correlation sites,
labels, label contexts and identity eligibility share declaration-only checks.
Strict cross-file validation remains and reports the owner and both paths.
Inventory cache keys include the adapter JAR content hash; the wire format remains
`kind_v8` and source inventory version 8.

#### Regression-first evidence

A simple map lambda did not reproduce. The real-JVM fixture compiling two nested
classes with getters, then removing those getters before source inventory,
reproduced the failure when a filter lambda was followed by `map(Item::getValue)`.
Before the fix, its owner assertion failed with `java.util.stream.Stream$map`
instead of `fixture.First$Nested` and `fixture.Second$Nested`.
After the fix, both owners and labels are lexical, with unavailable replay reasons
rather than invented compiled/source bindings. Rust end-to-end coverage verifies
inventory loading, graph validation, file ownership, readable labels and diagnostics.
Valid stream lambdas and local/anonymous type fallback have real-JVM coverage too.

#### Verification

- Adapter Gradle offline `installDist`: passed with existing Gradle cache access.
- `cargo test --workspace --locked`: all unignored tests passed.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: passed.
- Python Java lambda correlation: 12 passed; Java context: 9 passed.
- Python packaging: 10 passed.
- Rust unresolved-stream normalized-graph regression: passed.
- Rust cache invalidation and strict correlation boundary tests: passed.
- Java correlation and identity integration acceptance: 4 + 3 passed.
- Packaged Java resources acceptance: 2 passed; existing Gradle cache access required.
- Full `python3 tools/package.py`: passed, including desktop signing verification.
- Development, standalone engine and desktop adapter JAR SHA-256 match:
  `524f507199d2d0d05fe8eee2084444df99c37dc21d37eff3272ac32bc23a236e`.
- Actual packaged desktop engine process, using its bundled resources: a stream
  filter/map fixture produced three nodes and two portable lambda identities.
- Independent reviewer: no actionable findings.
- CodeScene: all 9 modified/new eligible source files score 10.0; pre-commit
  safeguard passed 9/9 eligible files, with no regressions.
- No `.sonar/config.json`; SonarMCP is inactive.

#### Original project retry

Used the saved successful Maven metadata with the existing
`recurring_billing_successful_build_outputs_preserve_both_generated_copies`
acceptance test; no application build or application-source edits were performed.
All 13 compiled contexts completed: **10,827 declarations, 17,331 relationships**.
The analyzer checks source and bytecode snapshots for changes during this run.
Both TaxReduction lambda owners now refer to their respective ArticleType classes.
Compiler replay remains unavailable where generated getters cannot be attributed,
with reason `Source attribution prevents compiler replay`.

Ephemeral detailed outputs: `/tmp/codegraph-task33-billing.json`,
`/tmp/codegraph-task33-core.tsv`, and `/tmp/codegraph-task33-packaged.json`.
Planning, exploration and implementation ran in the main agent; independent review
and integration acceptance testing ran in separate reviewer/tester agents.


<a id="source-evidence-analyzers-lombok-signatures-md"></a>

## evidence/analyzers/lombok-signatures.md

Source: [evidence/analyzers/lombok-signatures.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/lombok-signatures.md). Historical record; retained rationale, methods and limitations.

### Lombok signature resolution — TASK-14

Validated on 2026-11-09 with JDK 25 and pinned Lombok 1.18.48.

#### Regression and behavior

The original helper fails on the public fixture's `fixture.Value.ValueBuilder`
signature despite successful Lombok compilation. The fixed inventory keeps its
pre-attribution source declaration snapshot and resolves qualified error types
through a separate compiled-only compiler context. Compiler-qualified declaration
names handle generic `@SuperBuilder` error mirrors that retain type arguments even
after erasure. Binary names preserve nested-class dollar separators.

Unqualified unresolved names fail closed, including when an unrelated class of
the same name exists in the default package. Compiled lookup preserves the owning
classpath order and cannot load source files from that classpath. No processor is
run during inventory and no generated builder methods enter the authored graph.

The Rust coordinator writes inventory into a same-directory temporary file and
publishes it by atomic rename after process success and a cancellation check.
Tests cover partial failure, successful retry/cache reuse, running-process
cancellation, cancellation just before publication, and immediate cleanup after
rename failure even while the returned error remains alive.

#### Public verification

- `cargo test --test java_lombok -- --ignored --nocapture`: 2 passed, real Maven
  and Gradle builds. Exact signatures cover record/class builders, inherited
  generic builders, builder arrays and generic bounds. Authored edges survive;
  only the seven authored methods remain.
- Existing `java_builds`: 8 passed; duplicate-context acceptance: 2 passed;
  `packaged_java`: 2 passed. The private Recurring test is run separately.
- `cargo test --lib analyzers::java::tests -- --ignored --nocapture`: 4 passed.
- `python3 -m unittest discover -s tests`: 37 passed, including four real-JVM
  inventory boundary tests.
- `cargo test --all-targets` and `cargo clippy --all-targets -- -D warnings`: passed.
- Changed Rust files pass `rustfmt --check`; `git diff --check` passes.
  Repository-wide `cargo fmt --all --check` reports pre-existing formatting in
  vendored Rune sources, which were not changed.
- Independent review findings resolved and re-reviewed. CodeScene reviews score
  all seven eligible changed source files 10.0; safeguard passes with no regressions.
- Sonar is inactive because this project has no `.sonar/config.json`.

An early acceptance attempt raced a helper distribution rebuild and failed to
load an anonymous helper class. The stable-artifact rerun of every affected public
suite passed. The first full private analysis completed but its newly added test
assertion incorrectly compared a short container name against a qualified name;
the assertion was corrected before the final acceptance run.

#### Packaging

`python3 tools/package.py` rebuilt, signed and verified `dist/CodeGraph.app` and
produced the ZIP. The packaged helper SHA256 matches the tested development helper:
`94ea6b2da60e26ecd487f7974616ea67574a98f1546f5d33b3590edb787105c9`.

Private full graph JSON, logs and screenshots remain outside this repository.
No private source or credentials are included in this evidence.

#### Full Recurring acceptance

The final production Maven acceptance run completed all 13 compile contexts and
passed: **10,800 declarations and 17,293 relationships**. Graph validation passed.
The authored `F3DefaultsServiceImpl.buildCommonDefaults` method occurs once and
has the exact compiled nested-builder return type. Existing assertions retain
both modules' duplicate generated declarations and source/generated ownership.

Reproduction command (requires the existing private checkout/dependency access):

```sh
CODEGRAPH_RECURRING_BILLING=/path/to/Recurring-billing \
  cargo test --test java_duplicates \
  recurring_billing_retains_duplicate_generated_declarations -- --ignored --nocapture
```

The final packaged executable independently analyzed the private project from
scratch and published the same **10,800 symbols / 17,293 relationships** into an
isolated temporary SQLite store. Read-only SQL checks confirmed the exact formerly
failing signature and ten generated classes in each of the two expected modules.
The same packaged executable loaded this generation using `--scene cached` and
rendered the real project graph through headless Metal (181 frames at 60 fps).
The screenshot was visually inspected: the successful-analysis status, project
nodes and relationships are visible, without an analysis error. Native physical
window and trackpad interaction were not repeated for this backend fix.


<a id="source-evidence-analyzers-maven-md"></a>

## evidence/analyzers/maven.md

Source: [evidence/analyzers/maven.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/maven.md). Historical record; retained rationale, methods and limitations.

### Maven acceptance

Verified on Apple Silicon macOS with JDK 25 on 2026-11-08.

#### Build-tool matrix

| Runtime | Result |
| --- | --- |
| Maven 3.9.9, pinned distribution | All 7 Maven acceptance tests passed |
| Maven 3.10.0, installed distribution | All 7 Maven acceptance tests passed |
| Maven 4.0.0-rc-7, pinned distribution | All 7 Maven acceptance tests passed |
| Gradle 9.4.0, repository wrapper | Multi-module regression passed before and after extraction |

Distribution checksums are recorded in
[the metadata extension README](<../engineering/doc-28 - Maven-metadata-extension-internals.md>).
The extension builds with JDK 25 and targets Java 17 bytecode so Maven 3.9's
component scanner discovers it. Maven API dependencies are compile-only and
locked; the distributed extension contains only CodeGraph classes.

The real-build suite exercises standalone and reactor builds, sibling modules,
an external parent POM, default and explicitly configured profiles, custom
source/output directories, generated Java, test compilation, and a JUnit
execution sentinel. It verifies cross-module calls, stable identities, entry
points, source locations, production/test/generated ownership, generated-source
fingerprints, external resource edits, and external parent edits.

Failure scenarios cover skipped compilation with missing expected output,
successful commands without metadata, edits during compilation, cancellation,
failed refresh with a retained SQLite generation, and overlapping reactors
rejected before a compilation checkpoint. Maven 4 can create output directories
during validation, so overlap checks assert no compiled/generated Java rather
than assuming output directories do not exist.

#### Regression and distribution checks

- Full Rust suite: **96 passed**, including ignored compiler-backed tests,
  native UI interaction tests, and two packaged-resource acceptance tests.
- Full Python suite: **27 passed**.
- Clippy with warnings denied for project code: passed.
- CodeScene: modified source files reviewed; final safeguard passed with
  32 eligible files checked and no issues.
- SonarMCP inactive: this project has no `.sonar/config.json`.
- Signed `dist/CodeGraph.app` and `dist/CodeGraph.zip` built successfully;
  deep/strict signature verification passed.
- Actual packaged CLI analyzed a temporary standalone fixture through both an
  explicit `pom.xml` and its containing folder. Each published 4 symbols and
  1 relationship using an isolated SQLite cache.
- Packaged-resource tests analyze Maven and Gradle with copied resources and
  reject a missing bundled Maven extension without development-path fallback.

The final preparation-log diagnostic was additionally checked with the focused
missing-metadata regression and Clippy after the full Rust suite.

#### Reproduction

```sh
python3.13 tools/bootstrap.py
cargo test --all-targets -- --include-ignored --test-threads=1
python3.13 -m unittest discover -s tests -v
cargo clippy --all-targets --no-deps -- -D warnings

CODEGRAPH_TEST_MAVEN=/absolute/path/to/apache-maven-3.9.9/bin/mvn \
  cargo test --test java_builds maven_ -- --ignored --test-threads=1
CODEGRAPH_TEST_MAVEN=/absolute/path/to/apache-maven-4.0.0-rc-7/bin/mvn \
  cargo test --test java_builds maven_ -- --ignored --test-threads=1

python3.13 tools/package.py
```

Use `CARGO_INCREMENTAL=0` if local disk space is constrained. Format only this
package (`cargo fmt --package codegraph`) to avoid formatting vendored workspaces.

#### Behavior and limits

Maven uses project/user configuration and the nearest ancestor wrapper, then
installed Maven. The adapter observes evaluated Maven models and compiles through
`test-compile` with standard test execution skipped. Ordinary plugins bound to
earlier phases can execute. Explicit extension-classpath definitions retain
Maven's effective precedence before the CodeGraph extension is appended.

Maven 4's generated `.mvn/target/project-local-repo` does not stale authored
inputs. Configuration changes under `.mvn` remain tracked. Custom output roots
and missing optional inputs survive fingerprint serialization. Legacy saved
Gradle selections do not acquire Maven targets sharing the same directory; new
sessions store target kind as well as path, including explicit empty selections.

Java is supported; Kotlin, Scala, Android, later-lifecycle specialized packaging,
and new profile/property controls are outside this increment. Compatibility
claims above apply to the tested distributions; future Maven releases still
require validation.


<a id="source-evidence-analyzers-native-pipeline-md"></a>

## evidence/analyzers/native-pipeline.md

Source: [evidence/analyzers/native-pipeline.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/native-pipeline.md). Historical record; retained rationale, methods and limitations.

### Native analyzer pipeline acceptance

Validated on 2026-11-07 using the Rust pipeline shared by the app and
the helper now at `crates/codegraph-engine/src/bin/analyzer_check.rs`
(`cargo run -p codegraph-engine --bin analyzer_check -- ...`). These checks compile projects but do not launch the
applications or run their tests. Engine revisions and the CallAtlas identity
serialization patch are pinned in `analyzers.lock.json`.

| Case | Normalized symbols | Normalized relationships | Result |
|---|---:|---:|---|
| .NET multi-project fixture | 12 | 12 | Passed |
| Java 25 Gradle fixture | 13 | 14 | Passed |
| PacificFate solution | 20,494 | 75,881 | Passed |
| PacificFate.Core selected alone | 8,191 | 18,093 | Passed |
| StoryEngine Gradle backend | 14,504 | 22,326 | Passed |
| Temporary custom Gradle source-set fixture | 9 | 3 | Passed |

The PacificFate result includes 585 separately labeled possible-target
relationships reported by CallAtlas. Symbol identities include the analyzer's
assembly identity and the selected build path. Linked Core files are owned using
evaluated MSBuild `Compile` items rather than their directory location.

The StoryEngine result retains all 13,987 compiled method declarations, plus 517
explicit inherited/reference placeholders supplied by the analyzer. The latter
have `body_unavailable=true`; the app can expose them without pretending to have
resolved their bodies. Return types remain part of Java symbol identity to
distinguish compiler bridge methods. External receiver classes are excluded
using compiled-input class ownership, never package-name guesses.

#### Authored callbacks remain visible

The native Java normalization retained 1,635 authored synthetic lambda methods as
production source rather than hiding them as generated source. Compiler
synthetic flags alone do not imply that the source was generated.

The real-compiler regression test
`authored_lambda_flow_survives_the_default_production_filter` compiles a Java
class with `start() -> lambda -> leaf()`, executes the pinned bytecode analyzer,
normalizes its output and finds that three-node path with `Filter::default()`.
It passed using the installed JDK 25. The test is explicitly invoked because it
requires the installed SDK and bootstrapped analyzer distribution:

```sh
cargo test --lib --offline authored_lambda_flow -- --ignored
```

#### Build ownership and freshness

The Gradle init script enumerates every configured Java source set and only
depends on its Java compilation task. A separate fixture with an `integrationTest`
source set registered with a Gradle `Test` task retained three integration-test
declarations. An additional custom production source set retained three methods
in the production view, alongside the three main-source declarations. Classification
uses public Gradle `Test.testClassesDirs` ownership; unassigned custom source sets
remain visible as production rather than being hidden by their name. No
test-execution task ran.

Ownership is exported before compilation. The pipeline fingerprints authored
sources, build definitions and recursively reported compile/generator task input
files, including StoryEngine's `storyengine-api.yaml`. Compiled bytecode is
separately fingerprinted while the engine reads it. Changed inputs reject the
refresh rather than publishing an older graph under a newer fingerprint.

C# fingerprints also include evaluated `MSBuildAllProjects` imports and
`AdditionalFiles`. `Compile.RecursiveDir` identifies linked glob roots, allowing
new linked files to invalidate the graph even when the project itself is in a
different directory. The individual PacificFate.Core pipeline also passed and
persisted the external `/PacificFate/core` glob-root fingerprint. A regression test adds an external `core/new-domain/Added.cs`
and confirms that the pre-analysis snapshot is rejected.

Directory fingerprints use filtered entry names, not modification timestamps,
so creation of ignored `build`, `bin` and `obj` directories does not invalidate
source state. File fingerprints retain size, modification time and BLAKE3 hash.

#### Cancellation and limitations

The native cancellation test starts a process with a delayed descendant write,
cancels its owned process group and confirms that the descendant does not perform
the write. It passed in approximately 1.4 seconds. Already-cancelled work is also
verified not to start a process.

Compiler and build failures, empty bytecode output and analyzer frame limits fail
the operation. Known upstream gaps remain visible in graph diagnostics: most C#
source-generator output, incomplete operator-invocation coverage, limited
cross-assembly dispatch candidates, and unsupported dynamic/DI/framework wiring.
Java bytecode provides source file names and line tables but no source columns.

All owned Rust integration modules received CodeScene Code Health 10.0 reviews.


<a id="source-evidence-analyzers-terminal-launch-readme-md"></a>

## evidence/analyzers/terminal-launch/README.md

Source: [evidence/analyzers/terminal-launch/README.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/terminal-launch/README.md). Historical record; retained rationale, methods and limitations.

### Terminal-launched Gradle suspension — TASK-18

The live development app was waiting in `run_process` for a Gradle wrapper client
that macOS had suspended. Gradle had already exported complete ownership metadata
and printed `BUILD SUCCESSFUL in 5s`.

The app was the foreground group on its controlling terminal. The Gradle client
ran in its own background process group, as required for cancellation, but
inherited fd 0 from that terminal. Gradle's input forwarding attempted a terminal
read, causing `SIGTTIN`. The parent kept waiting for a process that could not exit.

A controlled reproduction using StoryEngine's wrapper and the user's SDKMAN
25.0.2-jbr confirmed:

| Standard input | Result | Time |
|---|---|---:|
| Inherited controlling terminal | Stopped with `SIGTTIN`, metadata already exported | 5.459 s |
| Closed (`/dev/null`) | Exited successfully | 4.232 s |

The fix gives one-shot analyzer processes `Stdio::null()` for stdin. Captured
stdout/stderr, separate process groups, cancellation, deadlines and exit validation
are unchanged. TypeScript's two-phase worker retains its explicit protocol pipe.

The regression launches the actual private Rust process runner in a separate
process under a Python-created controlling PTY. Its analyzer child reads fd 0;
before the fix the controller observed stopped state `T` and failed the test.
After the fix it verifies EOF and non-terminal stdin. Cleanup only targets the
helper session's process groups and reaps the helper.

Verification:

- Terminal regression failed before the fix and passed after it.
- Five process/worker/cancellation tests passed.
- All-target Clippy passed with warnings denied for project code.
- Both modified source files scored CodeScene 10.0; the two-file safeguard passed.
- The fixed **debug** CLI completed full StoryEngine analysis under a controlling
  terminal with SDKMAN 25.0.2-jbr in **42.188 seconds**: 8,136 symbols and 11,112
  relationships. Its graph cache was isolated from the user's workspace.
- Independent review found no remaining issues.

The previous performance measurements used non-terminal stdin and therefore did
not exercise this failure. The original user analysis process was not signalled,
cancelled or replaced during investigation. Restart the app to load the rebuilt
executable; the already-running process cannot pick up this change.

Evidence: [diagnosis](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/terminal-launch/diagnosis.json), [full terminal run](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/terminal-launch/storyengine-terminal.json).


<a id="source-evidence-analyzers-typescript-md"></a>

## evidence/analyzers/typescript.md

Source: [evidence/analyzers/typescript.md](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/analyzers/typescript.md). Historical record; retained rationale, methods and limitations.

### TypeScript, React and Next.js acceptance

Validated 2026-11-09 on Apple Silicon macOS with Node 24.15.0,
project-installed TypeScript 6.0.3, React 19.3.1 and Next.js 16.4.2.
The supported compiler range is 6.0.x with Node 24+; other compiler versions
fail explicitly rather than silently changing binding behavior.

#### Real-project and packaging gate

The signed packaged executable analyzed PacificFateMapEditor/tsconfig.json
without emitting JavaScript, running scripts, installing dependencies or launching
the editor. It published **5,850 symbols and 9,090 relationships**, including
`Page -> EditorPage` as `react-render`, certainty `possible`. `RootLayout` and
`Page` are framework entries; neither is labeled a verified executable start.
A timed run took **8.68 s**, with **867,942,400 bytes maximum resident set size**
reported by macOS time (process/toolchain high-water measurement, not a total
live-memory budget). Disk-image generation was active concurrently.

A second packaged run combined the C# fixture, Gradle fixture, standalone Maven
fixture and PacificFateMapEditor into **5,879 symbols and 9,117 relationships**.
It preserved both framework entries separately from three verified C#/Java starts.
`typescript-summary.json` records languages, relationship kinds, engine versions,
representative effect/event bindings and unresolved counts.

`../ui/typescript/map-editor.png` and its UI tree show the actual cached graph,
searching the real page source, focusing `Page`, its possible render edge to
`EditorPage`, and the explicit framework-entry Inspector label. The screenshot
uses the packaged app and production Graphviz layout, with no synthetic graph.
Headless native Metal interaction passed; physical mouse/trackpad validation is
not claimed.

#### Compiler and integration regressions

The Node fixtures create temporary projects against actual installed compiler,
React and Next.js dependencies. They cover aliases/re-exports, overloads, anonymous
callback ownership, constructor fields, accessors including bracket/compound
writes, conditional function targets, source-only project references, config/type
errors, React callbacks/cleanup/references and Next.js export conventions.
Negative fixtures prevent shadowed hooks, cached callbacks, private app folders,
non-route utility exports and unsupported delegation from creating proven calls.

Seven Rust production-pipeline tests cover callback attribution, reference closure
deduplication, conflicting owners, consumed dependency declarations and source
membership freshness, framework provenance/cache round trips, packaged resource
layout, CLI failed-refresh preservation and cancellation. Discovery tests also
protect Java packages named coverage/dist from TypeScript output exclusions.
Old generation metadata loads with empty framework-entry/provenance defaults.

Validation commands:

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --all-targets --locked -- --include-ignored --test-threads=1
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo clippy --all-targets --no-deps -- -D warnings
python3.13 -m unittest discover -s tests -v
node --test tools/typescript/adapter.test.js
python3.13 tools/package.py
```

All 108 Rust tests (including existing C#/Java compiler and packaged-resource
regressions), 27 Python tests and all 19 Node compiler fixtures passed. Packaging
validated pinned dependency sources, bundled the bridge and Graphviz, generated
ZIP/app artifacts, and verified the ad hoc signature. The refreshed DMG also passed
`hdiutil verify`. The final CodeScene safeguard passed across all 44 eligible files. The upstream Rune release
build retains its existing dead-code warning; CodeGraph Clippy passed with
warnings denied.

CodeScene reviewed all modified source. All modified source modules score 10.0 after compiler setup, dependency inputs,
accessor handling and framework rules were separated by responsibility.
Independent planning/exploration, implementation, static review and compiler
acceptance testing were performed. Sonar is inactive because this project has
no .sonar/config.json.

#### Accuracy and compatibility

Calls are bound through public compiler APIs. React/framework rules remain
separate from compiler binding and carry explicit provenance. Propagated callback
targets are possible; references and rendering do not establish runtime ordering,
frequency or context-specific execution. The real project reports 616 unresolved
dynamic call sites and explicit unresolved React callback/component/event/spread
counts. Mutable refs, callback maps and withStableCallbacks delegation are not
resolved by arbitrary data-flow guesses.

JavaScript/allowJs, Pages Router, custom Next.js routing configuration and general
higher-order-function propagation remain outside this release. TypeScript config
inheritance, project references, missing resolution candidates, dependency type
inputs and watched source directory membership participate in freshness checks.
Errors, cancellation and stale inputs cannot replace a successful generation.
