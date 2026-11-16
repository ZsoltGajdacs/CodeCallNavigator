---
id: doc-12
title: Testing strategy
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T18:07:00Z'
---
# Testing strategy

Migrated 2026-11-16 from `docs/testing-strategy.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

Test each behavior at the lowest and fastest layer that reliably detects its
failure. Verification quality takes precedence over speed. Aim for roughly one
minute of warm unit execution without enforcing a time budget. Do not add shallow
tests to improve a ratio, remove unique boundary checks, or shorten necessary
failure/recovery windows to meet that aspiration.

## Categories and ownership

| Category | What it proves | Appropriate dependencies |
| --- | --- | --- |
| Unit | An isolated production policy, transformation, algorithm or validation rule | Explicit in-memory data; standard serialization; fixture data as input |
| Integration | Collaboration or a real boundary | Temporary files, SQLite, Git, Graphviz, compilers, subprocesses, headless production UI |
| Acceptance | An assembled product journey through its public boundary | Public CLI, real MCP client/adapter/CLI, native desktop window, installed packages |
| Performance | Measured cost of production behavior | Release builds, isolated profiling copies, native GPU when applicable |

Classify by what executes, not source directory, framework annotation or ignored
status. Loading a fixture does not itself establish integration. A compiler test
that bypasses the public engine is integration, not engine acceptance. A headless
Rune test is UI integration, not native-window acceptance. Querying a saved real
artifact can prove a complete CLI query journey without recompiling that artifact.
A report-parser test cannot establish the application acceptance reported by it.

Core owns graph/flow semantics and artifact logic; engine owns build/compiler
extraction and public CLI operations; desktop owns presentation and interaction;
MCP owns argument/wire validation and process lifecycle only. TypeScript analyzer
helpers count as engine tests even though they live under `tools/`. Tooling owns
verification, packaging and harness behavior. Shared contract changes require both
owner and consumer checks. Keep MCP independent of application libraries in tests.

Use real production operations and assert observable results, including failures,
uncertainty, provenance, source identity and preservation of prior artifacts.
Extract cohesive pure logic when it improves production design; do not extract
artificial seams or replicate an algorithm just to create cheap tests.

Do not mock business behavior, fabricate successful compiler/CLI outputs, or build
replacement services that reproduce production decisions. Real temporary resources
and fixture data are welcome. Narrow process/protocol fault probes may reproduce
failures that real dependencies cannot reliably produce: blocked pipes, stalled
children, malformed frames or premature disconnects. Explain that necessity and
classify them as integration tests. They do not replace successful real-dependency
journeys. Existing test doubles require explicit migration assessment rather than
being relabeled as real acceptance.

## Commands and gates

Run commands from the workspace root:

```sh
./verify-changes.sh
./verify-changes.sh --format json
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

Agents must use this entrypoint for verification and test execution, rather than
raw test commands or manually selected category/component scripts. Reporting is
the default and creates no build or evidence artifacts. Execute the entire
computed plan with `--run`. There is no manual full-verification override.

The checked-in versioned rules in `tools/verification/change-rules.json` combine
by union. Desktop code selects UI correctness; core selects core and engine/UI/MCP
consumer correctness; engine and analyzer adapters select engine/UI/MCP; MCP code
selects MCP. Correctness includes every registered unit, integration and acceptance
check for the selected components, including compiler/native boundaries. Dedicated
registered test/gate files select their inventoried component/category pairs;
production modules retain production rules even when they contain tests.

Inventory edits select the union of changed, added and removed registrations from
the comparison base, index and working tree. Explicit fixture, schema, integration
asset and helper ownership rules add their consumers. Fixture and packaged-skill
rules take precedence over documentation-only suffixes through additive selection.
Ordinary Markdown, Backlog task/doc Markdown, licensing and issue templates select
static checks. Unknown paths always require full verification.

Dependencies, manifests, bootstrap, packaging, release workflows, vendor sources,
patches and unknown paths deterministically select full verification. Verification
infrastructure, selector/rule changes and verification-only dependency locks select
tooling correctness checks plus generated-plan equivalence tests; they do not build
or run unrelated application checks. Explicit UI rendering/layout/cache, core storage/artifact/graph/
flow/query, engine query and benchmark paths add performance checks. No elapsed
time, installed-tool availability or agent judgment changes selection. New paths
must receive an explicit reviewed rule or retain the conservative full fallback.

The default comparison is HEAD plus staged, unstaged and nonignored untracked
changes. `--base REF` directly compares that revision with the index and worktree;
it does not fetch or infer a merge base. Deletions, both rename endpoints, staged
changes restored in the worktree, and vendor/submodule changes remain visible.

Full reuses the existing full stage builder. Focused plans share category execution,
prerequisites, bootstrap, reports and cleanup; component unions compile once per
selected target/profile and execute each registration/gate once. Policy exclusions
are distinct from selected stages that did not run. Text and versioned JSON report
paths, rule matches, selected identities, requirements and ordered stages; execution
persists `change-plan.json` beside results. Existing category/component/full/light
scripts remain compatible building blocks, not the agent workflow.

