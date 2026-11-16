---
id: doc-33
title: Internal documentation index and migration manifest
type: readme
created_date: '2026-11-16T07:09:00Z'
updated_date: '2026-11-16T07:15:00Z'
---
# Internal documentation index and migration manifest

Internal documentation is owned by Backlog documents, edited through the CLI. Public usage, installation, integration and redistribution guides remain in [docs/](../../docs/). Executable baselines and manifests remain with their owning tooling/fixtures. Run output belongs in ignored target/verification; historical local evidence is retained but untracked.

Migration: TASK-66, 2026-11-16. Source revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. All reported historical results retain their original scope and limitations; they are not current verification guarantees. [Approved migration plan](<planning/doc-1 - Repository-data-and-documentation-migration-plan.md>).

## Maintained internal guides

- [Desktop query architecture and lifecycle](<architecture/doc-11 - Desktop-query-architecture-and-lifecycle.md>)
- [Resolution adaptation rationale and attribution](<architecture/doc-17 - Resolution-adaptation-rationale-and-attribution.md>)
- [Saved-edit freshness design and delivery rationale](<architecture/doc-19 - Saved-edit-freshness-design-and-delivery-rationale.md>)
- [Architectural layer implementation boundaries](<architecture/doc-20 - Architectural-layer-implementation-boundaries.md>)
- [Engine architecture and analyzer implementation](<architecture/doc-22 - Engine-architecture-and-analyzer-implementation.md>)
- [Function flow implementation boundaries](<architecture/doc-24 - Function-flow-implementation-boundaries.md>)
- [TypeScript compiler adapter internals](<engineering/doc-26 - TypeScript-compiler-adapter-internals.md>)
- [Java bytecode adapter internals](<engineering/doc-27 - Java-bytecode-adapter-internals.md>)
- [Maven metadata extension internals](<engineering/doc-28 - Maven-metadata-extension-internals.md>)
- [Maven fixture verification guide](<engineering/doc-29 - Maven-fixture-verification-guide.md>)
- [MediatR fixture verification guide](<engineering/doc-30 - MediatR-fixture-verification-guide.md>)
- [Spring event fixture verification guide](<engineering/doc-31 - Spring-event-fixture-verification-guide.md>)
- [Upstream patch purposes revisions and application order](<engineering/doc-32 - Upstream-patch-purposes-revisions-and-application-order.md>)
- [Public repository maintenance and publication checklist](<operations/doc-16 - Public-repository-maintenance-and-publication-checklist.md>)
- [Release maintenance and publication runbook](<operations/doc-23 - Release-maintenance-and-publication-runbook.md>)
- [Repository data and documentation migration plan](<planning/doc-1 - Repository-data-and-documentation-migration-plan.md>)
- [Function flowchart investigation and delivery history](<planning/doc-14 - Function-flowchart-investigation-and-delivery-history.md>)
- [Agent companion integration research and host boundaries](<research/doc-18 - Agent-companion-integration-research-and-host-boundaries.md>)
- [Testing strategy](<testing/doc-12 - Testing-strategy.md>)
- [Native saved-flow comparison acceptance](<testing/doc-13 - Native-saved-flow-comparison-acceptance.md>)
- [Verification prerequisites and operations](<testing/doc-21 - Verification-prerequisites-and-operations.md>)
- [MCP verification boundaries](<testing/doc-25 - MCP-verification-boundaries.md>)

## Historical findings by topic

- [Publication and release verification history](<history/doc-2 - Publication-and-release-verification-history.md>)
- [Agent integration research and host validation](<history/doc-3 - Agent-integration-research-and-host-validation.md>)
- [Analyzer behavior and regression acceptance](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md>)
- [Rendering interaction design and acceptance history](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md>)
- [Engine artifact query and MCP verification history](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md>)
- [Function flow semantics and delivery history](<history/doc-7 - Function-flow-semantics-and-delivery-history.md>)
- [Freshness design measurements and delivery history](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md>)
- [Performance methods and historical measurements](<history/doc-9 - Performance-methods-and-historical-measurements.md>)
- [Testing migration and verification operations](<history/doc-10 - Testing-migration-and-verification-operations.md>)

## Preserved executable inputs

No baseline was regenerated. Source bytes and embedded historical identities were preserved.

