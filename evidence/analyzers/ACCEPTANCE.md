# Analyzer acceptance evidence

## C#: CallAtlas

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

## Java: Eclipse JDT LS

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

## Accepted Java replacement: java-callgraph2

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
