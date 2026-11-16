---
id: doc-25
title: MCP verification boundaries
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:06:00Z'
---
# MCP verification boundaries

Migrated 2026-11-16 from `docs/mcp.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

## Verification

`cargo test --locked -p codecallnavigator-mcp` covers configuration, argument/wire
validation and real subprocess transport lifecycle. Process-only integration
acceptance under this package's `tests/` invokes the real CLI and compiler
fixtures without importing workspace libraries. The workspace verification runner
also enforces the dependency/source boundary and installs the packaged crate
outside the workspace before an MCP stdio smoke test.
