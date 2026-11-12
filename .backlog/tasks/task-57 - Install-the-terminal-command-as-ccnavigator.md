---
id: TASK-57
title: Install the terminal command as ccnavigator
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T21:09:00Z'
updated_date: '2026-11-12T21:45:00Z'
labels: []
dependencies: []
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The public command is shipped in the standalone engine package but is unavailable on terminal PATH after extraction. Add explicit user installation of the complete relocatable distribution while preserving internal executable and package names.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Default and custom-prefix installation expose ccnavigator on PATH with matching analyzer resources independent of the extracted download.
- [x] #2 Updates replace only installer-owned installations and conflicts or failed installation preserve previous working commands.
- [x] #3 Document install, update, removal and PATH setup without changing shell configuration.
- [x] #4 Focused tests and installed real-compiler acceptance pass, with full verification and CodeScene results recorded.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Reuse the existing public executable and standalone distribution; add a staged user installer with ownership checks and rollback.
2. Add process-boundary installation tests and installed real-TypeScript acceptance to full verification.
3. Document installation and run focused tests, CodeScene and full packaging verification; preserve unrelated changes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented staged complete-distribution installation, installer ownership and conflict checks, serialized updates, rollback and retained recovery files on rollback failure. Initial 11 behavioral tests were written against the missing installer (red), then passed; review added resource/marker symlink refusal and failed-rollback coverage, all 14 pass. Independent review reports no remaining blocker. CodeScene reviews: installer 9.38 (overall complexity only), acceptance and existing modified source files 10.0. Sonar inactive: no .sonar/config.json. Full verification started outside sandbox at target/verification/task57-full.

Installed the assembled matching release distribution into ~/.local/share/CodeCallNavigatorEngine with ~/.local/bin/ccnavigator. Confirmed command -v, --help and capabilities from /private/tmp. The installed acceptance preview passed 21 real TypeScript CLI operations after removing its extracted download; report target/verification/task57-installed-preview.json. Full verification has passed ordinary Rust, 189 Python tests, 26 tool tests and TypeScript tests; release acceptance is still running.

Final verification: full run target/verification/task57-full passed all behavior, benchmark, packaging/signature, installed CLI and MCP stages (642 release Rust tests, 189 Python tests, 26 tool tests and TypeScript tests); its only failure was CodeScene mean complexity. Extracted PATH/reporting from command handling without behavior changes; independent review and all 14 installer tests passed again. Re-ran the exact failed quality stage outside the sandbox after an initial sandbox process-cleanup denial: target/verification/task57-final-quality.json passes all six reviews at 10.0 and the safeguard with six files checked. Regenerated the standalone package and reran installed acceptance: target/verification/task57-final-installed-cli.json passes 21 real operations after deleting the extracted download. Updated the actual ~/.local installation and verified terminal resolution/help. No package/crate/internal executable names changed. Changes remain uncommitted; nothing pushed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added and installed the ccnavigator user CLI installer with complete resources, custom prefixes, ownership/conflict checks, staged validation and rollback. Added installation/update/removal documentation and full-verification installed-command acceptance. All required behavior and packaging stages passed; the sole initial CodeScene failure was fixed and its exact gate rerun passed at 10.0. Final installed-package acceptance and 14 installer regressions pass; user terminal command is ready. No commits or pushes.
<!-- SECTION:FINAL_SUMMARY:END -->
