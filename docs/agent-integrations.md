# Agent companion integration research

Research date: 2026-11-12. TASK-51 was promoted from DRAFT-2. The research
comparison below is retained as historical evidence; the subsequent user decision
for TASK-52 through TASK-54 supersedes its MCP setup recommendation.

## Current delivery decision

Deliver one shared graph-context skill packaged independently for Claude Code
and Codex. Both invoke the public `ccnavigator` command directly in the terminal
through PATH. Users do not need an MCP server, MCP registration, or a configured
binary path. The CLI owns query semantics and analyzed artifacts; the skills
supply the investigation workflow. The independently installed MCP remains a
separate supported client, not a dependency or fallback for these packages.

The [canonical workflow](../integrations/shared/graph-context.md) defines the
actual query envelopes and behavior. [Acceptance journeys](../integrations/shared/journeys.md)
separate real CLI/compiler assertions from host conversation acceptance. The
public JSON contract remains [CLI query version 1](cli-queries.md).

Start with explicit invocation and allow narrowly described implicit skill
selection after opt-in installation. No hooks, graph reader, algorithms,
persistent graph cache or background refresh are included. Ordinary queries are
read-only; `ccnavigator analyze` requires an explicit user refresh request or
approval. Possible relationships are source-derived evidence, not observed
runtime execution. Continuous refresh remains DRAFT-3.

## Dated capability comparison

This matrix records **historical documented support and TASK-51 probes**, not
the current package setup requirements. MCP and hook capabilities were assessed
as alternatives; the delivered skills use terminal CLI access. Documentation is
mutable; recheck relevant host packaging support when building packages.

| Capability | Claude Code | Codex | Local evidence / limit |
| --- | --- | --- | --- |
| Plugin package | `.claude-plugin/plugin.json`; skills, MCP and hooks [C1] | Root `plugin.json`, `skills/`, optional `mcp.json`; OpenAI-specific overlay under `extensions.com.openai` [O1] | Claude minimal manifest + skill validation passed; neither full plugin installed |
| Explicit / implicit skills | Slash invocation; description-based selection; `disable-model-invocation` for manual-only workflows [C2] | `$` mention or `/skills` in CLI; description-based implicit selection [O2] | Workflow invocation by a model unverified |
| Local stdio MCP | Supported; command plus argument array [C3] | Supported; command plus argument array [O3] | Register/read/remove passed in isolated Codex configuration; register/remove passed in isolated Claude configuration |
| Configuration scope | Local per-project personal, project `.mcp.json`, or user; local is default [C3] | User `config.toml` or trusted project `.codex/config.toml` [O3] | No real user/project configuration modified |
| Relevant hook events | SessionStart, UserPromptSubmit, PreToolUse, PostToolUse; JSON additionalContext [C4] | SessionStart, UserPromptSubmit, PreToolUse, PostToolUse; JSON additionalContext [O4] | Documented, not executed |
| Tool coverage | Match named tool events; inspect input rather than assuming all edits use one tool [C4] | Shell/unified exec aliases Bash; apply_patch aliases Edit/Write; local MCP tools; hosted tools may bypass hooks [O4] | Arbitrary shell source reads cannot be reliably mapped to a symbol |
| Trust / opt in | Workspace/project MCP approvals and hook settings; use normal interactive trust [C3,C4] | Non-managed hooks require trust of exact definition; changed definitions skipped pending review [O4] | Interactive trust, disabled hooks and upgrade re-trust unverified |
| Hook distribution | Plugin hooks documented [C1,C4] | Packaging guide restricts bundled lifecycle hooks to manually installed desktop plugins; not public directory eligible [O1] | Do not infer CLI plugin-hook parity from standalone CLI hook support |
| Surface / platform | Target local Claude Code first; no claim for claude.ai or Cowork [C1] | Target local Codex CLI/desktop; IDE and cloud require independent acceptance [O1,O3] | Only macOS arm64 checked; Windows/Linux not validated; local binaries/resources must exist in host environment |
| Version | Installed 2.1.306 | Installed codex-cli 0.166.0 | These are observed versions, not minimum requirements; no minimum version established by this spike |

Official sources, opened on the research date:

