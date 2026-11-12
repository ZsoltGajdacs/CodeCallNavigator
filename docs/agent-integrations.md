# Agent companion integration research

Research date: 2026-11-12. TASK-51 (promoted from DRAFT-2). This is an
implementation handoff, not an installed plugin. No public CLI/MCP changes are
required for the recommended first release.

## Recommendation

Deliver one shared graph-context skill, packaged independently for Claude Code
and Codex, using the separately installed [CLI-backed MCP](mcp.md). Start with
explicit invocation; allow narrowly described implicit skill selection after
users opt in by installing it. Do not ship graph-injecting hooks in the initial
packages. Skills can choose symbols and request focused MCP results without
intercepting every source read or trying to parse arbitrary shell commands.

A hook is a possible later convenience, not a substitute for this workflow.
Both hosts document context-bearing hooks, but installation, event matching,
trust and supported surfaces differ. The recommended optional hook experiment
only reminds the agent to recheck graph freshness after an edit. It must not
analyze, block edits, mutate tool arguments, or inject graph sections on every
read. Continuous refresh remains DRAFT-3.

## Dated capability comparison

The following is **documented support**, unless the Local evidence column says
otherwise. Documentation is mutable; recheck it when building the packages.

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

## Approaches evaluated

| Approach | Claude Code | Codex | Decision |
| --- | --- | --- | --- |
| Explicit skill | Namespaced slash workflow; user chooses when | Named skill mention; user chooses when | First acceptance path in both packages; simplest predictable experience |
| Skill-guided MCP | Narrow description can trigger structural investigation | Narrow description can trigger structural investigation | Recommended default after installation; model choice is not a guarantee of invocation |
| Hook-assisted context | Can add contextual reminders adjacent to tool events | Can add context on supported local tool events; distinct trust/distribution rules | Separate opt-in experimental follow-ups, dependent on useful skill acceptance |

A shared manifest does not establish equivalent host behavior. Avoid a single
unvalidated cross-host hook config, remote-host access to local stdio, global
hooks, command rewriting, and automatic refresh as fallbacks.

## Workflow contract and journeys

Select a configured server bound to the intended project before querying. Never
infer a project from a basename or switch roots inside a running MCP server.
Use explicit linked roots only. One project per server identity; use distinct
names for simultaneously configured projects. If identity is unclear, ask the
user to choose before issuing queries. Project access remains CLI-owned.

Each journey begins with `graph_status`. Bind subsequent analyzed queries to its
`generation` with `expected_generation`. Preserve freshness, certainty,
provenance, diagnostics and truncation; a possible call is not observed execution.

| Journey / trigger | Requests and scope | Result / fallback |
| --- | --- | --- |
| Explore a named symbol | `find_symbols` category + text, first 20 hits; `symbol_details` on candidate IDs | Present file/container/build distinctions for ambiguous hits; never silently choose the first result. No hit: ordinary source search |
| Explain callers/callees | Resolve symbol, then `graph_section` neighborhood with Incoming or Outgoing, depth 1, 100 nodes / 300 edges | Say which direction/filter was used. Truncation means incomplete; expand only for a concrete follow-up |
| Explain function flow | Resolve method, `function_flow`; `symbol_source` analyzed only if needed | Preserve unsupported/unavailable result; use source inspection with an explicit limitation, never fabricate execution |
| Before editing | Resolve target and bounded Incoming neighborhood, plus details for affected callers | Summarize possible impact and tests to inspect; graph relationships are structural evidence |
| After editing | `graph_status` again; stale graph gets a visible warning | Do not combine old analyzed flow with live source as one current answer. Offer explicit `refresh_graph`, then discard old IDs/context and resolve again |

Context defaults are workflow policy, not new MCP validation rules: search pages
of 20, depth 1 and 100/300 selection limits, and at most 8 KiB of rendered context
per request. Summaries must keep generation/freshness and omission markers.
`function_flow` and source tools have no caller size limit; inspect their returned
metadata and summarize to this budget rather than pretending they accept graph
limits. Never trim structured results so they look complete.

