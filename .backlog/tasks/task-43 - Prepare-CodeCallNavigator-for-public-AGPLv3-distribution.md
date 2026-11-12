---
id: TASK-43
title: Prepare CodeCallNavigator for public AGPLv3 distribution
status: In Progress
assignee:
  - '@codex'
created_date: '2026-11-12T04:55:00Z'
updated_date: '2026-11-12T05:26:00Z'
labels: []
dependencies: []
references:
  - https://www.gnu.org/licenses/agpl-3.0.html
  - >-
    https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/about-community-profiles-for-public-repositories
type: chore
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for an intentionally public, solo-maintainer project owned by Zsolt Gajdacs. User selected AGPLv3, public structured bug reports and feature requests only, private GitHub vulnerability reporting, downloadable macOS builds first and possible Linux builds later. No outside patches, GitHub Discussions, support guarantees or implementation promises. Third-party code retains its own licenses. Existing README and THIRD_PARTY_NOTICES.md need project licensing, issue forms, contribution/security guidance and verified binary-distribution obligations. Proposed SPDX identifier: AGPL-3.0-only. Planning only; implementation and remote settings remain for review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Standard AGPLv3 text, project copyright, consistent Rust package metadata and README licensing information distinguish project code from third-party licenses.
- [x] #2 Structured bug and feature forms are available, blank issues are disabled, privacy reminders are included and no response or implementation is promised.
- [x] #3 Bug reports capture version or commit, install method, OS, relevant analyzer/toolchain, reproduction and expected versus actual behavior, with optional logs and examples; feature requests capture problem, desired behavior and optional alternatives.
- [x] #4 Contribution guidance accepts bug reports and feature requests only; no outside code or documentation submissions, support requests or GitHub Discussions.
- [x] #5 SECURITY.md directs vulnerabilities to enabled GitHub private reporting with best-effort handling and no guaranteed response or fix.
- [x] #6 macOS release packaging includes project and dependency notices and verified access to matching corresponding source, build scripts and upstream patches; dependency compatibility and redistribution obligations are reviewed before release.
- [x] #7 GitHub checklist records public visibility, public issues, collaborator-only PRs, Discussions disabled, description/topics, community-file choices, secret scanning/push protection, Dependabot alerts and suitable code-scanning options; deferred and unavailable items are explicit.
- [x] #8 Verify issue-form syntax, links, metadata and packaged licenses/source; do not claim Linux support before implementation and testing.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Review the diff for scope and report local, packaged and live GitHub verification separately.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add AGPLv3 attribution and package metadata, public bug/feature forms, contribution/conduct/security guidance. 2. Extend packaging to preserve project notices and implement a validated matching-source release workflow using exact source inputs and dependency provenance. 3. Apply public issue, collaborator-only PR, private reporting and security settings; record the GitHub community checklist. 4. Run behavioral packaging/source tests, artifact inspection, CodeScene reviews and required Rust checks; record local versus live evidence.

5. User authorized commit and push: run the pre-commit safeguard, publish the approved changes to the empty public default branch main, verify issue forms/license/community files on GitHub, and record live evidence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-12: User approved DRAFT-4; promoted to TASK-43. Planner inspected license metadata, packaging and source-distribution risks. GitHub connector confirms public visibility and admin permission; gh authentication works outside the network sandbox.

Implemented AGPLv3 metadata, issue forms and solo-maintainer policies. Live GitHub: public; public issues; collaborator-only PRs; Discussions off; description/topics added; private vulnerability reporting enabled and verified; dependency alerts, secret scanning and push protection enabled. Added source archive workflow, source locks and package provenance. Initial packaging/source tests pass. Reviewer identified stale helper output and incomplete NuGet notices; release now rebuilds bootstrap helpers and preserves MIT text, copyrights and third-party notices. CodeScene initially flagged complexity; refactored affected functions.

Validation: planner/explorer/implementer/reviewer/tester roles completed. Reviewer blockers fixed and independent 14 packaging + 6 source tests passed; added epoch-timestamp regression after real engine ZIP failure, now 15 packaging tests. Full earlier Python run: 86 passed; final expanded run pending. Cargo check, regular desktop all-target tests and workspace Clippy passed. Required debug include-ignored run failed on three release-only canvas benchmarks and a process-group test; isolated process-group test passed outside sandbox. Saved/GPU benchmarks require a copied profiling workspace as well as release mode. Isolated committed snapshot release rebuilt helpers/app, produced all assets, verified 23,613 source hashes and all artifact SHA256s, restored all three Git bundles and reapplied patches, verified app signature and engine launch. No working-branch commit or push. GitHub source repository has no branches, so forms/license/community detection await default-branch publication; code scanning is explicitly deferred until language detection is possible.

Final checks: full Python suite 87/87 passed; both real packaged_java acceptance tests passed. Review/test evidence is evidence/public-repository.md; live settings checklist docs/public-repository-checklist.md. AC2 remains pending live issue-chooser verification after default-branch publication. Task remains In Progress because source has not been published and the mandatory debug include-ignored command still fails on release-only benchmarks; no waiver claimed. All source changes are uncommitted and no push/release upload was performed.

User requested commit, push and live GitHub verification. Pre-commit safeguard initially failed tools/package.py mean complexity 4.04; split installed-native-dependency validation into a named operation. CodeScene now scores 10.0 with no findings; 15 packaging and 6 source tests pass; pre-commit safeguard passes across all 4 eligible files. Remote main is configured as default but no branches exist; publish HEAD to main without force.

Published fcc74a8d0ef2f53ae8a79651e52363eaf432d812 to default main without force. GitHub public UI marks all intended community files and issue templates Added; license detected AGPL-3.0 and displayed GNU AGPLv3 summary. Both YAML form previews render required/optional fields and notices correctly. All eight remote community/template blobs match commit. GraphQL isBlankIssuesEnabled is viewer permission (owner true), consistent with the documented maintainer exception to config false. Private reporting/security policy and public issue/collaborator-only PR/Discussions settings verified. Composer required browser login, so no issues/reports submitted. AC2 verified through public previews/community detection; task remains In Progress solely for the prior unwaived full ignored Rust-suite gate.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented and published AGPLv3 licensing, public bug/feature forms and solo-maintainer policies, matching-source macOS release packaging and GitHub settings. Implementation commit fcc74a8 is live on default main. GitHub detects the license and community files; both form previews render correctly; published blobs match. Four-file pre-commit CodeScene safeguard and 21 focused packaging/source tests pass. Prior 87 Python tests, regular Rust checks/Clippy, packaged Java acceptance and isolated source release evidence retained. No test issues, release assets or security reports published. Full ignored Rust-suite benchmark prerequisites remain unwaived; task stays In Progress.
<!-- SECTION:FINAL_SUMMARY:END -->