| Former path | Current owner | SHA-256 |
| --- | --- | --- |
| `evidence/engine/corpus.json` | [fixtures/engine/corpus.json](<../../fixtures/engine/corpus.json>) | `eb45bac1d224a3dd34d67310d60d3d5d53b369a543269d40c870a839b45b5f50` |
| `evidence/flowcharts/csharp/pre-export-baseline.json` | [fixtures/flow/csharp-baselines/pre-export-baseline.json](<../../fixtures/flow/csharp-baselines/pre-export-baseline.json>) | `31620861924e13805688411ab247cd3effe0afffa1bc12b5b1f72bb3388fd12f` |
| `evidence/flowcharts/csharp/ingestion/pre-ingestion.json` | [fixtures/flow/csharp-baselines/pre-ingestion.json](<../../fixtures/flow/csharp-baselines/pre-ingestion.json>) | `7a84ee42b1d7702ee7b5450d4d7d592c6d9bfc1cf9c52a1844ccfe4c116fa635` |
| `evidence/flowcharts/csharp/ingestion/verification.json` | [fixtures/flow/csharp-baselines/ingestion-checkpoint.json](<../../fixtures/flow/csharp-baselines/ingestion-checkpoint.json>) | `6a654f856c06348ecd2694a08ee7e741cdfbef715bebbf44c543c5e25b09fad3` |

## Public docs dispositions

Public contract/schema/install/release paths remain stable.

| Source | Disposition | Internal owner when split or moved |
| --- | --- | --- |
| `docs/desktop-cli.md` | migrated in full | [doc-11 - Desktop-query-architecture-and-lifecycle.md](<architecture/doc-11 - Desktop-query-architecture-and-lifecycle.md>) |
| `docs/testing-strategy.md` | migrated in full | [doc-12 - Testing-strategy.md](<testing/doc-12 - Testing-strategy.md>) |
| `docs/testing/flow-comparison-native.md` | migrated in full | [doc-13 - Native-saved-flow-comparison-acceptance.md](<testing/doc-13 - Native-saved-flow-comparison-acceptance.md>) |
| `docs/plans/task-22-function-flowcharts.md` | migrated in full | [doc-14 - Function-flowchart-investigation-and-delivery-history.md](<planning/doc-14 - Function-flowchart-investigation-and-delivery-history.md>) |
| `docs/verification/rename-2026-11-11.md` | migrated in full | [doc-15 - CodeCallNavigator-rename-verification-2026-11-11.md](<history/doc-15 - CodeCallNavigator-rename-verification-2026-11-11.md>) |
| `docs/public-repository-checklist.md` | migrated in full | [doc-16 - Public-repository-maintenance-and-publication-checklist.md](<operations/doc-16 - Public-repository-maintenance-and-publication-checklist.md>) |
| `docs/engine-adaptations.md` | migrated in full | [doc-17 - Resolution-adaptation-rationale-and-attribution.md](<architecture/doc-17 - Resolution-adaptation-rationale-and-attribution.md>) |
| `docs/agent-integrations.md` | split external installation from historical internal research | [doc-18 - Agent-companion-integration-research-and-host-boundaries.md](<research/doc-18 - Agent-companion-integration-research-and-host-boundaries.md>) |
| `docs/graph-freshness.md` | split public guarantees from historical design | [doc-19 - Saved-edit-freshness-design-and-delivery-rationale.md](<architecture/doc-19 - Saved-edit-freshness-design-and-delivery-rationale.md>) |
| `docs/architectural-layers.md` | split internal module/cache ownership from external configuration | [doc-20 - Architectural-layer-implementation-boundaries.md](<architecture/doc-20 - Architectural-layer-implementation-boundaries.md>) |
| `docs/reference.md` | split verification runbook from public usage/build reference | [doc-21 - Verification-prerequisites-and-operations.md](<testing/doc-21 - Verification-prerequisites-and-operations.md>) |
| `docs/standalone-engine.md` | split implementation/library ownership from external contracts | [doc-22 - Engine-architecture-and-analyzer-implementation.md](<architecture/doc-22 - Engine-architecture-and-analyzer-implementation.md>) |
| `docs/public-releases.md` | split maintainer release workflow from external redistribution/source guide | [doc-23 - Release-maintenance-and-publication-runbook.md](<operations/doc-23 - Release-maintenance-and-publication-runbook.md>) |
| `docs/function-flowcharts.md` | split internal implementation from public chart operation/semantics | [doc-24 - Function-flow-implementation-boundaries.md](<architecture/doc-24 - Function-flow-implementation-boundaries.md>) |
| `docs/mcp.md` | split internal verification from public installation/tool contract | [doc-25 - MCP-verification-boundaries.md](<testing/doc-25 - MCP-verification-boundaries.md>) |
| `docs/graph-contract.md` | retained external | [graph-contract.md](<../../docs/graph-contract.md>) |
| `docs/cli-installation.md` | retained external | [cli-installation.md](<../../docs/cli-installation.md>) |
| `docs/cli-queries.md` | retained external | [cli-queries.md](<../../docs/cli-queries.md>) |
| `docs/graph.schema.json` | retained external machine-readable schema | [graph.schema.json](<../../docs/graph.schema.json>) |

