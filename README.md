# CodeCallNavigator

A native Apple Silicon macOS app for exploring call graphs in C#, Java and
TypeScript repositories.

Select a repository, choose its projects, and generate a graph. Browse by project,
namespace, folder, architectural layer or method; follow callers and callees,
search classes and methods, and inspect source and relationship provenance.
Java and TypeScript methods also support function flowcharts.

## Requirements

- Apple Silicon Mac; packaged builds require macOS 27.0+.
- To build the app: Rust/Cargo, Xcode command-line tools, Python 3.12+ and
  Graphviz 16.1.0 (`dot` on PATH).
- For C#: .NET 10 plus the SDKs and reference packs your projects need.
- For Java: JDK 25, plus a working Gradle wrapper or Maven 3.9+/4 and the
  toolchains your projects need.
- For TypeScript: Node 24+ and project-installed TypeScript 6.0.x.

Analysis can restore dependencies and compile source sets, including tests.
It does not run applications or test-execution tasks. Toolchains must already
be installed. Linux and Intel Macs are not currently supported.

## Build and run

```sh
./dev.sh             # Bootstrap, build and launch
./optimized-dev.sh   # Launch with release optimization
./prod.sh            # Package the app, ZIP and DMG in dist/
```

The scripts default to Python 3.13; set `PYTHON=python3.12` to use another
installed Python 3.12+ interpreter. Each script cleans Cargo artifacts before
building, so every run performs a full Rust rebuild. Local packages are ad hoc
signed and are not notarized.

## Use the app

1. Open a repository and select its .NET projects, Gradle/Maven builds or
   TypeScript configurations.
2. Generate the graph, then choose a grouping or search for a class or method.
3. Select an element to explore callers and callees. Double-click groups to drill
   down; use Back/Forward to retrace your exploration.

Drag to pan, scroll or pinch to zoom, and use Fit to frame the graph. Filters
control code visibility, relationship types and target resolution. Possible
targets and function references do not prove runtime execution.

Refresh manually after source or build changes. Failed or canceled analysis
preserves the last successful graph. **Clear cache** removes reusable inference
data while keeping that graph and navigation.

## Documentation

- [Detailed reference](docs/reference.md): controls, search, cache, build steps,
  analyzer behavior and toolchain-specific validation.
- [Standalone engine and CLI](docs/standalone-engine.md)
- [Function flowcharts](docs/function-flowcharts.md)
- [Architectural layers](docs/architectural-layers.md)
- [Analyzer acceptance](evidence/analyzers/ACCEPTANCE.md) and
  [validation results](evidence/VALIDATION.md)
- [Release and redistribution](docs/public-releases.md)

## Validation

```sh
./verify-light.sh   # Small changes with minimal production-code effect
./verify-full.sh    # Complete runtime/analyzer/benchmark/package verification
```

Light runs static checks and cleanup for documentation-only changes; otherwise it also
checks, lints and tests the workspace. Full preflights compiler toolchains and
explicit dependency/profiling inputs, runs ignored acceptance tests in release
mode on an isolated profiling snapshot, and verifies local packaging. Both stop
on failures/timeouts and preserve stage logs and results under `target/verification`.

See [verification setup and commands](docs/reference.md#validation), including
required full-mode inputs. Use `--dry-run` to inspect either script without running
checks. Both clean unused artifacts afterward: light keeps compiled dependency
caches; full reclaims debug/release build profiles. Use `--keep-artifacts` to retain
build state for investigation. Neither script formats vendored sources or rewrites
project files.

## License and feedback

Copyright (c) 2026 Zsolt Gajdacs. Licensed under [AGPL-3.0-only](LICENSE);
see [third-party notices](THIRD_PARTY_NOTICES.md) for bundled software.
Analyzing a repository does not change its license or require publication of
its source.

This is a personal project accepting [bug reports and feature requests](https://github.com/ZsoltGajdacs/CodeCallNavigator/issues/new/choose).
Outside code and documentation submissions are not accepted; support is not
guaranteed. See [contribution policy](CONTRIBUTING.md),
[conduct policy](CODE_OF_CONDUCT.md) and [security reporting](SECURITY.md).
