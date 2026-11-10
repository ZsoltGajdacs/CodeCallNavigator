---
id: TASK-25
title: Update vendor libraries and verify integration
status: Done
assignee:
  - '@codex'
created_date: '2026-11-10T09:02:00Z'
updated_date: '2026-11-10T09:08:00Z'
labels: []
dependencies: []
type: chore
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Refresh vendored upstream repositories while preserving existing local edits and reproducible revision pins; assess required CodeGraph integration changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Vendor default branches checked and available updates integrated without losing local edits
- [x] #2 Dependency pins match checkouts and analyzer patches remain valid
- [x] #3 Build and integration checks pass or limitations are recorded
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Fetch and inspect upstream changes. 2. Preserve Rune local edits and update its pinned revision. 3. Resolve required integration and lock changes. 4. Verify build, tests, analyzer acceptance and dependency source checks.

5. Adapt source-highlighting test assertions to the new lexer coalescing adjacent same-color ranges (for example, export const plus spaces); keep production scanner behavior and update its obsolete upstream-limit comment. Verify full expected byte coverage and reject partial or wrong-color matches.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fetched all three upstream defaults. CallAtlas and java-callgraph2 are unchanged at their pins. Rune advanced 147 commits from 5526e3fc to 7ed827a2. Preserved all 1171 pre-existing modified Rune files in stash 867111b258952e943db7fad7eeb8c6cd30fa7489 (codegraph-preserve-before-vendor-update-2026-11-10) and /tmp/codegraph-rune-pre-update.patch; left them unapplied to the new checkout. Updated analyzers.lock.json and Cargo.lock (new Rune highlighter/image dependencies). cargo check --all-targets passes without CodeGraph source changes; exact dependency source/patch provenance passes. Python 49/49 and TypeScript 46/46 pass. Full Rust integration suite and local packaging are running.

Planner/explorer confirmed the five syntax failures are test boundary assumptions: new Rune emits one Keyword highlight for export const plus spaces instead of separate exact-word spans. Production colors are correct. Full integration tests are continuing independently. Local app packaging and a native Metal class-tree snapshot both succeeded.

Implemented minimal syntax integration: assertions now verify requested paint covers every expected byte, accepting upstream merged spans while negative neighbor/partial-phrase cases still fail correctly. Production highlighter unchanged; cap comment updated. Implementer and independent reviewer/tester each ran focused 10-test suite successfully; review found no actionable issues. CodeScene re-review 10.0, no findings; pre-commit safeguard passed for 1 eligible file. Sonar inactive (no .sonar/config.json). Clippy passed; final post-adjustment run pending. Packaging completed at dist/CodeGraph.app and CodeGraph.zip, signatures verified by packager. Native Metal class-tree snapshot visually inspected successfully. Broad test command initially exposed missing private billing inputs, then five lexer span assertions; final suite skips both recurring_billing cases because external private-project inputs are unavailable.

Recovery: preserved Rune edits remain in its Git stash named codegraph-preserve-before-vendor-update-2026-11-10 (object 867111b258952e943db7fad7eeb8c6cd30fa7489); independent binary patch /tmp/codegraph-rune-pre-update.patch and status /tmp/codegraph-rune-pre-update-status.txt. They are intentionally unapplied to the updated revision. Final Clippy and codesign --verify --deep --strict dist/CodeGraph.app both pass. New Rune lexer powers TypeScript previews through the existing API automatically; switching the custom Java/C# literal scanners to upstream lexers would be optional separate work, with no need for this refresh. No runtime API migration needed for current CodeGraph controls. Existing ignored JDT LS and Lombok investigation artifacts are outside the active vendor Git repositories and remain unchanged.

Final full command cargo test --locked --all-targets -- --include-ignored --skip recurring_billing completed successfully: 299 top-level tests passed (300 result counts if nested terminal-child re-execution is included), 0 failed; two external private billing cases filtered. Python 49 and Node 46 pass. Final Clippy passes. Review and CodeScene pass. All validation complete; logs retained in /tmp/codegraph-vendor-*.log. Local commit prepared; no push requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Updated Rune by 147 upstream commits to 7ed827a2, refreshed both locks, preserved existing vendor edits in a named stash and backup patch, and adapted highlighting tests to coalesced lexer spans. CallAtlas/java-callgraph2 already current. No runtime integration changes needed. Rust 299, Python 49 and TypeScript 46 tests pass; two private billing cases excluded for unavailable external inputs. Clippy, CodeScene 10.0/safeguard, native UI snapshot, source provenance and signed macOS packaging pass. No Sonar configuration. Changes committed locally only.
<!-- SECTION:FINAL_SUMMARY:END -->
