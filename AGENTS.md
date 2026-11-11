
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

- [codegraph-core](crates/codegraph-core/agent.md)
- [codegraph-engine](crates/codegraph-engine/agent.md)

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
| [`codegraph-core`](crates/codegraph-core/agent.md) | Shared contracts, graph/flow operations, canonical artifacts, discovery and storage support |
| [`codegraph-engine`](crates/codegraph-engine/agent.md) | Standalone CLI, analysis lifecycle, build preparation and compiler-backed language adapters |
| `tools/` | Analyzer resources, bootstrap, acceptance helpers and packaging |
| `fixtures/`, `tests/`, `evidence/` | Reproducible inputs, integration checks and measured verification records |
| `vendor/`, `patches/` | Upstream checkouts and the recorded patches applied to them |

Keep production dependencies flowing from desktop and engine into core. Core
must not depend on either consumer. The desktop invokes the engine executable;
do not introduce production calls into its analyzer library. Engine dependencies
used by desktop acceptance tests are distinct from production coupling.
Use [the engine contract](docs/standalone-engine.md) for artifact, protocol and
language-extension behavior.

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

## Verification

Choose checks for the behavior changed. Run focused tests while iterating and
add regressions that assert observable results. Use real compiler/build fixtures
for analyzer behavior and narrow unit tests for isolated graph or geometry logic.
For documentation-only edits, verify paths, commands and the diff.

Run from the workspace root:

```sh
cargo check --locked -p codegraph --all-targets
cargo test --locked -p codegraph --all-targets
cargo clippy --workspace --all-targets --no-deps -- -D warnings
cargo test --workspace --all-targets -- --include-ignored
```

The final two commands are the full Rust validation documented in
[README.md](README.md); ignored acceptance tests need their actual toolchains.
Format only touched project-owned Rust files, for example
`rustfmt --edition 2024 --config skip_children=true path/to/changed.rs`.
Do not run formatting recursively into vendored sources. For bootstrap,
packaging, analyzer or UI changes, run the corresponding README acceptance and
packaging checks as well. Report unavailable prerequisites and pre-existing
failures precisely; never describe an unrun check as passing.
