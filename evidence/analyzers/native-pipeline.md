# Native analyzer pipeline acceptance

Validated on 2026-11-07 using the Rust pipeline shared by the app and
`src/bin/analyzer_check.rs`. These checks compile projects but do not launch the
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

## Authored callbacks remain visible

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

## Build ownership and freshness

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

## Cancellation and limitations

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