## Local internal guide dispositions

These source directories retain short pointers; package-facing license/readme/skill material remains distributed in its required location.

- `tools/typescript/README.md` → [doc-26 - TypeScript-compiler-adapter-internals.md](<engineering/doc-26 - TypeScript-compiler-adapter-internals.md>)
- `tools/java-bytecode/README.md` → [doc-27 - Java-bytecode-adapter-internals.md](<engineering/doc-27 - Java-bytecode-adapter-internals.md>)
- `tools/maven-metadata/README.md` → [doc-28 - Maven-metadata-extension-internals.md](<engineering/doc-28 - Maven-metadata-extension-internals.md>)
- `fixtures/maven/README.md` → [doc-29 - Maven-fixture-verification-guide.md](<engineering/doc-29 - Maven-fixture-verification-guide.md>)
- `fixtures/mediatr-dispatch/README.md` → [doc-30 - MediatR-fixture-verification-guide.md](<engineering/doc-30 - MediatR-fixture-verification-guide.md>)
- `fixtures/spring-events/README.md` → [doc-31 - Spring-event-fixture-verification-guide.md](<engineering/doc-31 - Spring-event-fixture-verification-guide.md>)
- `patches/README.md` → [doc-32 - Upstream-patch-purposes-revisions-and-application-order.md](<engineering/doc-32 - Upstream-patch-purposes-revisions-and-application-order.md>)

## Evidence Markdown dispositions

Each source is consolidated into the linked topic. Decision/reproduction prose, dated scope and limitations are retained; large raw transcript/output blocks link to the immutable original commit instead of being copied. Other tracked evidence files are generated captures/reports, except the four executable inputs above. All original evidence files remain locally and their old tracked versions remain in Git history. No new artifact publication or history rewrite is involved.

