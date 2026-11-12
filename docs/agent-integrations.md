# Agent companion integration research

Research date: 2026-11-12. TASK-51 was promoted from DRAFT-2. The comparison
below is historical research. The latest user decision supersedes the interim
terminal-only plugin route: plugins now use native MCP tools, while the public
`ccnavigator` terminal command remains independently available on PATH.

## Current delivery decision

Deliver a shared graph-context skill packaged independently for Claude Code and
Codex. It calls the separately installed, project-bound `codecallnavigator-mcp`
server through each host's native MCP tools. Configure the server with
`--engine ccnavigator`: both programs are command names on PATH, with no binary
paths required. The project root remains an explicit server configuration value.

The [canonical workflow](../integrations/shared/graph-context.md) defines typed
tool calls and behavior; [acceptance journeys](../integrations/shared/journeys.md)
separate native-MCP compiler assertions from real host conversations. The
[CLI-backed MCP contract](mcp.md) owns transport and the unchanged CLI query
version 1. The standalone CLI remains directly usable, but is not a silent graph
fallback for the plugins.

Start with explicit invocation; allow narrow description-based implicit
selection after opt-in installation. Plugins contain workflow instructions only:
no hooks, bundled server binaries or MCP implementation, graph reader,
algorithms, persistent graph cache or background analysis. Ordinary queries
remain read-only. Only an explicit user refresh request/approval permits
`refresh_graph`. Continuous refresh stays with DRAFT-3.

## Dated capability comparison

This matrix records **historical documented support and TASK-51 probes**, not
the current package setup requirements. MCP and hook capabilities were assessed
as alternatives; the delivered skills now use native MCP over the installed CLI. Documentation is
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

Install matching engine and MCP adapter distributions with `ccnavigator` and
`codecallnavigator-mcp` on PATH and analyzer resources intact. Plugin installation
and project-bound MCP registration are separate user steps; the package must
not guess the project from its install directory or auto-register a server.

For Claude Code, run this **from the intended project**:

```sh
claude mcp add --scope local codecallnavigator -- codecallnavigator-mcp --engine ccnavigator --project "$PWD"
```

For Codex, use the intended trusted project's `.codex/config.toml`:

```toml
[mcp_servers.codecallnavigator]
command = "codecallnavigator-mcp"
args = ["--engine", "ccnavigator", "--project", "/absolute/path/project"]
tool_timeout_sec = 1800
```

A fixed-project user-scope Codex registration can alternatively use
`codex mcp add codecallnavigator -- codecallnavigator-mcp --engine ccnavigator
--project "$PWD"` from that project. Keep distinct server names for different
projects. Linked roots are explicit adapter `--root NAME=PATH` arguments; never
infer them. Executable values remain PATH names; the explicit project/linked
root paths are data addresses, not executable-path configuration.

Establish the selected server-to-project mapping from trusted host configuration
or an explicit user binding before querying. `graph_status` does not expose the
configured project identity. Workspace path, tool names and generation digest
cannot prove the mapping; ask if unclear. Keep normal host trust/tool approvals
and do not blanket-approve refresh at installation.

Both packages contain the exact canonical body with host-specific metadata.
Packaging detects drift, produces self-contained archives, rejects unexpected
source files/symlinks and includes no server wiring or binaries. Host guides:
[Claude Code](../integrations/claude-code/README.md) and
[Codex CLI](../integrations/codex/README.md). Layout:

```text
integrations/shared/graph-context.md
integrations/shared/journeys.md
integrations/claude-code/.claude-plugin/plugin.json
integrations/claude-code/skills/graph-context/SKILL.md
integrations/codex/plugin.json
integrations/codex/skills/graph-context/SKILL.md
```

Every investigation starts with native `graph_status`; later analyzed tools bind
its generation. Preserve ambiguity, relationship certainty/provenance,
freshness and truncation. Defaults remain 20 search hits, depth 1 selections at
100 nodes/300 edges and 8 KiB rendered context. Repeated unchanged verified-match
context gets a status-only check and reference to the prior summary. Stale and
unverifiable graphs are never presented as verified current evidence.

`discover_builds` is read-only; `refresh_graph` requires an explicit user
refresh request or approval. No shell analysis or direct terminal graph fallback.
Missing/disabled MCP or CLI
errors are explained before source-only fallback, which excludes
`.codecallnavigator/`. Removing a plugin does not remove its separately
configured server: remove the selected project table or personal registration
at its actual scope, then verify skill/tools disappear in a new session.
Preserve independent binaries, source and saved graph artifacts.

## Evidence, remaining validation and follow-ups

[Research evidence](../evidence/agent-integrations/README.md) includes historical
native MCP/compiler probes. The new `tools/agent_mcp_journeys.py` supplies
reproducible current SDK-stdio/compiler acceptance. `tools/agent_journeys.py`
remains standalone CLI compatibility coverage, not the current plugin route.
Neither backend test proves model selection or context behavior.

[Current delivery evidence](../evidence/agent-integrations/delivery-mcp.md) links
the native MCP compiler transcript, measured host conversations and package
validation. The [earlier TASK-52 evidence](../evidence/agent-integrations/delivery-task52.md)
records the interim direct-CLI route and remains historical; it does not
validate current MCP package behavior. Desktop UI and optional hook behavior
remain separately unverified.

No graph query contract or MCP implementation extension is needed. Existing tool
envelopes carry the necessary metadata. Tiny-fixture measurements do not prove
large-project latency, token cost, model accuracy or hook overhead.

TASK-52 owns shared workflow and reproducible acceptance; TASK-53 and TASK-54
own Claude/Codex packages. TASK-55/56 are optional hook experiments and remain
outside this delivery. Minimum versions, other operating systems and desktop
surface support require their own evidence; default package claims target local
macOS CLI hosts with no hooks.
