# Maven acceptance

Verified on Apple Silicon macOS with JDK 25 on 2026-11-08.

## Build-tool matrix

| Runtime | Result |
| --- | --- |
| Maven 3.9.9, pinned distribution | All 7 Maven acceptance tests passed |
| Maven 3.10.0, installed distribution | All 7 Maven acceptance tests passed |
| Maven 4.0.0-rc-7, pinned distribution | All 7 Maven acceptance tests passed |
| Gradle 9.4.0, repository wrapper | Multi-module regression passed before and after extraction |

Distribution checksums are recorded in
[the metadata extension README](../../tools/maven-metadata/README.md).
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

## Regression and distribution checks

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

## Reproduction

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

## Behavior and limits

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
