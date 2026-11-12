---
id: TASK-44
title: Automate tag-triggered macOS releases with matching source
status: Done
assignee:
  - '@codex'
created_date: '2026-11-12T06:04:00Z'
updated_date: '2026-11-12T06:45:00Z'
labels: []
dependencies: []
type: chore
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The public project has a verified local macOS/source release packager but no GitHub Actions automation. The owner requests automatic GitHub releases when tags are created on main. Tags are not branches, so release eligibility must be checked against main history. Existing packaging pins native dependencies and preserves AGPL corresponding source; hosted runner setup must honor those inputs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Creating a tag whose commit belongs to main triggers an Apple Silicon macOS build; off-main tags and tag updates do not publish releases.
- [x] #2 The runner installs compatible toolchains and exact reviewed native dependencies, downloads checksum-verified corresponding sources and invokes the existing release packager from a clean checkout.
- [x] #3 Successful eligible tag runs publish the macOS application ZIP, engine ZIP, matching complete source archive and SHA256SUMS together; failures do not publish a release.
- [x] #4 A manual verification run builds and uploads artifacts without publishing a release; workflow syntax, tag guards, setup helpers and artifact contents are verified with recorded local/live evidence.
- [x] #5 Documentation explains runner/platform, tag/main semantics, release assets and manual verification; no signing or Linux support is implied beyond existing capabilities.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Run applicable CodeScene safeguards, focused tests and a terminal GitHub verification build; report unavailable gates honestly.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Inspect packaging and hosted runner prerequisites. 2. Add main-ancestry tag gate, pinned macOS setup and checksum-verified source download helpers. 3. Add tag publishing plus manual build-only workflow and release documentation. 4. Review and test guards/setup, run CodeScene and a live build-only workflow, inspect artifacts and record terminal outcome.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented new-tag trigger with main ancestry guard, manual main build-only dispatch, isolated write-permission publication, and draft-first asset uploads. Hosted target is documented xcode-27 Apple Silicon/macOS27 runner. Exact official Homebrew bottle digests verified by explorer against live GHCR bytes; provisioning installs the pinned native versions and all corresponding sources. Fourteen CI behavior tests and actionlint pass; CodeScene reviews of both CI helpers and their tests score10.0. Reviewing before pushing and running a live build-only workflow.

Independent review found no blocking issues. Tester added owned-draft retry, published-release no-overwrite, duplicate-checksum and missing-asset cases. All18 CI tests,15 packaging tests,6 source tests and actionlint pass. No local Homebrew modifications. Workflow/manual dispatch ready for hosted integration. Unrelated README.md and docs/reference.md edits are excluded from this task commit.

First hosted build-only run43166736110 reached native provisioning, then failed because xcode-27 ships Homebrew7.0.12. That version routes local bottle paths through a path-restricted generic loader. Explorer traced upstream fix31986b7 (released7.0.13), which adds explicit supported bottle dispatch. Fix will explicitly update Homebrew CLI and require>=7.0.13 before removing any native package; no developer/path-security bypass and no local Homebrew mutation. The publish job was skipped as designed.

Hosted corrected run43167349273 on commitacc7da9 succeeded. It used Homebrew7.0.13 and installed exact native bottles; bootstrap, all40 focused regressions, complete app/engine/source packaging, checksum/signature/engine-help checks and artifact upload passed. Manual dispatch correctly skipped publish; no tag or GitHub release created. Artifact13920893239 macos-release is372882894bytes with SHA2565378a6bc07fe3918508abc2fe51a1381ff1a0abee1a4d62c9eacd2826e979869. Downloading artifact for independent archive/source inspection. Evidence draft: evidence/ci/tag-release.md. Concurrent unrelated Rust UI edits are being preserved.

Independent artifact inspection passed. Downloaded artifact matches GitHub SHA256 5378a6bc07fe3918508abc2fe51a1381ff1a0abee1a4d62c9eacd2826e979869 and contains exactly four expected assets; all three inner archives match SHA256SUMS. App/engine source identities match tested acc7da9, with a clean source tree and exact native versions. All 23,613 source-manifest hashes match, including pinned dependency sources/upstream bundles. Project bundle restores the compiled commit; downloaded app signature verifies after extraction. Initial large transfer reset; resumable chunked retry completed without changing the artifacts. Publication ordering/retries are proven by behavioral tests; actual publish remained skipped in manual verification, and no test tag/release was created. Evidence: evidence/ci/tag-release.md. All acceptance criteria and DoD verified.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added and published a new-tag macOS release workflow with main-ancestry validation, pinned Apple Silicon/macOS 27 toolchains/native inputs, verified complete source bundling, and draft-first four-asset publication. Manual main dispatch builds without publishing. Implementation e85efd4 and Homebrew compatibility fix acc7da9 are on main. Hosted run 43167349273 passed all 40 focused tests, complete packaging, signature/checksum/engine checks and artifact upload; independent download inspection verified the GitHub digest, three archive checksums, 23,613 source hashes and exact source commit. CodeScene scores are 10.0 and safeguards/actionlint pass. No actual tag/release was created, and unrelated concurrent work was preserved.
<!-- SECTION:FINAL_SUMMARY:END -->
