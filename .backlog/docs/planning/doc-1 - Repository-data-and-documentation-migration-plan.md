---
id: doc-1
title: Repository data and documentation migration plan
type: guide
created_date: '2026-11-16T07:02:00Z'
updated_date: '2026-11-16T07:15:00Z'
---
# Repository data and documentation migration plan

> Execution status: approved by the user on 2026-11-16 and implemented as TASK-66 on the existing branch. The user subsequently replaced full verification with light checks only; that instruction supersedes the full-verification proposal below. This document preserves the researched plan; current outcomes and source dispositions are in the internal documentation index.

Requested 2026-11-16. Planning only. Recheck branch/status before implementation; preserve concurrent work. Latest inspected working tree was clean, but the preceding investigation had TASK-22 WIP. No reset, stash, history rewrite, commit or push is part of this proposal.

## Ownership policy

Internal project documentation belongs in Backlog documents, maintained through the Backlog CLI. Public user, installation, integration, wire/schema and redistribution documentation stays in docs/. Split mixed documents by audience. Keep task scope and execution plans in Backlog tasks. Keep executable baselines/fixtures and machine manifests with their owning code, not in prose or output directories. Generated logs, captures, traces, archives and reports belong in ignored output or externally retained artifacts.

## Required data relocation

| Current path | Proposed destination | Consumers |
| --- | --- | --- |
| evidence/engine/corpus.json | fixtures/engine/corpus.json | tools/check_engine_artifact.py, check_engine_reference.py, update_engine_corpus.py |
| evidence/flowcharts/csharp/pre-export-baseline.json | fixtures/flow/csharp-baselines/pre-export-baseline.json | tools/csharp_flow_acceptance.py, tests/test_csharp_fixture_ownership.py, tools/test_dependency_patches.py |
| evidence/flowcharts/csharp/ingestion/pre-ingestion.json | fixtures/flow/csharp-baselines/pre-ingestion.json | tools/csharp_flow_ingestion_acceptance.py, tests/test_csharp_fixture_ownership.py |
| evidence/flowcharts/csharp/ingestion/verification.json | fixtures/flow/csharp-baselines/ingestion-checkpoint.json | tools/test_dependency_patches.py reads patch_sha256 |

Initially preserve all four files byte-for-byte. Reduce the last file to checkpoint metadata only after confirming that nothing else needs its fields. Do not regenerate compiler expectations or rewrite historical physical paths embedded in JSON. Compare hashes before and after relocation.

The repeated tracked-code literal-path and split-path searches found no additional repository-level evidence data readers. tests/test_verification.py creates evidence under a temporary root; graph properties named evidence are unrelated. Repeat the audit when implementing, including untracked/new files, dynamic paths, shell/config/packaging snippets and transitive inputs. Add any newly discovered input to a source/destination/consumer manifest before untracking outputs.

tools/check_callatlas.py and check_jdt.py default to evidence/analyzers for generated probe inputs/outputs; check_bytecode.py writes its summary there. Redirect defaults to target/verification/analyzers, create parents as needed and preserve explicit overrides. Their generated summary files do not become fixtures.

Keep fixtures/.codecallnavigator/graph.json and boundaries.yml, tools/verification/test-inventory.json, analyzers.lock.json and patches/checkpoints with their existing owners.

## Useful evidence documents: consolidate by topic

Use backlog doc list/search, create and update. Never hand-edit managed Markdown. There were no Backlog documents before this plan.

