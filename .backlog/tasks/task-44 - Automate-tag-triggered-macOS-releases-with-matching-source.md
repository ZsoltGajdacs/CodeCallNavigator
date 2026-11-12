---
id: TASK-44
title: Automate tag-triggered macOS releases with matching source
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-12T06:04:00Z'
updated_date: '2026-11-12T06:18:00Z'
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
- [ ] #1 Creating a tag whose commit belongs to main triggers an Apple Silicon macOS build; off-main tags and tag updates do not publish releases.
- [ ] #2 The runner installs compatible toolchains and exact reviewed native dependencies, downloads checksum-verified corresponding sources and invokes the existing release packager from a clean checkout.
- [ ] #3 Successful eligible tag runs publish the macOS application ZIP, engine ZIP, matching complete source archive and SHA256SUMS together; failures do not publish a release.
- [ ] #4 A manual verification run builds and uploads artifacts without publishing a release; workflow syntax, tag guards, setup helpers and artifact contents are verified with recorded local/live evidence.
- [ ] #5 Documentation explains runner/platform, tag/main semantics, release assets and manual verification; no signing or Linux support is implied beyond existing capabilities.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Run applicable CodeScene safeguards, focused tests and a terminal GitHub verification build; report unavailable gates honestly.
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
<!-- SECTION:NOTES:END -->
