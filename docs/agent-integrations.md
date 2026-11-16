# Agent companion integrations

## Shared workflow

The shared graph-context skill is packaged independently for Claude Code and
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

Use explicit invocation or narrow description-based implicit
selection after opt-in installation. Plugins contain workflow instructions only:
no hooks, bundled server binaries or MCP implementation, graph reader,
algorithms, persistent graph cache or background analysis. Ordinary queries
remain read-only. Only an explicit user refresh request/approval permits
`refresh_graph`. Automatic refresh is a separate explicit opt-in; see [freshness](graph-freshness.md).

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

Both host packages use the same workflow. See their installation guides:
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
