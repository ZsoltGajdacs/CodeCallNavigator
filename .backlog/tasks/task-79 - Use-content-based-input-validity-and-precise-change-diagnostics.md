---
id: TASK-79
title: Use content-based input validity and precise change diagnostics
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T14:02:00Z'
updated_date: '2026-11-17T17:56:00Z'
labels: []
dependencies: []
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Timestamp-only build rewrites currently reject otherwise consistent analysis. Slice 1 of approved continuous-analysis plan; existing generated-directory work is TASK-78.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Identical-content rewrites remain valid and preserved-metadata content changes invalidate
- [x] #2 Creation deletion membership and required links retain exact typed diagnostics
- [x] #3 Generic fresh-output and required-input regressions pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Remove metadata-only rejection from shared fingerprint classification while preserving serialized metadata and hash/link/missing semantics. Extend real filesystem tests with controlled mtimes, same-content rewrite and same-length changed content with restored mtime. Preserve typed exact diagnostics and existing generated/required-input cases. Run deterministic selected verification, review and Code Health before finalizing.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Exploration identified empty file/directory hash collision when dropping metadata comparison. New captures will retain explicit input kind; legacy fingerprints without kind keep conservative metadata checks until recaptured. Tests-only regression prepared; selected red verification is running with fixture TypeScript modules and private Maven checkout.

Red selector changes-155hg7rt stopped at existing typescript_output_conventions_do_not_hide_java_source_packages: TASK78 blanket dist exclusion hides src/main/java/dist/Worker.java. Timestamp regression not reached, so red failure for timestamps is not claimed. Correct generated handling using existing declared output_roots rather than a global dist name blacklist; retain existing authored-package assertion and explicit required-output-root coverage. Native/compiler integration groups before that failure passed; later selected stages not run.

Green selector changes-44j04zau has passed the strengthened fingerprint::tests::detects_edits_additions_and_deletions and retained typescript_output_conventions_do_not_hide_java_source_packages. Timestamp touch/same-byte rewrite, restored-mtime content edits and empty file/directory replacement cases passed. Compiler/Clippy/unit checks passed; later selected integration/acceptance stages still running. Code Health reviews and safeguard passed 3 changed source files.

Green run changes-44j04zau passed static/compiler/Clippy, all Rust/Python/Node integration and packaging checks; acceptance failed existing compiler_source_aliases_match_canonical_spelling_with_named_roots at normalized provenance input parity. Difference: 10 lexical project node_modules dependency probes present only for aliased source spelling, unmapped counts259 versus239. Reviewer traced to compiler.js unconditional lexical input persistence from earlier TS work; no baseline rerun claimed. Preserve strict acceptance assertion and required links; normalize dependency aliases deterministically using configured projects and physical probes as necessary cross-consumer compatibility correction. Later acceptance gates not run; cleanup passed.

User explicitly directed clean cut for the entire change: do not add compatibility for earlier states. Remove newly added optional fingerprint-kind and legacy metadata fallback; new fingerprints require kind. Subsequent session/protocol slices replace old contracts directly rather than negotiate fallback behavior. Current run changes-89r9qz35 predates this steering and cannot alone verify the clean-cut revision.

Clean-cut steering applied: FileFingerprint.kind is required, legacy metadata fallback removed, missing-kind JSON rejected, inventory registration renamed. Interrupted obsolete run changes-89r9qz35 by verified owned runner PID; runner initially reported process-group cleanup unavailable, but final integration category cleanup passed with errors=[] and no owned run-path processes remained in process table. New complete run launched for current contract. No pass claimed for interrupted run.

Clean-cut run changes-mqlej44l passed all unit/integration and packaging, then alias acceptance failed only limitations totals: portable inputs all244 now equal, unmapped249 alias versus239 canonical. Stop before another tracking workaround: identical total compiler-probe inventories across source spellings is a false premise. Actors: compiler observations, emitted portable inventory, test comparison. Preserve graph/portableinput/repeatbyte strictness; validate each limitation count independently against Complete.fingerprints and Complete.roots, selected-target mapping and required lexical/physical alias coverage, then exclude validated request-specific limitations from cross-request equality. Reviewer approved this coverage mapping; no fabricated probes/count normalization.

Run changes-s5h9kxo3 passed revised alias acceptance, exact runtime coverage/digest/alias assertions, all unit/integration and packaging. Next acceptance group found empty_external_referenced_project_scope_detects_new_sources_after_publication missing @imports/. Owning-source review: Preprocessor transient roots include project directory but Snapshot.paths omits it, so final publication loses empty external project scope. Carry same already-observed directory through returned paths and derive roots from that single list; retain failing acceptance unchanged and extend existing preprocessing test for both paths/fingerprints. Code Health 10.0 for both changed C# files. Later stages not run; no baseline attribution claimed.

Run changes-gzsrpjsy passed all unit/integration and Rust acceptance, including alias trace accounting and unchanged empty external C# project scope test; automatic_refresh_acceptance and csharp_cleanup_acceptance also passed. Native csharp_desktop_acceptance failed before behavior assertions: WindowLease could not find PID10695 window4191 in AeroSpace within setup interval. Native window-request and frame traces exist; native behavior not claimed passed. Later selected acceptance/performance/quality stages not run. Source Code Health reviews and safeguard passed 10 files. Asked user whether to defer native/later blocked gates to final Slice6 checkpoint or resolve native setup before continuing. No exception assumed; task remains In Progress.

Checkpoint evidence changes-gzsrpjsy: 49 stages passed, 1 failed (native window setup), 13 not run. All three behavioral acceptance criteria proven; task stays In Progress pending required remaining gates or explicit accepted deferral. Final source safeguard passed 10/10 eligible files with no regressions; individual reviews 10.0. Timings seconds: setup/static20.312; compilation/packaging435.942; executed tests/gates1283.715 (external gates include their internal setup); cleanup11.729. Sonar inactive: project has no .sonar/config.json. Saving a local implementation checkpoint, no push; no full-verification claim.

User explicitly accepted deferring the native-window gate and the checks it prevented from running to final Slice6 verification, and directed continuing engine slices. Behavioral criteria all passed; 49 passed stages and source Code Health recorded above. Remaining gates are deferred, not passed. Local checkpoint 990c30e; no push.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Content-based input validity with mandatory input kind; declared-output handling and required scopes preserved. Consumer regressions corrected and verified. 49 selected stages passed; native AeroSpace setup and 13 later checks explicitly deferred by user to final Slice6. Committed 990c30e.
<!-- SECTION:FINAL_SUMMARY:END -->
