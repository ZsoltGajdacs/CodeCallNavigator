
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