Static/build/CodeScene checks remain applicable. Unavailable prerequisites and
unrun tests are never passing evidence. Run automatically selected full verification
outside the macOS sandbox. See [the validation reference](<doc-21 - Verification-prerequisites-and-operations.md#validation>).

Keep functional acceptance separate from performance and compiler requirements.
Run each test once per selected composite unless a recorded release-specific risk
requires multiple configurations. Use release mode for canvas benchmarks and an
isolated copied profiling store, never the live application store. Parallelize
isolated work; serialize only demonstrated shared-resource constraints.
The core storage-scale benchmark uses its own synthetic SQLite database and needs
no saved application profile. See [current counts by component and category](<../history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-testing-strategy-distribution-md>).

The machine-readable inventory records exact identities, component, category,
requirements and coverage purpose. Its validator must reject missing and duplicate
classification and make standalone harnesses visible. Changes in identity require
inventory updates. Reclassification alone is not a speed improvement.
Name standalone tooling harnesses `*_acceptance.py`, `*_benchmark.py` or
`*_journeys.py` directly under `tools/`, and Rust benchmark executables
`*benchmark*.rs` under `src/bin/`. Discovery excludes `test_` Python modules and
rejects unclassified, stale or duplicate harness paths. Extend discovery explicitly
when introducing a different harness convention.

Inspect the checked-in inventory with
`PYTHONPATH=tools .venv-verification/bin/python -m verification.inventory`.
Use that command's `--add FILE`, explicit owner/category/purpose and optional
`--requirement`/`--test-name` options for new registrations. Mixed files need exact
test names, not a blanket category. Existing metadata is never silently regenerated.
Rust executable lists and the selected Python modules' actual unittest registration
are checked against the inventory before execution. Unsupported Node registrations
fail explicitly. Reviewed Rust fixture `environment_append` settings affect only
their child test group and preserve inherited options; they never modify the
parent environment or replace a real compiler.

Rust unit groups use libtest's default parallelism. Other isolated correctness
groups use four workers to bound nested SDK/compiler oversubscription. Reviewed
`serial_group` metadata, independently of category, reduces only shared-resource
groups to one worker: the private Recurring Billing checkout, the repository
MediatR solution's `obj/bin`, and profiling-machine CPU/GPU/store measurements.
Artifacts execute sequentially, so shared checkouts cannot overlap across targets.
Ignored and ordinary Rust registrations run in separate exact-name groups; only
the ignored group receives `--ignored`. No broad `--include-ignored` run is needed.
Python cases remain serial in one interpreter because existing bootstrap/CI checks
mutate shared module roots and process environment; process isolation is required
before parallelizing those cases. Record a concrete resource conflict before
adding a serial group.

## Cleanup and timing

Each category owns an isolated build workspace, builds its prerequisites once,
and reuses them across that category's selected tests. Before returning, stop/reap
owned processes and clean category-owned Rust outputs plus temporary fixtures and
profiling copies, including after failure, timeout and catchable interruption.
Prove ownership and inactivity before deletion. Never delete another run's build.
Reclaim abandoned workspaces only when their ownership and inactivity are proven.
Scratch directories use short external paths with matching run ownership markers.
This avoids native socket-length limits and keeps independent MCP package builds
outside the repository's Cargo workspace. Verification disables reusable .NET build
servers so they cannot outlive a category and pin its temporary files; it does not
shut down unrelated user build servers. Inconclusive or active usage still prevents
deletion, rather than being reported as successful cleanup.

Preserve downloaded Cargo registry/Git caches, stage logs, timing/acceptance
reports, current packages and one rollback per app name. Cleanup uncertainty or
failure makes verification unsuccessful. `--keep-artifacts` is an explicit
diagnostic exception that reports retained paths and sizes. Report bytes reclaimed
and remaining; compilation outputs must not accumulate silently between categories.
Verification sets `CARGO_CACHE_AUTO_CLEAN_FREQUENCY=never` so Cargo's own global
cache collector cannot remove downloaded dependencies during builds; category
cleanup removes only owned compiled outputs. See [Cargo's cache configuration](https://doc.rust-lang.org/cargo/reference/config.html#cacheauto-clean-frequency).

Report build, setup, execution and cleanup times separately, together with machine,
toolchain, selected tests and cold/warm conditions. Cleanup increases subsequent
compilation cost; expose it rather than claiming execution time is whole-run time.
Compilation stages measure building the test executables. Real SDK compilation
performed by a compiler integration test remains part of that test's execution cost.
Do not fail a correct suite merely for exceeding the approximate unit timing goal.

## Migration and evidence

Keep this work on the current branch and working tree, preserving unrelated work.
Use Backlog to track reviewable stages. Begin with an exact current inventory and
recorded timing baseline, then introduce shared scripts before moving coverage.
Prioritize measured cost, duplicate execution and repeated fixture preparation.

For each migrated or removed expensive test, record:

- Its identity, covered defect scenario, assertions and measured cost.
- The retained lower-layer test identities and remaining real-boundary journeys.
- How the replacement detects the original failure, with evidence rather than a
  claim based on names or test counts.
- Before/after selection, timing and cleanup results, including limitations.

Do not delete tests with unique boundary value. Share immutable fixture preparation
within a run only where isolation remains intact. Preserve the complete union of
selected tests/harnesses, and detect omissions or unintended double execution.

Initial historical evidence is available under `target/verification/`: for example,
`full-l9fmsdlu/results.json` recorded ordinary Rust execution at 226.394 seconds and
the combined release acceptance/benchmark stage at 1774.885 seconds. These are past
stage observations, not current benchmarks or attribution to individual tests. The
older full command used `--include-ignored`, repeating ordinary Rust tests in
release mode. Verify the current command/report before relying on those figures.

A migration is complete only with reviewed coverage mappings, applicable runtime,
native acceptance and Code Health results, truthful prerequisite/failure reporting,
and validated cleanup on successful, failed and interrupted runs. Count/timing
reports alone do not establish preserved verification quality.
