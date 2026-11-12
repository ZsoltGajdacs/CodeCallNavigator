# CodeCallNavigator for Codex CLI

This local-executor plugin teaches Codex to investigate symbols, callers/callees,
function flow and possible change impact through configured native MCP tools.
Install the engine and adapter separately so `ccnavigator` and
`codecallnavigator-mcp` are on PATH, with analyzer resources intact. The package
contains instructions, not binaries, hooks, MCP implementation, bundled server
wiring or automatic analysis.

## Install and configure

From the extracted `codecallnavigator-codex` directory:

```sh
codex plugin marketplace add .
codex plugin add codecallnavigator@codecallnavigator-local
```

Separately configure a project-bound MCP server in the intended trusted
project's `.codex/config.toml`, substituting its explicit project root:

```toml
[mcp_servers.codecallnavigator]
command = "codecallnavigator-mcp"
args = ["--engine", "ccnavigator", "--project", "/absolute/path/project"]
tool_timeout_sec = 1800
```

The timeout accommodates an explicitly requested analysis (the adapter allows
30 minutes); ordinary queries retain the adapter's shorter query deadline.
Codex otherwise defaults to a 60-second tool timeout. See the
[official MCP configuration guide](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

The executable values are command names on PATH, not binary paths. The project
argument is an explicit data address; never bind to the plugin directory or
infer it from a server's working directory. The adapter's optional `--root
NAME=PATH` arguments select only explicitly configured linked roots.

A user-scope registration is an alternative. Run this in the intended project
and use a distinct server name when retaining several projects:

```sh
codex mcp add codecallnavigator -- codecallnavigator-mcp --engine ccnavigator --project "$PWD"
```

For that alternative, set `tool_timeout_sec = 1800` in the corresponding
user-level server table as well if you will request long analyses.

Use one registration at the intended scope rather than duplicate conflicting
project/user entries. Keep normal Codex trust and MCP tool approvals; do not
blanket approve refresh. Tell the agent the selected server-to-project mapping
or expose trusted configuration. Neither workspace path nor status generation
proves that the available server is bound to that project.

## Invoke

Start a new Codex CLI session in the intended project and mention the skill:

```text
$graph-context Explain possible callers of leaf.
```

Use `/skills` to select the installed `graph-context` skill if another skill has
the same name. You can also ask “Use the CodeCallNavigator graph to explain
callers of leaf.” Description-based selection is a model decision; invoke
explicitly if needed. An unclear server binding causes a clarification before
queries.

The workflow uses native `graph_status` and generation-bound `find_symbols`,
`symbol_details`, `graph_section`, `function_flow` and related tools. It preserves
ambiguity, freshness and bounded evidence. Possible relationships are not
observed execution. Only explicit user refresh requests/approval allow
`refresh_graph`; ordinary queries do not analyze.

If MCP is absent/disabled, the configured CLI is incompatible or a tool fails,
Codex explains the limitation and may inspect source while excluding saved graph
storage. The skill does not silently query the terminal CLI, read graph artifacts,
auto-configure servers or run shell analysis. The independent `ccnavigator`
terminal command remains separately available to users.

## Disable and remove

Disable the installed plugin through its existing Codex configuration entry:

```toml
[plugins."codecallnavigator@codecallnavigator-local"]
enabled = false
```

Remove the package and local marketplace:

```sh
codex plugin remove codecallnavigator@codecallnavigator-local
codex plugin marketplace remove codecallnavigator-local
```

Separately remove the project `[mcp_servers.codecallnavigator]` table if that was
the configured scope. For a user-scope registration instead use:

```sh
codex mcp remove codecallnavigator
```

Use the actual configured server name and scope. Package removal alone does not
remove separately configured MCP. Start a new session and verify both removed
skill and tools disappear. Preserve the installed engine/MCP binaries, source
and published graph artifacts.

## Packaging and supported surface

From the repository root, maintainers check canonical-body drift with:

```sh
python3.13 tools/package_agent_integrations.py --host codex --check
```

`--sync` explicitly regenerates wrappers. Default builds require a new output
root and emit self-contained plugin directories and deterministic ZIPs with the
license. Both host skill bodies are copied from the same canonical file. There
is no bundled guessed-project MCP configuration.

Target: local Codex CLI on macOS with normal authentication and native MCP
access. `requires_local_executor` is a runtime requirement, not proof of desktop
support. Desktop install/invocation/removal have not been validated in an
isolated desktop configuration, so this package makes no desktop support claim.
IDE/cloud/web and other operating systems are unvalidated. Exact CLI versions
and native-MCP acceptance are recorded in the repository evidence; prior CLI-only
conversations do not prove this route.
