---
id: TASK-33
title: Fix Java lambda ownership during refresh
status: Done
assignee:
  - '@codex'
created_date: '2026-11-11T05:20:00Z'
updated_date: '2026-11-11T05:28:00Z'
labels: []
dependencies: []
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Processor-disabled attribution assigns stream lambdas from separate TaxReduction classes to java.util.stream.Stream$map, aborting refresh at strict cross-file validation. Follow-up to completed TASK-32.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Lambda owners and labels use enclosing class declarations, including nested and local types
- [x] #2 Real JVM unresolved-getter regression loads without cross-file failure and retains conservative replay diagnostics
- [x] #3 Strict validation includes owner and both files; adapter cache invalidation and packaging verified
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add failing real-JVM attribution regression. 2. Share declaration-only owner resolution and restrict related checks; improve Rust diagnostic. 3. Verify adapter cache fingerprint, Java/Rust regressions and packaged resources. 4. Retry existing billing outputs without rebuilding application; run CodeScene checks.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Regression-first: initial simple map lambda did not reproduce; chained filter lambda followed by unresolved generated-getter method reference failed with the exact java.util.stream.Stream$map owner. Added shared ClassTree-only LambdaOwner for site ownership, labels, label context and stable-type eligibility; retained strict Rust validation and added both paths to its error. Added adapter-content cache key and invariant test without changing kind_v8. Real-JVM Python correlation 12/12 and package Python 10/10 pass; normalized graph unresolved-reference regression and all unignored workspace tests pass. All 9 changed/new eligible source files score 10.0; CodeScene safeguard passes 9/9. Independent reviewer found no actionable findings. Full package, strict Clippy, focused integration acceptance and existing-output billing retry are in progress. Planning, exploration and implementation were performed in the main agent; reviewer and tester delegated under backlog-task-loop.

Final verification: strict workspace all-target Clippy passed; Java context Python 9/9, Java correlation/identity integration 7/7 and packaged-resource acceptance 2/2 passed. Full desktop/engine packaging and signing passed; all three adapter JARs have identical SHA-256. Actual packaged engine smoke produced 3 nodes and 2 portable stream-lambda identities. Saved successful billing metadata replay passed all 13 contexts in 324.80 seconds with 10827 declarations and 17331 relationships; no application source edits or rebuilds. The two TaxReduction owners now name their own ArticleType classes; unavailable processor-free replay remains diagnosed. Evidence and action log: evidence/analyzers/java-lambda-owner-refresh.md. No Sonar config; inactive. No push or PR requested.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed expression TypeElements being mistaken for lexical lambda owners by sharing declaration-only resolution across correlation, labels and identity checks. Preserved strict validation with actionable owner/file diagnostics; adapter-content hashes invalidate inventory caches without wire-format changes. Regression-first Java/JVM and normalized graph coverage pass, as do workspace tests, strict Clippy, packaging and CodeScene 9/9 safeguard. Existing-output billing retry passed all 13 contexts: 10827 declarations, 17331 relationships. Detailed evidence: evidence/analyzers/java-lambda-owner-refresh.md.
<!-- SECTION:FINAL_SUMMARY:END -->
