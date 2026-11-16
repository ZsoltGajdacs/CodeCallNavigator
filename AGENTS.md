
<!-- BACKLOG.MD GUIDELINES START -->
<!-- backlog.md-instructions-version: 1.55.3 -->
<CRITICAL_INSTRUCTION>

## Backlog.md Workflow

This project uses Backlog.md for task and project management.

**At the beginning of each conversation in this project, run `backlog instructions overview` before answering or taking action. Re-read it only if you have not read it yet in the current conversation.**

Use the overview to decide whether to search, read, create, or update Backlog tasks.

Before task lifecycle actions, read the matching detailed guide:
- `backlog instructions task-creation` before creating or splitting tasks
- `backlog instructions task-execution` before planning, changing status or assignee, adding a plan or implementation notes, or implementing task work
- `backlog instructions task-finalization` before checking acceptance criteria, writing final summaries, or moving tasks to terminal statuses

Use `backlog <command> --help` before running unfamiliar commands. Help shows options, fields, and examples.

Do not edit Backlog task, draft, document, decision, or milestone markdown files directly. Use the `backlog` CLI so metadata, relationships, and history stay consistent.

</CRITICAL_INSTRUCTION>
<!-- BACKLOG.MD GUIDELINES END -->

## Project Structure and Coding Practices

The workspace practices below apply throughout the project. For work in either
member crate, also read its scoped guide:

- [codecallnavigator-core](crates/codecallnavigator-core/AGENTS.md)
- [codecallnavigator-engine](crates/codecallnavigator-engine/AGENTS.md)

The crate guides supplement the practices and mandatory policies in this file.

## Upstream Dependency Changes

Keep all local changes to upstream projects, including vendored libraries, as
checked-in patch files under `patches/`. Upstream checkout edits must be captured
in those patches; do not rely on unrecorded edits or Git stashes to preserve them.
Document each patch's purpose, upstream base revision, and application order, and
integrate its application into the dependency bootstrap where applicable.

When updating an upstream project, reapply its patches to the new revision,
resolve any conflicts, refresh the patch files and revision/checksum metadata,
and verify the patched integration. Remove patches whose changes have landed
upstream. Avoid formatting unrelated upstream files, and scope formatting tools
to this project's files so they do not rewrite vendored sources.

## CodeScene Code Health

Check AI-modified source files with the CodeScene MCP `code_health_review` tool.
Use its findings to make small, behavior-preserving refactorings, then review the
affected files again. Keep build-tool adapters separate from graph analysis and
avoid duplicating logic between the UI and CLI.

Before committing, run `pre_commit_code_health_safeguard` and resolve reported
regressions. Before opening a pull request, run `analyze_change_set` against the
intended base branch. If a check cannot run, report the limitation instead of
claiming it passed. Code Health checks complement the project's tests and do not
replace compiler, analyzer-acceptance, or packaging verification.

## Ownership and dependencies

| Area | Responsibility |
| --- | --- |
| Root package, `src/` | Native desktop, presentation, interaction, background jobs and engine process client |
| [`codecallnavigator-core`](crates/codecallnavigator-core/AGENTS.md) | Shared contracts, graph/flow operations, canonical artifacts, discovery and storage support |
| [`codecallnavigator-engine`](crates/codecallnavigator-engine/AGENTS.md) | Standalone CLI, analysis lifecycle, build preparation and compiler-backed language adapters |
| `crates/codecallnavigator-mcp/` | Independently installed MCP adapter, public CLI wire contracts and owned process lifecycle |
| `tools/` | Analyzer resources, bootstrap, acceptance helpers and packaging |
| `fixtures/`, `tests/` | Reproducible inputs, frozen baselines and integration checks |
| `.backlog/docs/` | Internal architecture, research, testing policy and durable verification summaries |
| `docs/` | External user, installation, integration and redistribution documentation |
| `vendor/`, `patches/` | Upstream checkouts and the recorded patches applied to them |

