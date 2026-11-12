# Public repository checklist

Reviewed 2026-11-12 for `ZsoltGajdacs/CodeCallNavigator`. This project is maintained
by Zsolt Gajdacs alone. Public contributions are bug reports and feature requests.

## Repository files

| Item | Decision |
| --- | --- |
| README | Build/run prerequisites, supported platform, limitations, license and participation links |
| LICENSE | Standard GNU AGPLv3 text; AGPL-3.0-only package metadata |
| CONTRIBUTING.md | Issues only; no outside patches, documentation submissions or support requests |
| CODE_OF_CONDUCT.md | Short conduct/moderation policy for issue participation |
| SECURITY.md | Private GitHub reports; best effort, no response/fix/bounty commitments |
| Issue forms | Structured bugs and features; blank issues disabled; private security-report link |
| SUPPORT.md | Omitted: support policy is in README and CONTRIBUTING; no support channel offered |
| Pull request template | Omitted: outside pull requests are not accepted |
| ACCESSIBILITY.md | Deferred: no separate accessibility-support commitment; specific bugs may use the bug form |
| Third-party notices | Preserve upstream terms, package-specific copyrights, permission text and notices |
| Releases | macOS first; exact corresponding source and binary checksums published together; Linux deferred |

## Live GitHub settings

Read back through authenticated GitHub REST API on 2026-11-12:

| Setting | Verified state |
| --- | --- |
| Visibility | Public |
| Issues | Enabled; no repository interaction limits; public issue creation |
| Pull request creation | `collaborators_only` |
| Discussions | Disabled |
| Description and topics | Added call-graph/static-analysis/macOS/Rust/developer-tools/AGPL information |
| Private vulnerability reporting | Enabled |
| Dependabot vulnerability alerts | Enabled (endpoint returns HTTP 204) |
| Dependabot security updates | Already enabled; retained |
| Secret scanning | Already enabled; retained |
| Push protection | Already enabled; retained |
| Code scanning | Default setup not configured; defer selection until source is on the default branch so GitHub can detect supported languages |
| Maintainer content reporting | Organization-only control; this is a personal repository. Conduct policy directs users to GitHub reporting where available |

At the initial pre-publication review, the GitHub repository had **no branches**.
Community files and forms had been prepared locally. After publishing the source
to the default branch, verify the
issue chooser, license detection, community profile, security link and code
scanning selection on GitHub. A custom conduct policy and intentional omissions
may leave GitHub's community score below 100%; the score is not a release gate.

## Release verification

Use [public-releases.md](public-releases.md) for the source/archive workflow.
Keep local unit tests, actual macOS packaging, source-archive checks and live
GitHub verification separate in task evidence. Do not describe an unrun check
as passing or claim Linux support before its own implementation and validation.

References: [GitHub community profile](https://docs.github.com/en/communities/setting-up-your-project-for-healthy-contributions/about-community-profiles-for-public-repositories),
[pull request controls](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/enabling-features-for-your-repository/disabling-pull-requests),
[private vulnerability reporting](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately),
[content reports for organizations](https://docs.github.com/en/communities/moderating-comments-and-conversations/managing-how-contributors-report-abuse-in-your-organizations-repository).
