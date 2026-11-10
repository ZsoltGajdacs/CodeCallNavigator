# Architectural layer verification

Verified on 2026-11-09 on Apple Silicon macOS.

## Checks

- `cargo test --all-targets --offline --quiet`: 185 passed, 39 ignored. The ignored
  cases need real toolchains or private-project inputs and were exercised separately.
- Real-toolchain checks: 37 of the ignored cases passed across Roslyn, Java source
  inventory, Maven/Gradle, packaged analyzer resources and TypeScript workers.
- Two private-project cases could not run because `CODEGRAPH_COMPILED_METADATA`
  and/or `CODEGRAPH_RECURRING_BILLING` were unset: the archived successful-build
  metadata check and the recurring-billing generated-declaration check. Invoking
  them without those inputs fails at their explicit environment preconditions;
  this is not reported as a pass.
- `python3.13 -m unittest discover -s tests -v`: 48 passed.
- `node --test tools/typescript/*.test.js`: 31 passed using the installed compiler.
- `cargo clippy --all-targets --offline --no-deps -- -D warnings`: passed.
- `cargo fmt --package codegraph --check`: passed. Use the package scope to avoid
  formatting dependency workspaces.
- CodeScene source reviews prompted focused extraction of matching, grouping,
  geometry, controls and test fixtures. The final safeguard passed with no
  regressions across 56 eligible files (74 modified files at the check).
- SonarMCP is inactive because this repository has no `.sonar/config.json`.

The new configuration tests cover first-match semantics, conjunctive matching,
attribute suffix normalization, test precedence, inheritance/conversion failures,
native-file priority, nested package paths, dotted Java build directories and
canonical repository paths. Real analyzer acceptance covers type-only declarations,
SOURCE-retained Java annotations, C# linked sources compiled in separate projects,
TypeScript decorators and transitive inheritance, stable graph identities and cache
roundtrips. UI/lifecycle tests cover recovery after invalid edits, selection/tab
persistence, obsolete jobs, filtered relationship counts and control overlap.

## Packaging

The normal package provenance check refused the existing local Rune checkout:
its 1,171-file formatting diff predates this task (sample modification timestamps
were 16:11–16:13 local time, before implementation began). The checkout was
preserved. A clean-copy formatter comparison confirmed the file set but not an
exact byte match, so no restore was performed. A backup of the local diff was
kept outside the repository.

Packaging was instead run from an isolated root under `/private/tmp` with the
current CodeGraph manifest/lock/source, the clean pinned Rune revision, the exact
checked-in CallAtlas patch and the current analyzer artifacts. `tools/package.py`
ran its unchanged provenance gate, release build, dependency/license bundling,
ad-hoc signing and strict signature verification successfully. The resulting app
and ZIP are under `dist/`. This does not claim the unmodified packaging command
can pass against the existing dirty vendor checkout.

The packaged binary then analyzed the Maven standalone fixture with `--layers
spring`, using bundled analyzer resources, and published a graph successfully.
Its classification summary is [packaged-cli.txt](packaged-cli.txt).

## Visual evidence

[Four native Metal captures](../ui/architectural-layers/README.md) cover light/dark
appearance at 1420×900 and 1050×680. They use explicit synthetic graph data through
production Graphviz geometry. They do not substitute for the real compiler tests.
Physical trackpad and VoiceOver validation were not performed.
