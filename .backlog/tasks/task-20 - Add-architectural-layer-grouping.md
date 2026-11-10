---
id: TASK-20
title: Add architectural layer grouping
status: Done
assignee:
  - '@codex'
created_date: '2026-11-09T19:28:00Z'
updated_date: '2026-11-09T19:47:00Z'
labels: []
dependencies: []
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expose architecture layers over the existing compiler-backed graph, reusing review-app configuration concepts while keeping analysis, classification and presentation separate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Native versioned configuration, presets and safe automatic review-config import classify all supported languages with semantic facts.
- [x] #2 Ordered bands, expansion, tabs and support lanes preserve filtered relationships, navigation and source access.
- [x] #3 Cached facts support configuration reload, invalid-config recovery and older-cache guidance.
- [x] #4 Regression, analyzer, UI and packaging checks plus CodeScene reviews verify the change and documentation.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Establish regression baseline and shared architecture types/configuration/classification. 2. Export and persist compiler-backed per-file semantic facts. 3. Add configuration lifecycle, selection and diagnostics. 4. Refactor grouping and implement ordered bands. 5. Verify real analyzers, UI, packaging and Code Health; document evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Established 18-test projection baseline. Shared native configuration/import/classification and cached declaration facts are implemented. Refactored grouping and matching into focused modules; new pure modules reviewed at Code Health 10.0. Real Java and TypeScript compiler checks passed; C# shared-source ownership issue found during review and being corrected. Flowchart follow-up recorded separately as TASK-21.

Completed compiler-owned per-file facts and SQLite persistence, including Java SOURCE annotations, TS transitive heritage and separate C# linked-file compile contexts. Independent review found and fixed nested imported path bases, dotted Java build roots, canonical repo paths, stale recovery warnings and hidden/tab edge accounting. UI testing found and fixed an animated canvas overlap intercepting tab clicks. Refactored all affected boundaries; final CodeScene safeguard passed for 56 eligible files. Final verification: 185 regular Rust checks, 37 additional real-toolchain cases, 48 Python checks and 31 Node checks passed; Clippy and package-scoped formatting passed. Two unavailable private-project environment fixtures are explicitly documented in evidence/architecture/README.md. Preserved pre-existing modified Rune vendor checkout; isolated clean-pinned-source packaging, signing and packaged Maven/classification smoke test passed. Four native Metal captures cover both themes and window sizes. Sonar inactive without local configuration.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added configurable architectural Layer grouping with ordered bands, parallel flows, support lanes, tabs, semantic compiler facts and cached reclassification. Native YAML takes precedence over safe one-time review-config import; presets, file rules, inspector provenance and headless classification share one architecture library. Refactoring separates input conversion, matching, compiler facts, workspace jobs, grouping and geometry. Documentation and verification evidence are under docs/architectural-layers.md and evidence/architecture/README.md. TASK-21 tracks function flowcharts independently. Applicable validation passed; two private-project fixtures and physical trackpad/VoiceOver remain unverified, and packaging used a clean pinned Rune staging copy to preserve the existing local vendor diff.
<!-- SECTION:FINAL_SUMMARY:END -->