- [C1: Claude Code plugins](https://code.claude.com/docs/en/plugins)
- [C2: Claude Code skills](https://code.claude.com/docs/en/skills)
- [C3: Claude Code MCP](https://code.claude.com/docs/en/mcp)
- [C4: Claude Code hooks](https://code.claude.com/docs/en/hooks)
- [O1: Codex plugin packaging](https://developers.openai.com/plugins/build/plugins)
- [O2: Build skills](https://learn.chatgpt.com/docs/build-skills)
- [O3: Codex MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)
- [O4: Codex hooks](https://learn.chatgpt.com/docs/hooks)

## Current workflow and installation

Install the matching engine distribution so `ccnavigator` is on the terminal
PATH. Run `ccnavigator capabilities` to confirm query contract version 1 and
generation binding. Preserve analyzer resources alongside the installed engine.
The skill receives an explicit project root in every query, independent of its
own plugin installation directory; no server selection or MCP configuration is
needed. For linked roots, use only explicitly configured root mappings.

Both host packages contain the same canonical workflow body with only host
metadata differing. Packaging copies the shared body into each installed skill;
a drift check rejects divergence. Installed skills must not depend on symlinks
or references outside their plugin root. Proposed layout:

```text
integrations/shared/graph-context.md
integrations/shared/journeys.md
integrations/claude-code/.claude-plugin/plugin.json
integrations/claude-code/skills/graph-context/SKILL.md
integrations/codex/plugin.json
integrations/codex/skills/graph-context/SKILL.md
```

Install the selected skill package through the host's normal plugin path and
invoke it explicitly first. Keep normal host terminal approvals and trust.
Explicit `ccnavigator analyze --project PROJECT TARGET...` is the only analysis
path; neither invocation nor installation grants blanket refresh approval.
Disabling terminal access or removing `ccnavigator` from PATH produces an
honest unavailable-CLI fallback to normal source exploration, not an MCP
fallback, auto-install, or hidden analysis.

Uninstall through the host's plugin manager and restart/reload as required. The
skill and its instructions must disappear from a new conversation. Do not
remove the independently installed CLI, project source or published graph
artifacts. There is no separate MCP registration to clean up for this delivery.
Host-specific installation and removal commands belong in each package guide.

Every investigation starts with status and binds subsequent queries to its
generation. The shared workflow defines ambiguity, caller/callee direction,
function flow, before/after edit handling, the 20-hit/100-node/300-edge defaults,
8 KiB rendered context budget and conversation-only repeat suppression. Preserve
freshness as `match`, `stale` or `unverifiable`; no fresh/current claim is allowed
for unverifiable inputs. Unsupported flow and errors remain explicit.

## Evidence, remaining validation and follow-ups

[Evidence and reproduction](../evidence/agent-integrations/README.md) includes
historical MCP stdio transcripts, timings and isolated host configuration checks.
Current CLI baseline acceptance is reproducible with tools/agent_journeys.py.
Backend assertions do not prove end-to-end model behavior. The
[TASK-52 delivery evidence](../evidence/agent-integrations/delivery-task52.md)
records actual Claude and Codex shared-skill conversations, including ambiguity,
unsupported flow and status-only repeat suppression. Full host-package install,
implicit selection and removal remain acceptance gates on TASK-53/54. Desktop UI
and hook behavior are not claimed by these CLI conversations.

No query-contract extension is required. Existing CLI envelopes carry the
metadata required for the skill; the public command name is `ccnavigator`. Initial measured graph queries
were milliseconds on a tiny warm compiler fixture; this does not establish
large-project latency, token usage, model accuracy or hook overhead.

Follow-ups are TASK-52 (shared workflow and reproducible journey acceptance),
TASK-53 (Claude packaging), TASK-54 (Codex packaging), TASK-55 (optional Claude
reminder-hook experiment), and TASK-56 (optional Codex desktop reminder-hook
experiment). Packaging depends
on the shared workflow. Hook experiments depend on their respective package and
must demonstrate usefulness and bounded overhead before recommending release.

Open constraints are validation gates, not hidden implementation choices:
minimum host versions, non-macOS support, model selection quality, large graph
latency, and whether reminders improve behavior enough to justify hooks. Default
release is local macOS, no hooks. Unsupported surfaces get explicit fallback;
expanding platform claims requires evidence.
