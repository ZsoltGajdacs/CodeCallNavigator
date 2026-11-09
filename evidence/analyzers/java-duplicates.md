# Java duplicate-class analysis

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

## Source-focused engine scope

The user clarified that relations between actual project source files are the
priority, rather than whole-dependency analysis. The proposed reduced-precision
retry was removed. Engine failures and frame-snapshot limits still fail analysis;
there is no precision downgrade or retry policy.

Verbose upstream expression-exclusion logging is disabled because it repeats
the project class-name filter for each excluded method. Structured CodeGraph
visibility diagnostics remain enabled. Successful context scratch directories
are deleted immediately to bound disk consumption; a failing production
context retains its files for diagnosis.

## Reproduction

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

## Validation record

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