| Source | Destination |
| --- | --- |
| `evidence/VALIDATION.md` | [Historical source section](<history/doc-2 - Publication-and-release-verification-history.md#source-evidence-validation-md>) |
| `evidence/ci/tag-release.md` | [Historical source section](<history/doc-2 - Publication-and-release-verification-history.md#source-evidence-ci-tag-release-md>) |
| `evidence/public-repository.md` | [Historical source section](<history/doc-2 - Publication-and-release-verification-history.md#source-evidence-public-repository-md>) |
| `evidence/agent-integrations/README.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-readme-md>) |
| `evidence/agent-integrations/claude-native-mcp.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-claude-native-mcp-md>) |
| `evidence/agent-integrations/codex-native-mcp.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-codex-native-mcp-md>) |
| `evidence/agent-integrations/delivery-mcp.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-delivery-mcp-md>) |
| `evidence/agent-integrations/delivery-task52.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-delivery-task52-md>) |
| `evidence/agent-integrations/hook-spikes.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-hook-spikes-md>) |
| `evidence/agent-integrations/hosts.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-hosts-md>) |
| `evidence/agent-integrations/native-mcp-hosts-initial.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-native-mcp-hosts-initial-md>) |
| `evidence/agent-integrations/protocol.md` | [Historical source section](<history/doc-3 - Agent-integration-research-and-host-validation.md#source-evidence-agent-integrations-protocol-md>) |
| `evidence/analyzers/ACCEPTANCE.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-acceptance-md>) |
| `evidence/analyzers/java-duplicates.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-java-duplicates-md>) |
| `evidence/analyzers/java-lambda-correlation.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-java-lambda-correlation-md>) |
| `evidence/analyzers/java-lambda-owner-refresh.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-java-lambda-owner-refresh-md>) |
| `evidence/analyzers/lombok-signatures.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-lombok-signatures-md>) |
| `evidence/analyzers/maven.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-maven-md>) |
| `evidence/analyzers/native-pipeline.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-native-pipeline-md>) |
| `evidence/analyzers/terminal-launch/README.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-terminal-launch-readme-md>) |
| `evidence/analyzers/typescript.md` | [Historical source section](<history/doc-4 - Analyzer-behavior-and-regression-acceptance.md#source-evidence-analyzers-typescript-md>) |
| `evidence/architecture/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-architecture-readme-md>) |
| `evidence/ui/aligned-levels/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-aligned-levels-readme-md>) |
| `evidence/ui/architectural-layers/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-architectural-layers-readme-md>) |
| `evidence/ui/build-picker-2026-11-12/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-build-picker-2026-11-12-readme-md>) |
| `evidence/ui/centered-traversal/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-centered-traversal-readme-md>) |
| `evidence/ui/cli-backend-2026-11-12/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-cli-backend-2026-11-12-readme-md>) |
| `evidence/ui/function-flowcharts/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-function-flowcharts-readme-md>) |
| `evidence/ui/graph-studio/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-graph-studio-readme-md>) |
| `evidence/ui/project-explorer-2026-11-12/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-project-explorer-2026-11-12-readme-md>) |
| `evidence/ui/relationship-filters-2026-11-11/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-relationship-filters-2026-11-11-readme-md>) |
| `evidence/ui/search-modal/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-search-modal-readme-md>) |
| `evidence/ui/selection-emphasis/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-selection-emphasis-readme-md>) |
| `evidence/ui/source-popup/README.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-evidence-ui-source-popup-readme-md>) |
| `design-qa.md` | [Historical source section](<history/doc-5 - Rendering-interaction-design-and-acceptance-history.md#source-design-qa-md>) |
| `evidence/engine/README.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-readme-md>) |
| `evidence/engine/review/README.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-review-readme-md>) |
| `evidence/engine/stages/boundaries.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-stages-boundaries-md>) |
| `evidence/engine/stages/incremental.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-stages-incremental-md>) |
| `evidence/engine/stages/java-equality-inference-2026-11-11.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-engine-stages-java-equality-inference-2026-11-11-md>) |
| `evidence/graph-comparison/README.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-graph-comparison-readme-md>) |
| `evidence/graph-contract/README.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-graph-contract-readme-md>) |
| `evidence/mcp/README.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-mcp-readme-md>) |
| `evidence/queries/README.md` | [Historical source section](<history/doc-6 - Engine-artifact-query-and-MCP-verification-history.md#source-evidence-queries-readme-md>) |
| `evidence/flowcharts/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-readme-md>) |
| `evidence/flowcharts/comparison-desktop/catalog-publication-fix/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-catalog-publication-fix-readme-md>) |
| `evidence/flowcharts/comparison-desktop/cleanup-reaping/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-cleanup-reaping-readme-md>) |
| `evidence/flowcharts/comparison-desktop/final-verification/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-final-verification-readme-md>) |
| `evidence/flowcharts/comparison-desktop/full-verification/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-full-verification-readme-md>) |
| `evidence/flowcharts/comparison-desktop/native-final/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-native-final-readme-md>) |
| `evidence/flowcharts/comparison-desktop/native-label-red/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-native-label-red-readme-md>) |
| `evidence/flowcharts/comparison-desktop/target-label-fix/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-target-label-fix-readme-md>) |
| `evidence/flowcharts/comparison-desktop/unit3b/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-unit3b-readme-md>) |
| `evidence/flowcharts/comparison-desktop/unit4/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-comparison-desktop-unit4-readme-md>) |
| `evidence/flowcharts/csharp/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-csharp-readme-md>) |
| `evidence/flowcharts/csharp/cleanup/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-csharp-cleanup-readme-md>) |
| `evidence/flowcharts/csharp/desktop/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-csharp-desktop-readme-md>) |
| `evidence/flowcharts/csharp/ingestion/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-csharp-ingestion-readme-md>) |
| `evidence/flowcharts/expansion/acceptance-map.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-acceptance-map-md>) |
| `evidence/flowcharts/expansion/desktop/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-desktop-readme-md>) |
| `evidence/flowcharts/expansion/desktop/acceptance-map.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-desktop-acceptance-map-md>) |
| `evidence/flowcharts/expansion/final/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-final-readme-md>) |
| `evidence/flowcharts/expansion/lock-lifetime/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-lock-lifetime-readme-md>) |
| `evidence/flowcharts/expansion/resumed-testing/coverage-map.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-resumed-testing-coverage-map-md>) |
| `evidence/flowcharts/expansion/unit1/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit1-readme-md>) |
| `evidence/flowcharts/expansion/unit2a/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit2a-readme-md>) |
| `evidence/flowcharts/expansion/unit2b/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit2b-readme-md>) |
| `evidence/flowcharts/expansion/unit2c/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit2c-readme-md>) |
| `evidence/flowcharts/expansion/unit3/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit3-readme-md>) |
| `evidence/flowcharts/expansion/unit4a/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4a-readme-md>) |
| `evidence/flowcharts/expansion/unit4b/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit4b-readme-md>) |
| `evidence/flowcharts/expansion/unit5a/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit5a-readme-md>) |
| `evidence/flowcharts/expansion/unit5b/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit5b-readme-md>) |
| `evidence/flowcharts/expansion/unit6/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-expansion-unit6-readme-md>) |
| `evidence/flowcharts/revisions/README.md` | [Historical source section](<history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-revisions-readme-md>) |
| `evidence/freshness/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-readme-md>) |
| `evidence/freshness/automatic-refresh/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-automatic-refresh-readme-md>) |
| `evidence/freshness/automatic-refresh/verification/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-automatic-refresh-verification-readme-md>) |
| `evidence/freshness/consumer-adoption/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-consumer-adoption-readme-md>) |
| `evidence/freshness/csharp-import-fix/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-csharp-import-fix-readme-md>) |
| `evidence/freshness/observations/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-observations-readme-md>) |
| `evidence/freshness/task61-desktop/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-task61-desktop-readme-md>) |
| `evidence/freshness/task62-desktop/README.md` | [Historical source section](<history/doc-8 - Freshness-design-measurements-and-delivery-history.md#source-evidence-freshness-task62-desktop-readme-md>) |
| `evidence/performance/analysis.md` | [Historical source section](<history/doc-9 - Performance-methods-and-historical-measurements.md#source-evidence-performance-analysis-md>) |
| `evidence/performance/canvas/README.md` | [Historical source section](<history/doc-9 - Performance-methods-and-historical-measurements.md#source-evidence-performance-canvas-readme-md>) |
| `evidence/performance/panning/README.md` | [Historical source section](<history/doc-9 - Performance-methods-and-historical-measurements.md#source-evidence-performance-panning-readme-md>) |
| `evidence/testing-strategy/distribution.md` | [Historical source section](<history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-testing-strategy-distribution-md>) |
| `evidence/testing-strategy/migration.md` | [Historical source section](<history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-testing-strategy-migration-md>) |
| `evidence/verification/2026-11-12/README.md` | [Historical source section](<history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-verification-2026-11-12-readme-md>) |
| `evidence/verification/cache-clearing.md` | [Historical source section](<history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-verification-cache-clearing-md>) |
| `evidence/verification/vendor-refresh-2026-11-11.md` | [Historical source section](<history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-verification-vendor-refresh-2026-11-11-md>) |
| `evidence/verification/vendor-refresh-2026-11-13.md` | [Historical source section](<history/doc-10 - Testing-migration-and-verification-operations.md#source-evidence-verification-vendor-refresh-2026-11-13-md>) |

## Historical links in tasks

Dated task implementation notes and recorded command strings remain historical. Use this manifest to resolve their former evidence/docs locations; current task documentation metadata points to canonical owners. Unavailable local captures can be retrieved from the recorded source revision.

## Migration validation and review — 2026-11-16

All four baseline SHA-256 hashes match the former HEAD inputs. All 929 formerly tracked evidence files remain on disk, Git tracks no evidence paths, and /evidence/ is ignored. No compiler baseline was regenerated.

Focused checks passed: five fixture ownership tests, two engine-reference tests, eight real dependency-patch checks, seven public schema/consumer tests six prerequisite-selection tests and sixteen packaging tests (44 focused tests total). Real C# exporter acceptance, real installed-engine C# ingestion acceptance and real Maven corpus repeat/relocation equality passed. The checker probes preserve explicit output overrides and structured failures; an incomplete historical JDT StoryEngine recording fails identically before and after this migration.

Final light checks passed: eleven changed Python files parse; four relocated JSON files parse; 315 local documentation paths/anchors resolve; git diff whitespace is clean. CodeScene reviews score all eleven changed source files 10.0; safeguard passed eleven eligible files with no issues. Independent reviewer found no remaining actionable findings after fixing duplicate root design prose and sanitizing workstation-specific documentation paths. Sonar remains inactive because the repository has no .sonar/config.json.

The user explicitly requested light checks instead of full verification. The first full attempt stopped at an unset private-Maven prerequisite. A configured retry passed prerequisites, syntax, bootstrap, Cargo check, MCP boundary and Clippy, then was interrupted during unit compilation at the user's request. Full verification did not complete and is not claimed as passing. No owned runner/compiler processes remained after interruption; category cleanup reclaimed its 1,459,680,432-byte workspace. The broad final cleanup retained an existing target/release directory (1,146,092,741 bytes) because native lsof gave an inconclusive usage result; no unrelated processes or artifacts were removed. No additional full run was started.

Local run details are retained under target/verification/task66-focused/, task66-full/ and task66-full-configured/. Logs/reports remain ignored rather than being copied into Git. Task TASK-66 contains the action log and accepted verification scope.