Keep production dependencies flowing from desktop and engine into core. Core
must not depend on either consumer. The desktop invokes the engine executable;
do not introduce production calls into its analyzer library. Engine dependencies
used by desktop acceptance tests are distinct from production coupling.
Use [the engine contract](docs/standalone-engine.md) for artifact, protocol and
language-extension behavior.

The MCP package communicates only with the installed CLI for graph information.
It must not depend on core, engine or desktop libraries, including test
dependencies or source inclusion. Keep project/source/artifact/database access,
graph algorithms and graph caching in the CLI; MCP owns argument/wire validation
and subprocess lifecycle. See [its contract and installation](docs/mcp.md).

## Documentation ownership

Internal project documentation belongs in Backlog documents and must be created
or updated through the Backlog CLI. This includes architecture and design
rationale, research, implementation investigations, maintainer runbooks, testing
strategy and durable verification summaries. Keep task scope and execution plans
in Backlog tasks. Use the [internal documentation index](<.backlog/docs/doc-33 - Internal-documentation-index-and-migration-manifest.md>)
to find the canonical document.

Reserve `docs/` for external user, installation, integration, public contract and
redistribution documentation. Split mixed documents by audience and link to the
canonical owner instead of duplicating content. Keep executable fixtures,
schemas, manifests, instruction entry points, package-facing skills and license
notices with their owning code or distribution.

Generated logs, screenshots, traces and reports belong in ignored artifact
directories and must not be checked in. Use `target/verification/` for new run
output; historical local captures under ignored `evidence/` may be retained.

## Organizing changes

- Give each module one cohesive responsibility and a small explicit interface.
  Split by domain responsibility, such as geometry versus interaction or build
  preparation versus symbol normalization, rather than arbitrary line counts.
- Keep shared semantic decisions in core or the owning analyzer, then consume
  their results from the desktop and CLI. Avoid parallel implementations of
  identity, filtering, relationship resolution or artifact conversion.
- Keep entry points and event handlers thin. Move substantial work into named
  operations with explicit inputs and results. Prefer existing modules and
  concrete types over speculative traits, registries or generic frameworks.
- Keep public APIs deliberate; default helpers to private or `pub(crate)`.
  Document invariants and nonobvious tradeoffs near the code that enforces them.

## Rust and runtime practices

- Use domain structs and enums for meaningful states instead of loosely related
  booleans, tuples or string tags. Validate external data at its boundary and
  preserve validated invariants internally.
- Prefer borrowed inputs and clear ownership. Clone large graphs or source text
  only when the operation requires an independent copy; keep expensive work out
  of frame preparation and input handling.
- Return recoverable failures through `Result` with useful operation/path
  context. Reserve `unwrap`, `expect` and panics for demonstrated invariants or
  tests; do not silently convert failed analysis into an empty successful graph.
- Keep mutable state with its owner. Background jobs must support cancellation
  and publish results only when complete; keep blocking subprocess, filesystem
  and analysis work off the UI path.
- Preserve certainty, provenance, source locations and unsupported-case
  diagnostics. A possible relationship or reference does not prove execution.
- Keep changes focused, avoid unrelated formatting, and add dependencies only
  when existing code and standard-library facilities do not meet the need.

## Testing strategy and verification

Follow [the testing strategy](<.backlog/docs/testing/doc-12 - Testing-strategy.md>). Choose the lowest and
fastest layer that can reliably prove the behavior. Unit tests should carry most
behavioral combinations; integration tests prove real boundaries, and a smaller
set of end-to-end journeys proves the assembled application. There is no numerical
quota. Roughly one minute of warm unit-test execution is an aspiration, never a
pass/fail gate or permission to weaken verification.

- Test production code through observable results, not copied implementation or
  internal call sequences. Extract cohesive logic only when it improves design.
