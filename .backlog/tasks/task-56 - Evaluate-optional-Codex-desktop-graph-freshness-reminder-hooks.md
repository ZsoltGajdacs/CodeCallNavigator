---
id: TASK-56
title: Evaluate optional Codex desktop graph freshness reminder hooks
status: To Do
assignee: []
created_date: '2026-11-12T18:50:00Z'
updated_date: '2026-11-13T18:41:00Z'
labels: []
dependencies:
  - TASK-54
references:
  - TASK-51
documentation:
  - docs/agent-integrations.md
  - evidence/agent-integrations/hook-spikes.md
type: spike
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Optional experimental spike, dependent on the Codex skill package. Target manually installed desktop plugin hooks only per current packaging docs; reverify official surface restrictions first. In isolation, inspect PostToolUse apply_patch input for explicit project files, ignore unknown shapes/shell/MCP calls, and emit at most a 512-character advisory reminder per turn with session deduplication metadata only. No graph injection/cache/refresh, blocking or argument rewrite. Validate exact-definition trust, changed-definition re-review, disabled/untrusted hooks, repeats, failures, timeout and removal. Compare editing with and without reminders; record host version, event delivery, usefulness and overhead. Keep CLI plugin-hook support unclaimed unless separately proven. Full verification/CodeScene for retained executable prototype.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Normal desktop hook trust, changed-definition re-review, disable/removal and editing comparisons have evidence or precise unavailable validation records.
- [ ] #2 A release recommendation is evidence-backed and opt-in; public plugin hook distribution and unvalidated surface parity are not claimed.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Recheck Codex desktop plugin hook distribution and trust constraints.
2. Prototype only isolated manual-desktop PostToolUse reminders using the task bounded policy.
3. Validate definition-hash trust/re-review, disablement, repeats, failures, timeout and removal under ordinary trust.
4. Compare edit journeys with and without reminders, report overhead and usefulness and recommend or reject release; run applicable full verification and CodeScene.

5. Record current official schema and host versions; validate desktop support separately from CLI and record precise unavailable cases. Use disposable prototypes and keep production packages hook-free pending measured benefit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Started 2026-11-13, assigned @codex. Dependencies complete. Planner confirmed unavailable desktop validation may be recorded under AC1, but no desktop or CLI parity may be inferred from handler probes.

Host feasibility: codex-cli 0.168.0 is installed. Current official hook docs describe tool_input.command and turn_id for apply_patch, exact-definition trust and changed-definition review; hook-dependent packaging remains manual desktop distribution. Native desktop control is disabled in available CUA; no supported isolated desktop-profile launch was found. Desktop trust, delivery, re-review, disable/removal, and model comparison remain unrun.

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

Codex-specific findings and remaining work:
- Installed CLI was codex-cli 0.168.0; no desktop version was measured. CLI success cannot prove desktop plugin support.
- Official https://learn.chatgpt.com/docs/hooks documents canonical tool_name=apply_patch, tool_input.command and turn_id. Edit/Write are matcher aliases, not a reason to assume a different payload. Advisory additionalContext is supported; exact-definition trust is required and changed definitions require re-review.
- https://developers.openai.com/plugins/build/plugins and https://developers.openai.com/plugins/guides/submit-claude-plugin describe manual desktop distribution for hook-dependent plugins. Do not claim public-directory hook distribution or CLI plugin-hook parity.
- Available computer-use APIs permit browser surfaces only; native desktop control was disabled. No documented isolated desktop profile/second-instance launch was established. This prevented observing native installation/trust UI; it does not establish that Codex desktop lacks hooks.
- Resume with a controlled isolated manual-desktop plugin and record actual desktop host version, hook payload/event delivery and turn identity. Validate exact-definition trust, changed-definition re-review, disabled/untrusted hooks, repeats, failures, timeout and removal before comparing model behavior.
- AC1 permits precise unavailable validation records; that permission does not prove benefit or desktop behavior. Keep the release recommendation provisional until actual host comparisons support a conclusion.
<!-- SECTION:NOTES:END -->
