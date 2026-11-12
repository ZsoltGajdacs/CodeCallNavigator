# Isolated host configuration probes

2026-11-12; codex-cli 0.166.0; Claude Code 2.1.306. Temporary CODEX_HOME and CLAUDE_CONFIG_DIR; no production configuration changed. These are configuration/manifest checks, not model invocation or interactive hook-trust tests.

```json
[
  {
    "command": [
      "codex",
      "mcp",
      "add",
      "graph-research",
      "--",
      "<repository>/dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp",
      "--engine",
      "<repository>/dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine",
      "--project",
      "<isolated-config>"
    ],
    "exit": 0,
    "stdout": "Added global MCP server 'graph-research'.\n",
    "stderr": ""
  },
  {
    "command": [
      "codex",
      "mcp",
      "get",
      "graph-research"
    ],
    "exit": 0,
    "stdout": "graph-research\n  enabled: true\n  transport: stdio\n  command: <repository>/dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp\n  args: --engine <repository>/dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine --project <isolated-config>\n  cwd: -\n  env: -\n  remove: codex mcp remove graph-research\n",
    "stderr": ""
  },
  {
    "command": [
      "codex",
      "mcp",
      "remove",
      "graph-research"
    ],
    "exit": 0,
    "stdout": "Removed global MCP server 'graph-research'.\n",
    "stderr": ""
  },
  {
    "command": [
      "codex",
      "mcp",
      "list"
    ],
    "exit": 0,
    "stdout": "No MCP servers configured yet. Try `codex mcp add my-tool -- my-command`.\n",
    "stderr": ""
  },
  {
    "command": [
      "claude",
      "mcp",
      "add",
      "--scope",
      "user",
      "graph-research",
      "--",
      "<repository>/dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp",
      "--engine",
      "<repository>/dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine",
      "--project",
      "<isolated-config>"
    ],
    "exit": 0,
    "stdout": "Added stdio MCP server graph-research with command: <repository>/dist/CodeCallNavigatorMcp/bin/codecallnavigator-mcp --engine <repository>/dist/CodeCallNavigatorEngine/bin/codecallnavigator-engine --project <isolated-config> to user config\nFile modified: <isolated-config>/claude/.claude.json\n",
    "stderr": ""
  },
  {
    "command": [
      "claude",
      "mcp",
      "remove",
      "--scope",
      "user",
      "graph-research"
    ],
    "exit": 0,
    "stdout": "Removed MCP server graph-research from user config\nFile modified: <isolated-config>/claude/.claude.json\n",
    "stderr": ""
  },
  {
    "command": [
      "claude",
      "plugin",
      "validate",
      "<isolated-config>/plugin"
    ],
    "exit": 0,
    "stdout": "Validating plugin manifest: <isolated-config>/plugin/.claude-plugin/plugin.json\n\n\u26a0 Found 1 warning:\n\n  \u276f author: No author information provided. Consider adding author details for plugin attribution\n\nPlugin directory checks for <isolated-config>/plugin. The plugin directory is the catalog that plugins are submitted to; this is a subset of what it runs.\n\n\u2718 Found 2 problems the plugin directory refuses a submission for:\n\n  \u276f README.md: README missing (README_MISSING). This plugin has no README.md. People who install it and the directory's automated review read the README to learn what it does; the first line of a README in the plugin folder also stands in as the listing summary when plugin.json has no description.\n    Fix: Add a README.md to the plugin folder describing what the plugin does, how to use it, and any setup it needs, in at least 40 words. A README at the repository root also counts for a plugin in a sub-folder.\n  \u276f LICENSE: License missing (LICENSE_MISSING). This plugin has no LICENSE file and plugin.json has no `license`. The directory requires one of them.\n    Fix: Add a LICENSE file to the plugin folder, or set `license` (an SPDX id) in plugin.json. A LICENSE at the repository root also counts for a plugin in a sub-folder.\n\n\u26a0 Found 1 plugin directory warning:\n\n  \u276f .claude-plugin/plugin.json icon: No icon (ICON_MISSING). No icon found in the plugin. The validator looked for `icon` in plugin.json and for a file at .claude-plugin/icon.png or .svg, assets/icon.png or .svg, or a similar path.\n    Fix: Add a square PNG to the plugin folder at .claude-plugin/icon.png, or set `icon` in plugin.json to the path of a square PNG or JPEG in the plugin: 512 to 2048 px on each side, under 2 MB. SVG and WebP are not accepted. That file may become the listing icon only once: the first time the plugin is saved or submitted in the developer portal. Adding or changing it later does not change the icon.\n\nThese plugin directory checks do not change the result below. To fail on problems the directory refuses a submission for, run this command with --profile directory.\n\nThe directory also runs a safety scan and a policy review that this command does not.\n\n\u2714 Validation passed with warnings\n",
    "stderr": ""
  }
]
```
