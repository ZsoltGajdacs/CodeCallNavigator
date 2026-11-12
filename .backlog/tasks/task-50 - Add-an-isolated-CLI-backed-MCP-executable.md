---
id: TASK-50
title: Add an isolated CLI-backed MCP executable
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T13:17:00Z'
updated_date: '2026-11-12T17:47:00Z'
labels: []
dependencies:
  - TASK-49
references:
  - DRAFT-1
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expose the desktop-proven public CLI interface to agents without linking MCP to engine, core, desktop, analyzer or storage implementation. Independently installable MCP communicates only with the CLI executable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Separate Rust MCP package exposes graph query tools and explicit refresh over stdio using the official SDK.
- [x] #2 MCP imports no core, engine or desktop libraries including tests and performs no direct graph/source/database access or graph algorithms.
- [x] #3 Configured executable/project/roots, compatibility checks, structured metadata/errors and cancellation work through CLI subprocesses.
- [x] #4 Real-CLI end-to-end coverage, dependency-boundary checks, packaged installation, full verification and CodeScene pass.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add independently installable Rust MCP workspace package using official rmcp3.6 SDK/stdin-stdout, separate configuration/wire/tools/backend/process responsibilities and no repository-library imports including tests. 2. Configure fixed installed CLI/project/linked roots; startup capability query verifies version1/required operations. Expose bounded named status/search/symbol/selection/neighbors/source/flow/explorer and discovery/explicit refresh tools, generation binding and structured response metadata. Start with one CLI subprocess per operation; no MCP graph cache/session required for agent request cadence. 3. Validate external tool shapes and CLI wire envelopes without semantic graph logic; lifecycle refresh reuses CLI discovery defaults and serve-once analyze protocol, fixed configured roots, no inferred extra roots. Cancellation/EOF/deadlines stop owned CLI work, cooperatively cancel analysis before bounded forced cleanup. 4. Verify real stdio MCP+CLI compiler-generated query/refresh/cancellation behavior and narrow fake transport failures; enforce dependency/source boundary mechanically. 5. Add MCP-only package/install verification independent of app/analyzer resources, integrate full verification, documentation and CodeScene; finish local commit after gates, no push.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User explicitly instructed during TASK-49 implementation: finish and commit desktop work, then stop. MCP implementation must remain unstarted until a later request. Task remains To Do; no MCP package or dependencies have been created.

User explicitly authorized starting MCP after desktop completion on 2026-11-12. Prior defer is revoked; task49 is Done in local commit3b4ffa9. Beginning planner/explorer and official SDK research; strict CLI-only runtime and test dependency boundary remains mandatory.

Planner/explorer completed: bounded agent tools omit desktop full-data exceptions; refresh uses CLI discovery defaults and serve-once machine protocol with stdin kept open. Chose one subprocess per MCP operation to keep adapter state limited to startup configuration, capabilities and process lifecycle; interactive CLI-owned query-session optimization remains available but is not required for agent cadence. Added transitive dependency/source boundary regressions (observed missing module red, three tests now green), independent cargo archive/install packaging+realstdio smoke, full verification integration, and documentation link. Root tooling CodeScene findings resolved with cohesive dependency reachability/read-message helpers; reviews now10. Sonar remains inactive: no .sonar/config.json.

First observable MCP executable: real stdio initialize/list/status against packagedCLI passed, preserving graph_missing without starting analysis. Independently built and installed source .crate outside workspace successfully; first package assembly found incorrect NOTICE filename, corrected to existing THIRD_PARTY_NOTICES.md. Tester realTypeScript refresh/directCLI query parity and realGradle cancellation passed: exact prior compiler-produced artifact retained, wrapper+sleep reaped and same-generation query worked afterward. Reviewer identified forced fallback missing separately grouped analyzer descendants, malformed freshness accepted, and missing --help packaging command; implementing focused fixes before finalfull verification. Runtime/process boundary remains independent; OS process metadata inspection is lifecycle cleanup support, never graph/source access.

Package verification caught a stale Cargo install artifact: packaged main.rs already contained --help but shared build cache reused an older binary because archive timestamps are normalized. Fixed independent installation to use a fresh build directory outside the workspace and override both Cargo target/build directories; complete standalone package, signature/help and realCLIstdio smoke now pass (target/task50-package-fresh.log). Final process review deliberately scopes ownership guarantees to liveCLI cancellation/EOF: unexpected CLI crash before descendant observation cannot guarantee discovery of orphaned separate groups; adapter must report unverified cleanup explicitly instead of retaining unsafe long-lived bare PIDs. No polling registry or graph cache added.

Final source independent review closed all findings: early --help/version without backend, validated nonnull analyzed freshness, stderr drain concurrent with pre-request hello, and live analysis descendant-aware cleanup. Abrupt-exit uncertainty is surfaced explicitly, with no historical PID cache. Eighteen focused unit tests and all-targetClippy pass; all productionRustfilesCodeScene10. Final full run started at target/verification/task50-full-20261112. npm_config_audit=false/fund=false avoids unrelated private-fixture npm audit metadata network retries; no compiler/test/verification stage is skipped.

First full run stopped in existing Python verification timeout fixture after545ordinaryRusttests passed: 300ms deadline expired before fake server wrote readiness PID. Exact16-test quality suite reproduced independently. Narrow test-only correction gives interpreter startup1s while retaining <3s overall assertion,30s hanging server and both PID cleanup assertions;16tests now pass and fileCodeScene10. Independent review confirms unchanged cleanup semantics. Restarted all full stages at target/verification/task50-full-final-20261112; original failure evidence retained.

Second full retry exposed a separate test-only readiness race: desktop blocked-stdin fixture observed an empty PID file between creation and write. Replaced affected desktop/MCP/quality PID fixture publication with same-directory temporary files and atomic rename; no production behavior changed. Seven desktop query tests and16quality tests pass; independent review confirms timeout, process-death, restart and late-response assertions retained. CodeScene10 for changed fixtures. Restarted complete verification at target/verification/task50-full-20261112-r3, retaining both earlier failure reports. Root AGENTS ownership table now records the permanent strict MCP boundary.

All44 full stages passed: target/verification/task50-full-20261112-r3. Rust546ordinary/636release,Python164+26,TypeScript112; storage,CPU/GPU,packaging,signatures,independentMCPinstall+stdio allpass. CodeScene23/23 nofindings/regressions;cleanup passed. Evidence evidence/mcp/README.md. Abrupt CLI crash cleanup uncertainty documented; live cancellation/disconnect verified. Sonar unconfigured.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Separate CLI-only MCP package with ten tools, explicit refresh, generation/freshness validation and bounded owned subprocesses. Dependency guards and independent archive installation enforce separation. Full verification and CodeScene passed. Setup docs/mcp.md; evidence evidence/mcp/README.md. Local commit, no push.
<!-- SECTION:FINAL_SUMMARY:END -->