- Unit tests use explicit in-memory inputs and isolated logic. Filesystem,
  database, compiler, native-window and application-process behavior belongs in
  integration or acceptance tests with real dependencies and temporary resources.
- Do not mock business behavior, fabricate successful analyzer results, or build
  replacement services that reproduce production decisions. Narrow, explicitly
  documented process/protocol fault probes are permitted for failures that real
  dependencies cannot reliably produce (blocked pipes, stalled children, malformed
  frames). These are integration tests, not evidence of successful real journeys.
- Every new expensive test must state the risk covered and why a cheaper layer
  cannot prove it. Keep tests with unique boundary value even when slow. Do not
  shorten necessary correctness/recovery timeouts to meet a timing aspiration.
- Classify each test by component, category, requirements and coverage purpose in
  the verification inventory. Location and ignored status do not define category.
  Fixture setup alone does not make a test an integration test. Preserve certainty,
  provenance, source ownership and failure-recovery assertions.
- Before removing an expensive test, map its assertions and defect scenario to
  retained coverage and demonstrate that the replacement detects the failure.

Run category scripts from the workspace root:

```sh
./verify-unit.sh --component core
./verify-integration.sh --component engine
./verify-acceptance.sh --component mcp
./verify-performance.sh --component ui
```

Category scripts accept `--component`; `verify-ui.sh`, `verify-core.sh`,
`verify-engine.sh`, `verify-mcp.sh` and `verify-tooling.sh` accept `--suite` and
otherwise run unit plus integration checks. Use their `--help` and `--dry-run` for
selection, exact stages and prerequisites. During iteration run affected unit
and real-boundary suites; shared contract changes also require consumer checks.

`./verify-light.sh` and `./verify-full.sh` compose those same category execution
paths with static, build, quality and packaging stages. Light is for small,
low-risk changes. Full is mandatory for substantial/cross-component changes,
analyzer, dependency, bootstrap or packaging changes, uncertain impact and release
candidates. Full adds toolchain-backed integration, acceptance, performance and
packaging checks; do not rerun ordinary tests in debug and release without a
recorded release-specific reason. Report skipped/unavailable checks as not run.
CodeScene requirements above remain mandatory and do not replace behavioral tests.

Documentation/descriptive metadata changes require content, path, syntax and diff
checks only. Do not run application builds or runtime suites for those changes.
Executable/dependency/bootstrap/packaging changes require applicable runtime checks.

Every category owns an isolated build workspace and cleans its Rust outputs and
temporary fixtures/profiling copies before returning, including on failure,
timeout or catchable interruption. Stop and reap owned processes first; prove
ownership and inactivity before deletion. Never remove another run's artifacts.
Preserve downloaded Cargo caches, logs, reports, acceptance evidence, current
packages and one package rollback per app name. Cleanup failure or uncertainty
fails verification. Reclaim abandoned workspaces only after proving inactivity.
`--keep-artifacts` is an explicit diagnostic exception: report paths and sizes.
Report compilation, setup, test execution and cleanup timing separately; do not
hide rebuilding cost or weaken cleanup for apparent speed.

Run full outside the macOS sandbox so Metal and descendant cleanup are available.
Benchmarks use an isolated copied profiling workspace, never the live store.
Keep operational hang timeouts separate from performance aspirations. See
[validation prerequisites](<.backlog/docs/testing/doc-21 - Verification-prerequisites-and-operations.md#validation>). Both composite scripts
record stages under `target/verification/`, fail nonzero on failures/unavailable
prerequisites, and mark later stages not run. Use `--base <ref>` for comparisons.

Format only touched project-owned Rust files, for example
`rustfmt --edition 2024 --config skip_children=true path/to/changed.rs`.
Never format recursively into vendor sources. Do not run debug-profile
`--include-ignored`: canvas benchmarks require release mode. Perform applicable
native visual acceptance for substantial UI changes and report limitations honestly.