Deduplicate within the conversation by configured project/server, generation,
symbol or selection, direction/filter and requested bounds. Reuse an existing
answer only after status establishes the same fresh generation; after edits,
staleness, refresh or generation mismatch, invalidate reuse. Do not add an MCP
cache. A generation mismatch requires status and fresh resolution, not a silent
retry that combines generations. Repeated requests should reference the earlier
summary rather than repeat it unless scope changes.

Unverifiable freshness: report that current-source agreement is unknown; do not label
the graph current or reuse it as verified fresh context. The fixture initially
reported an unmapped-input reason, which the MCP preserved.

Missing graph: explain and offer explicit discovery/refresh. Missing or
incompatible CLI, disabled MCP, tool failure or unsupported host: explain once,
continue normal source exploration, and label the missing structural evidence.
Never turn a transport failure into an empty successful graph. Hook failure must
be advisory and must not prevent ordinary work. No background analysis.

## Installation and removal design

Prerequisite: separately install matching engine and MCP binaries, with engine
analyzer resources intact. Substitute actual absolute paths below. A plugin
should package workflow instructions; initially configure the local server
separately so package paths and project addresses are not hard-coded together.
These are proposed user setup instructions, not changes made by this spike.

Codex, trusted project `.codex/config.toml`:

```toml
[mcp_servers.codecallnavigator]
command = "/absolute/path/codecallnavigator-mcp"
args = ["--engine", "/absolute/path/codecallnavigator-engine", "--project", "/absolute/path/project"]
```

Claude Code, personal per-project server:

```sh
claude mcp add --scope local codecallnavigator -- /absolute/path/codecallnavigator-mcp --engine /absolute/path/codecallnavigator-engine --project /absolute/path/project
```

Alternatively share Claude project configuration in `.mcp.json` with
`mcpServers.codecallnavigator.command` and `args` matching the same command/array.
Do not commit machine-specific absolute paths as a portable team configuration.
User-scope Codex setup can use `codex mcp add codecallnavigator -- ...` with the
same executable and arguments; this still binds one explicit project.

Install the chosen host skill package, inspect normal host approvals and invoke
it explicitly first. Verify it reports missing/stale state before granting any
refresh request. Do not auto-approve refresh on installation. Hooks stay absent
from the initial package and need a separate explicit installation/trust step.

Disable Codex MCP with `enabled = false`, or remove its exact config table;
`codex mcp remove codecallnavigator` removes a user-scope registration. Remove a
project table separately. Claude removal uses
`claude mcp remove --scope local codecallnavigator` (choose project/user if that
was the installed scope). Uninstall the host plugin through its plugin manager;
remove separately registered MCP configuration as a separate step. Neither action
should delete analysis artifacts, engine binaries or user source. Restart/reload
as required by the host, then verify tools and skill disappear.

Proposed repository package layout for follow-up work:

```text
integrations/shared/graph-context.md              # canonical workflow
integrations/claude-code/.claude-plugin/plugin.json
integrations/claude-code/skills/graph-context/SKILL.md
integrations/codex/plugin.json
integrations/codex/skills/graph-context/SKILL.md
```

Host wrappers include the shared workflow at packaging time with a drift check;
no symlink outside installed plugin roots. Package no graph reader, algorithm,
source resolver or refresh daemon. Hooks, if later validated, remain separate
optional artifacts and call the installed CLI rather than importing app crates.

## Evidence, remaining validation and follow-ups

[Evidence and reproduction](../evidence/agent-integrations/README.md) includes
real stdio transcripts, timings and isolated host configuration checks. These
verify the backend contract, not end-to-end model behavior. Host workflow
invocation, interactive trust/disabled-hook behavior, full plugin installation
and removal, and repeated-context behavior in actual conversations remain
**unverified** and are acceptance gates on the follow-up packages.

No demonstrated CLI/MCP gap requires an extension. Existing tool envelopes carry
the metadata required for the proposed skill. Initial measured graph queries
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