| Proposed Backlog topic | Sources | Useful content |
| --- | --- | --- |
| Analyzer behavior and regression acceptance | evidence/analyzers/**/*.md | Compiler ownership/certainty, unsupported cases, regression rationale and reproduction |
| Engine, graph contract, queries and MCP architecture | evidence/engine/**/*.md, graph-contract/, graph-comparison/, queries/, mcp/ Markdown | Boundaries, determinism, compatibility, process recovery and measurement methods |
| Function-flow semantics and delivery | evidence/flowcharts/**/*.md | C# export/ingestion/cleanup/execution, expansion and revision comparison rationale and unique acceptance boundaries |
| Freshness design and measurements | evidence/freshness/**/*.md | Invalidation/polling choices, measured scope, failure semantics and remaining limits |
| Agent integration research and host validation | evidence/agent-integrations/*.md | Native MCP decision, host trust/delivery limits, outstanding TASK-55/56 work; label superseded routes historical |
| Rendering and interaction design | evidence/ui/**/*.md, evidence/architecture/README.md, design-qa.md | Approved design decisions, meaningful interaction invariants and reproducible native checks |
| Performance methods and historical results | evidence/performance/**/*.md | Workloads, environment, before/after findings and limitations |
| Testing migration and verification operations | evidence/testing-strategy/*.md, evidence/verification/**/*.md | Coverage-placement rationale, cleanup ownership/recovery and dated dependency verification |
| Publication and release history | evidence/public-repository.md, evidence/ci/*.md, evidence/VALIDATION.md | Durable publication/compliance findings and unresolved validation |

Inventory every source Markdown file with a disposition: migrated in full; consolidated into a named document/section; retained externally; or redundant/superseded with reason. Do not copy every run report. Preserve useful methods, decisions, unresolved limitations and reproduction commands. Record original path, source commit/date and historical status so old counts/results never become current guarantees. Check existing tasks for already-recorded findings before duplicating.

Resolve links to removed raw artifacts: inline small decisive dated measurements, link a stable artifact if available or a source-commit permalink, or explain historical capture availability and provide reproduction. No external publication is authorized here. Retain a curated design image outside evidence only if uniquely useful, for example a Backlog docs asset after checking supported path conventions. Do not migrate screenshot collections, raw model sessions, credentials or workstation details.

## All current docs/ files

| File | Proposed disposition |
| --- | --- |
| cli-installation.md | Keep external installation/update/removal; packaging copies it into INSTALL.md. |
| cli-queries.md | Keep public commands and wire contract; extract internal delivery/testing commentary if present. |
| graph-contract.md | Keep external portable graph contract and consumer compatibility. |
| graph.schema.json | Keep external machine-readable schema at its public path. |
| mcp.md | Keep installation, public tools and response/cancellation guarantees; internal verification methods move. |
| function-flowcharts.md | Keep user operation, supported semantics/provenance/limits; move delivery records. |
| architectural-layers.md | Split: keep user config/import/headless commands; move cache/module internals. |
| reference.md | Split: keep usage, source-build prerequisites and public analyzer limits; move internal testing gates/runbooks. Repair README and prerequisite-message links. |
| standalone-engine.md | Split: keep commands, artifacts, process/wire contract and supported semantics; move internal module ownership, analyzer implementation and library-only expansion details. Keep genuinely supported third-party extension contracts public. |
| public-releases.md | Split: keep distribution/license/corresponding-source/rebuild content consumed by tools/package.py; move maintainer-only release preparation. |
| desktop-cli.md | Move internal desktop/CLI lifecycle and architecture into Backlog. |
| testing-strategy.md | Move internal strategy, ownership, cleanup and timing into Backlog; repair root/scoped AGENTS links. |
| plans/task-22-function-flowcharts.md | Move useful investigation/decision history into Backlog, link TASK-22; current execution plans belong in tasks. |
| testing/flow-comparison-native.md | Move internal native acceptance runbook into Backlog. |
| verification/rename-2026-11-11.md | Move/consolidate useful dated verification and limitations into Backlog history. |
| public-repository-checklist.md | Move maintainer GitHub settings/publication checklist into Backlog. |
| agent-integrations.md | Move dated research/comparison into Backlog; retain a short external installation/usage entry point if needed. |
| graph-freshness.md | Split: move feasibility/design/scheduling/delivery research; keep public states/configuration/guarantees/limits. |
| engine-adaptations.md | Move internal roadmap/rationale; preserve attribution in public notices/distributed documentation where appropriate. |

During reference repair, inspect internal prose in fixture/tool/patch readmes and integration journey notes; consolidate substantial internal documentation and leave short local purpose/pointer files where useful. Preserve distributed package-facing READMEs, integration SKILL.md files, license/notices, instruction entry points, public schemas and executable manifests in their conventional ownership locations.

## AGENTS policy to add during implementation

Internal project documentation belongs in Backlog documents and must be created or updated through the Backlog CLI. This includes architecture and design rationale, research, implementation investigations, maintainer runbooks, testing strategy and durable verification summaries. Keep task scope and execution plans in Backlog tasks. Reserve docs/ for external user, installation, integration, public contract and redistribution documentation. Split mixed documents by audience and link to the canonical owner instead of duplicating content. Keep executable fixtures, schemas, manifests, instruction entry points, package-facing skills and license notices with their owning code or distribution. Generated logs, screenshots, traces and reports belong in ignored artifact directories and must not be checked in.

Update root ownership guidance and all scoped AGENTS links to actual CLI-created paths. Add a Backlog internal-document index; keep public navigation in README focused on external docs.

## Sequenced execution

1. Revalidate inventory/status; relocate required data, update readers and redirect output defaults. Preserve baseline hashes and run affected behavioral checks.
2. Create internal topic documents and the complete source-disposition manifest before removing any original prose. Label historical findings and unresolved blockers explicitly.
3. Split/migrate docs by audience. Build an old-path/anchor to new-path/anchor map. Repair relative links among Backlog docs, public docs, root/scoped AGENTS, README, patches, fixture/tool readmes, task documentation metadata and verification messages. Use CLI for all managed records; preserve historical command quotations rather than silently rewriting history. Respect archived read-only records.
4. Record AGENTS policy. Confirm packaged INSTALL.md, release-source guide and graph schema remain usable. tools/package.py directly reads docs/cli-installation.md and docs/public-releases.md.
5. After all needed inputs, useful prose and essential curated assets have new owners, untrack evidence while retaining local outputs; add /evidence/ to .gitignore. Prefer target/verification for new outputs. Do not rewrite Git history or delete local captures.
6. Validate and review the full diff; no unrelated changes or WIP alterations.

## Verification

- Data hashes and JSON validity; workload/source paths resolve; source encodings and frozen expectations remain unchanged.
- Affected fixture-ownership and dependency-checkpoint tests, plus real C# acceptance/corpus boundaries that consume relocated inputs. No tests merely duplicating path constants.
- Content/path/anchor/syntax/diff checks for documentation-only slices. Required runtime/compiler checks for executable default/path changes. Applicable packaging checks for public embedded documents.
- Full verification for the combined verification/bootstrap/packaging-impacting migration, outside the macOS sandbox as required by AGENTS. Keep compilation/setup/execution/cleanup timing and limitations explicit.
- CodeScene review modified source files; safeguard before a future commit and analyze_change_set before a future PR. No commit/PR is requested here.
- Final checks: no live reader depends on repository evidence, no required input is ignored, source document dispositions are complete, public contracts remain navigable and output is untracked.

Revalidate the researched proposal when picking up its draft task. This plan does not execute the migration.
