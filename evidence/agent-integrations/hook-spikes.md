# Optional graph freshness hook spikes: TASK-55 and TASK-56

2026-11-13, macOS. **Initial feasibility evidence, not completed host acceptance.**
Observed CLI versions: Claude Code 2.1.306 and codex-cli 0.168.0. The latter
is not a desktop version measurement. No production hook was installed and no
integration package was changed.

## Current documentation and prototype boundary

Official sources opened on this date:

- [Claude hook reference](https://code.claude.com/docs/en/hooks): successful
  `PostToolUse` edits expose explicit `tool_input.file_path`; JSON can add
  advisory context. Current common inputs document `prompt_id`. Its presence
  in the installed Claude version remains unverified.
- [Codex hook reference](https://learn.chatgpt.com/docs/hooks): `apply_patch`
  uses `tool_input.command`, with `turn_id`; post-tool JSON can add context.
  Non-managed hooks require review of the exact definition; changed definitions
  require review again.
- [Codex plugin packaging](https://developers.openai.com/plugins/build/plugins)
  and [Claude plugin submission guidance](https://developers.openai.com/plugins/guides/submit-claude-plugin):
  hook-dependent distribution targets manual desktop installation. This report
  makes no public hook-distribution or CLI plugin-hook parity claim.

The disposable Python prototype took host, explicit project root and metadata
folder as arguments. It accepted Claude `Edit`/`Write` absolute paths or Codex
`apply_patch` patch headers, checked canonical containment of all named paths
including rename destinations, and ignored shell, MCP and unknown inputs.
It emitted only `hookSpecificOutput` with `hookEventName` and `additionalContext`:

> Before relying on graph context after this edit, check graph_status and report freshness; refresh only with explicit user authorization.

A file lock serialized deduplication by session and turn (`prompt_id` for Claude,
`turn_id` for Codex). Missing identities produced no reminder. State contained
hashed identity metadata only; there was no graph read, cache, query, refresh,
blocking decision or argument rewrite. Exercised malformed-input, metadata-I/O and path-resolution failures produced
no advisory and exit zero. This was a feasibility handler, not a complete patch parser or released
hook artifact. It was not loaded by either host. The scanner deliberately supports only a
conservative subset of patch structure, so unrecognized valid edit forms can
miss reminders; no broad patch-parser coverage is claimed.

## Direct subprocess observations

Python 3.9.6. An empty handler first failed the observable explicit-edit assertion.
After implementation, **41 checks passed** against real subprocess stdout,
exit status and JSON shape:

| Cases | Observed result |
| --- | --- |
| Explicit edit / Claude Write / next turn | Advisory emitted; text below 512 characters |
| Repeated edit in same turn | No second advisory |
| Eight concurrent duplicate Codex events | Exactly one advisory |
| Shell, MCP, unknown tools, non-post event | No output |
| Missing session/turn/cwd, malformed JSON, non-object/unknown input | No output |
| Outside paths, traversal, symlink escape, outside rename, mixed inside/outside patch | No output |
| Unknown patch marker, header-only Add patch or non-patch command | No output |
| Symlink cycle | No output, exit zero |
| Supported update with `*** End of File` | Advisory emitted |
| Metadata path is a file instead of a directory | No output, exit zero |

Thirty sequential eligible Codex-handler process launches measured with
`perf_counter_ns` had median **33.668 ms**, minimum 28.873 ms, maximum 44.077 ms.
These include Python startup and metadata I/O; they are **not** host delivery
latency, model overhead or evidence of usefulness. Timeout and failed-hook host
behavior were not exercised by these bounded subprocess checks.

Temporary working artifacts were created under
`/private/tmp/ccnavigator-hook-spike.XNEobg` for review and removed after independent testing.
Final handler SHA-256:
`2ee9c752fece04d3fc0f3705a9247243934ebce58f947004425de691e80686d8`.
No executable prototype is retained in this repository. The results above are
a record of a disposable experiment; this report does not claim the removed
prototype can be reproduced byte-for-byte from retained source.

## Host validation limits and next steps

| Required observation | TASK-55 Claude | TASK-56 Codex desktop |
| --- | --- | --- |
| Normal-trust reminder delivery | Not tested | Unavailable in this session |
| Untrusted/disabled hook | Not tested | Unavailable in this session |
| Changed-definition review | Not applicable as Codex-specific criterion | Unavailable in this session |
| Failed/timed-out hook; removal | Not tested | Unavailable in this session |
| Same editing journey with/without reminder | Not tested | Unavailable in this session |
| Model freshness checks and delivery timing | Not measured | Not measured |

The normal Claude authentication probe reported `loggedIn: true`; the isolated
`CLAUDE_CONFIG_DIR=/private/tmp/ccn-hook-host-5ce0kjul` probe reported
`loggedIn: false`, `authMethod: none`, exit 1. No credentials were copied, login
started or global configuration changed. An authenticated isolated interactive
Claude session is therefore still needed. Print mode cannot establish ordinary
interactive project trust. Missing `prompt_id` on the installed version would
require a separately validated turn-boundary design, not session-only suppression.

Native desktop computer control is disabled in the available UI tool; it supports
browser surfaces only. No documented independent desktop profile/instance
isolation was established. The current Codex app context does not expose a
verified version or plugin trust UI. These limits prevent observing desktop
installation, definition review, delivery, disablement and removal here; CLI or
direct-handler success cannot substitute for them.

For each host, continue in an isolated authenticated fixture with the existing
native graph-context skill and project-bound MCP server. Establish graph status,
then ask for a small `entry.ts` edit followed by an impact explanation, without
requesting refresh. Repeat with and without the reminder using identical
starting graph/source bytes. Record actual edit payload, hook start/completion,
model-visible advisory, subsequent `graph_status`, freshness and any refresh
attempt. Repeat edits in one turn and a new turn. Then test disabled/untrusted,
failure, timeout and removal; Codex additionally needs definition modification
and re-review. Keep normal host trust and approvals throughout.

**Recommendation: do not release either hook yet.** Handler feasibility and
small process cost do not demonstrate model benefit. TASK-55 still requires its
real comparison and normal-trust negative cases; TASK-56 may record unavailable
validation, but desktop behavior and usefulness remain unproven. Keep both
production packages hook-free.
