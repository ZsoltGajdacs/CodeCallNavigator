# CodeCallNavigator for Claude Code

This plugin teaches Claude Code to investigate symbols, callers/callees,
function flow and possible change impact through configured native MCP tools.
Install the engine and MCP adapter separately so `ccnavigator` and
`codecallnavigator-mcp` are on PATH. Preserve engine analyzer resources. The
plugin contains instructions only: no binaries, bundled server wiring, hooks,
graph reader or automatic analysis.

## Install and configure

From the extracted `codecallnavigator-claude-code` package directory:

```sh
claude plugin marketplace add .
claude plugin install codecallnavigator@codecallnavigator-local
```

Separately, **from the project you intend to analyze**, register its personal
project-scoped server:

```sh
claude mcp add --scope local codecallnavigator -- codecallnavigator-mcp --engine ccnavigator --project "$PWD"
```

This captures the explicit project address while both executables are command
names on PATH; no binary paths are configured. It does not use the plugin
installation directory as the project. Use distinct server names for different
projects. Optional linked roots must be explicitly configured with the adapter's
`--root NAME=PATH` arguments; do not infer them from the plugin or workspace.

Keep normal Claude Code plugin/MCP trust and tool approvals. Do not blanket
approve refresh at installation. Restart/reload as requested and confirm the
server is connected. Tell the agent that the selected server is bound to this
project (or expose the trusted host configuration). A workspace path or graph
generation cannot establish that mapping by itself.

## Invoke

```text
/codecallnavigator:graph-context Explain possible callers of leaf.
```

You can also ask “Use the CodeCallNavigator graph to explain callers of leaf.”
The installed description permits implicit selection; invoke explicitly if the
model does not select it. With an unclear project/server mapping the skill asks
before issuing graph queries.

The workflow calls native `graph_status`, then generation-bound tools such as
`find_symbols`, `symbol_details`, `graph_section` and `function_flow`. It preserves
ambiguity, freshness, provenance and truncation. Possible relationships are not
observed execution. Only an explicit user refresh request/approval permits
`refresh_graph`; ordinary queries never analyze.

Missing/disabled MCP, a missing/incompatible CLI or failed tools produce an
honest limitation. Source exploration may continue, excluding saved graph
storage. There is no silent terminal graph-query fallback, auto-install or shell
analysis. The independent `ccnavigator` terminal command remains available to
users separately from this plugin workflow.

## Disable and remove

```sh
claude plugin disable codecallnavigator@codecallnavigator-local
claude plugin uninstall codecallnavigator@codecallnavigator-local
claude plugin marketplace remove codecallnavigator-local
```

For the registration above, separately run from the same project:

```sh
claude mcp remove --scope local codecallnavigator
```

Use the actual name and scope selected during setup. Removing the plugin does
not remove a separately registered server automatically. Start a new session
and confirm the skill and removed server tools are absent. Do not delete the
independently installed engine/MCP binaries, source or published graph artifacts.

## Packaging and support

Maintainers detect canonical-body drift from the repository root with:

```sh
python3.13 tools/package_agent_integrations.py --host claude-code --check
```

`--sync` explicitly regenerates wrappers; default builds require a new output
directory and produce a self-contained plugin directory and deterministic ZIP
with the license. There is intentionally no bundled `.mcp.json` with a guessed
working directory: project-bound server configuration is a separate user step.

Target: local Claude Code on macOS, with normal authentication, MCP trust and
tool approvals. Other operating systems, claude.ai and Cowork are unvalidated.
Exact tested versions and native-MCP journeys are recorded in the repository's
agent-integration evidence; earlier CLI-only evidence does not validate this route.
