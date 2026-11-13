---
id: TASK-55
title: Evaluate optional Claude Code graph freshness reminder hooks
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-13T18:41:00Z'
labels: []
dependencies:
  - TASK-53
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
  - evidence/agent-integrations/hook-spikes.md
type: spike
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Optional experimental spike, dependent on the Claude skill package. In an isolated configuration, implement a disposable PostToolUse reminder for explicit Edit/Write paths only; ignore unknown shell commands and MCP calls to prevent recursion. Emit at most a 512-character advisory reminder per turn for edited project files; do not block/rewrite tools or query/analyze graph automatically. Keep per-session deduplication metadata only, no graph cache. Validate real interactive hook trust, disablement, repeated edits, failures, removal and timeout. Compare the same editing journey with and without reminder, record delivery timing and model freshness checks, and recommend release only with measurable benefit. Run full verification/CodeScene if executable prototype is retained.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A reproducible comparison records usefulness and overhead plus disabled/untrusted/failing-hook behavior under normal Claude trust.
- [ ] #2 The report either recommends a separate opt-in artifact with evidence or rejects hooks; no production hook is installed.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Recheck Claude hook documentation and create an isolated optional reminder prototype with the bounded policy in the task description.
2. Validate ordinary trust, disabled/untrusted/failed hooks, repeat suppression, timeout and removal.
3. Compare identical edit journeys with and without hooks and record overhead and observable freshness-check behavior.
4. Recommend opt-in release only with demonstrated benefit; otherwise reject hooks. Apply full verification and CodeScene to retained source.

5. Record current official schema and host versions; use disposable prototypes outside integration packages, distinguish handler probes from normal-trust host evidence, and retain no executable prototype unless full verification is completed.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Started 2026-11-13, assigned @codex. Dependencies complete. Planner reviewed end state: AC1 requires a real Claude comparison and normal-trust negative cases; direct handler probes alone cannot close it.

Host feasibility: Claude 2.1.306 normal host authentication is available, but a disposable CLAUDE_CONFIG_DIR has no authentication even outside sandbox (exit 1). No credentials copied or global configuration changed. Packaged MCP exists at dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp. Authenticated isolated interactive comparison and normal-trust matrix remain unrun.

Initial spike evidence: evidence/agent-integrations/hook-spikes.md. Disposable bounded handler passed 41 implementation probes; independent tester reran core/regressions and verified concurrent same-turn suppression. Review found symlink-cycle and malformed patch issues; fixed and reverified, including valid EOF markers; final review has no remaining findings. All disposable prototype files removed; no production hooks installed. Documentation-only ./verify-light.sh passed (target/verification/light-xe5cd_al/results.json). Acceptance criteria remain unchecked: interactive trust and model usefulness comparisons are unrun.

2026-11-13 handoff — returned to To Do at user request because research is incomplete. No acceptance criteria were met by the feasibility work alone; keep all unchecked.

Disposable prototype findings:
- Explicit project root and separate metadata directory were required. Accept only explicit project edit paths; resolve canonical paths and check every source/rename destination stays within the root. Reject traversal, outside paths, symlink escapes, mixed inside/outside patches and unknown inputs. Ignore shell and MCP events to avoid recursion.
- Emit only hookSpecificOutput.hookEventName=PostToolUse and additionalContext, at most 512 characters. Final text: "Before relying on graph context after this edit, check graph_status and report freshness; refresh only with explicit user authorization." Never read/query/cache/refresh graphs, block tools or rewrite arguments. Tool input alone does not prove a successful edit, so do not assert that files changed.
- Deduplicate with a file lock and hashes of session and turn identities; retain session metadata only, no paths/source content. Missing turn identity must produce no reminder rather than silently changing the policy to once per session. Concurrent duplicate events emitted exactly once; a new turn emitted again.
- Initial review exposed an uncaught Python 3.9 pathlib.resolve RuntimeError for symlink cycles; catching it restored silent exit-zero behavior. Header-only malformed patches initially emitted misleading reminders; reject them. A valid "*** End of File" marker initially suppressed reminders; support was added and verified. The disposable scanner still supported only a conservative patch subset; unknown valid shapes can miss reminders.
- Final prototype passed 41 implementation checks; independent testing reran 33 core cases and 3 regressions plus concurrent suppression, valid EOF, malformed headers and symlink-loop checks. Covered invalid JSON/types, missing identities/cwd, unknown/non-post events, path containment and metadata I/O errors. Review ended with no remaining findings for this disposable scope.
- Python 3.9.6, macOS: 30 final Codex-handler subprocess launches had median 33.668 ms, min 28.873 ms, max 44.077 ms. These include Python startup and metadata I/O. They measure neither host delivery latency nor model overhead/usefulness. Independent measurements are consistent with tens of milliseconds but do not broaden that claim.
- No hook was loaded by either real host. No real trust, disabled/untrusted/failing/timed-out/removal behavior or with/without-reminder model journey was validated. Direct handler checks cannot establish any of those.
- All disposable executable prototypes/test copies were removed; no production package/configuration was changed. Final handler SHA-256 was 2ee9c752fece04d3fc0f3705a9247243934ebce58f947004425de691e80686d8. Retained report is evidence/agent-integrations/hook-spikes.md, not byte-for-byte reproducible source.
- Documentation-only ./verify-light.sh passed; no full runtime/CodeScene claim is made for a retained executable integration. Any executable prototype retained on resumption requires full verification and CodeScene per task scope.

Provisional conclusion: technically feasible bounded advisory handlers, with no demonstrated agent benefit. Do not release either hook yet; this is not a completed research conclusion.

Shared next experiment: use the existing native graph-context skill and an explicitly project-bound real MCP server. From identical saved graph/source bytes, compare baseline and reminder arms: establish graph_status, request a small entry.ts edit and an impact explanation without requesting refresh, record actual payload, hook start/completion, model-visible reminder, subsequent graph_status/freshness and any refresh attempt. Repeat edits within one turn and across turns, then test negative lifecycle cases. Preserve normal trust and approvals.

Claude-specific findings and remaining work:
- Installed Claude Code was 2.1.306. Official https://code.claude.com/docs/en/hooks documents Edit/Write tool_input.file_path and advisory JSON context. Current docs include prompt_id, but its presence in this installed version was never observed; newer docs may postdate the host.
- CLI supports session-local --plugin-dir, --settings, --setting-sources '', --strict-mcp-config, --include-hook-events and PTY interaction. Print mode treats the folder as trusted and cannot establish ordinary interactive workspace trust.
- A sandboxed auth-status probe appeared unauthenticated; normal execution confirmed existing account authentication. A fresh isolated CLAUDE_CONFIG_DIR still reported loggedIn=false/authMethod=none, exit 1, outside the sandbox. This is an isolation/authentication limitation, not proof that the normal account is logged out. No credentials were copied, login initiated or global configuration changed.
- ccnavigator is on PATH. codecallnavigator-mcp is not, but a packaged binary exists at dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp.
- Resume with an authenticated isolated fixture and interactive PTY under ordinary trust. Verify real event payload and turn identity first; if prompt_id is absent, design and validate a reliable turn boundary before comparisons.
- Complete normal interactive trust, disabled/untrusted hooks, repeated edits, failure, timeout and removal, then the identical baseline/reminder comparison and actual model freshness checks. AC1 requires real reproducible comparison and normal-trust negative evidence; unavailable records alone do not satisfy its wording.
<!-- SECTION:NOTES:END -->
