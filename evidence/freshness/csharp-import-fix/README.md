# C# evaluated input coverage (TASK-63)

The real release-mode regression first failed on the existing analyzer:
`obj/Fixture.csproj.nuget.g.targets` disappeared after rewriting the project with
identical bytes. SDK 11.0.100/MSBuild 18.12.2 reports a timestamp-sensitive
`MSBuildAllProjects` value rather than a complete evaluated import inventory.
See the original causal probe in `../csharp-import-probe.json`.

Review also reproduced a consumed design-time-only import omitted by normal preprocessing: actual Roslyn exported DesignOnly but the manifest lacked Design.props. The old built engine also omitted both inner-framework property imports in a valid net10.0/net11.0 fixture. These were blocking input-coverage failures.

The analyzer now obtains actual evaluated imports from strict MSBuild preprocess
frames across normal, pinned Roslyn 5.9 design-time and inner-framework contexts, observes explicit generated/SDK/external files, and rechecks import
membership and content before metadata, after metadata and after CallAtlas.
The [input policy](../../../docs/standalone-engine.md#c-evaluated-build-inputs)
documents the supported format and observed-input limitations.

## Focused verification

- Before production changes: real cold/warm regression failed as expected.
- After changes: the same real regression passed, including independent clean
  full-artifact equality and identical-byte project rewrites.
- Real external nested property-only imports with `--` and `&` paths passed;
  false imports and unframed author comments were excluded. Imported property
  changes were stale, and an invalid imported file preserved the saved artifact.
- Real design-time imports and external linked source scopes are retained; new linked sources are stale. Both inner-framework property imports are retained, and a real solution covers SolutionDir-dependent imports.
- A real SDK round-trip preserves literal semicolon/comma/percent property values without overwriting DesignTimeBuild. The actual compiler consumes a relative design-time import from the special solution path. The separate native SolutionDir-expanded import limitation is retained in literal-property-probe.json; ordinary SolutionDir expansion remains an acceptance case.
- A real conditional IsTestProject/Compile AutoGen regression preserves configured
  normal-root flags while still collecting design-only imports and sources.
- A real external marker swaps A/B imports between normal/design contexts without
  changing file bytes or the union; the per-context inventory check rejects it.
- Real repeated preprocessing rejected a newly appearing external conditional import.
- An empty external referenced project retained its directory scope in the
  published manifest; adding its first source file was detected as stale.
- Parser tests cover root/parent framing, optional parentless end frames,
  escaped-file ambiguity and special paths.
- All 79 core unit tests passed; focused verification tests cover conservative
  legacy C# status, unchanged bytes, stale precedence and Java/TypeScript isolation.

`manifests.json` retains the fixed cold/warm observed-input manifests and version
report. The two canonical artifacts were exactly equal. External SDK imports are
captured in runtime fingerprints; those outside declared portable roots still
produce explicit `unmapped_input`, rather than a false verified match.

`cli-audit-v2/` retains the final public CLI audit under `msbuild-preprocess.v2`:
nine saved-edit scenarios, each with a first run and one warm repetition. All 18
successful refreshes matched independent clean analysis exactly; the harness also
checked invalid-code recovery and generation mismatch rejection. These are
correctness checks, not a new latency distribution. `cli-audit/` retains the
earlier prototype audit for comparison. Raw samples in both directories are
losslessly compressed as `samples.jsonl.gz`; metadata records reproducible
commands and toolchains. The earlier TASK-59 timing series remains unchanged.

Reproduce with installed .NET 11 and pinned analyzer resources:

```sh
CODECALLNAVIGATOR_CSHARP_INPUT_EVIDENCE=/tmp/task63-manifests.json \
  cargo test --locked --release -p codecallnavigator-engine \
  --test csharp_inputs -- --ignored --nocapture
cargo test --locked --release -p codecallnavigator-engine \
  --lib analyzers::csharp::inputs -- --include-ignored
cargo test --locked --release -p codecallnavigator-core --lib
```

Full workspace verification passed all 36 stages with `--timeout 3600`, including
the complete release acceptance suite, profiling snapshot/benchmarks, packaging
and signatures, installed CLI and independent MCP journeys, and CodeScene.
The default 1800-second release-stage limit was insufficient for the two private
Maven builds plus the remaining compiler fixtures; that timeout report is also
retained. No assertion failed in that attempt. See [verification/results.json](verification/results.json)
for the successful retry and the adjacent quality/install logs. The pre-commit
safeguard passed all ten modified source files with no regressions.
