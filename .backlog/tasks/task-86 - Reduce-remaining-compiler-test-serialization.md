---
id: TASK-86
title: Reduce remaining compiler test serialization
status: Done
assignee:
  - '@codex'
created_date: '2026-11-17T20:40:00Z'
updated_date: '2026-11-17T23:24:00Z'
labels: []
dependencies:
  - TASK-80
  - TASK-85
type: enhancement
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After the first bounded-parallel scheduler, matching Rust integration dropped from885.473to384.107s, but Rust acceptance still took234.938s. One nine-case CSharp group alone took224.59s because each whole binary/environment group occupies only one worker. The CSharp flow-ingestion CLI gate still took343.937s with independent top-level fixtures executed sequentially. User requested further optimization after committing Slice2 (8096a4b).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Measured critical paths and safe parallel boundaries are recorded from actual current runs
- [x] #2 Independent cases within large Rust boundary groups use the existing global four-job budget while unit and serial/native/performance semantics remain correct
- [x] #3 Independent CLI acceptance fixtures run concurrently only when project and mutable-resource ownership are isolated
- [x] #4 Selected test identities, assertions, diagnostics, cancellation and cleanup coverage are retained and matched timings demonstrate the result
- [x] #5 Late signal denial for a terminated group is accepted only with positive native absence evidence; live or unverified cleanup remains failed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Inspect actual per-group timings and fixture roots. First make Rust boundary scheduling granular enough to fill idle slots without increasing the four-job cap; preserve unit batching and serial/native/performance exclusions. Add a real same-binary protocol regression before changing execution, validate through the deterministic tooling selector and commit a small checkpoint. Then inspect the five CSharp CLI fixture operations, preserve sequential mutation/fault sequences within each fixture, and parallelize only independent top-level cases with isolated command records and the same compiler budget. Prove fixture isolation and failure draining, run computed tooling checks, then run the deterministic application comparison against the previous engine baseline. Keep all selected coverage, compiler authenticity, native/performance exclusivity and cleanup. No timeout reductions, cache-retention exceptions or project-specific overrides.

Before retaining Unit1, diagnose the repeatedly observed existing CodeScene timeout-cleanup flake that blocks complete tooling verification. Keep fail-closed live-group permission behavior and all correctness timeouts. Add deterministic real exited-group/forced-denial regression, preserve exact signal/errno diagnostics, and accept an already-gone group only with independent positive absence evidence. No blind retries or treating denied cleanup as success. Record this small reliability checkpoint separately if a fix is warranted.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Measured current logs: Rustacceptance234.938s, of which onecsharp_inputs nine-case invocation224.59s (~96%criticalpath); othergroups linked_sources71.70s,csharp_flows69.10s. Integrationtopgroups includeprivateJava duplicate98.55s, entry_metadata98.11s, root11cases95.20s, AspNet5cases90.75s, CSharp policy4cases87.76s, root239cases62.82s. Planner recommends splitting only independent boundary groups into singletonjobs under existingfourworkerpool; keepunit/native/performance/serialgroupsbatched. Testerfirstsamebinary+sameenv6case barrierred underway, productionunchanged. CLIownershipreview found distinctbaseline roots, work/linked andwork/mixed-linked, but shared Client.records and scratchnames/fault-bin require explicitper-case workroots andClientrecordlists. Preserve allwithin-case nativeanalysis/source-edit/delete/restore/historical/fault sequences, deterministicrecordmergingandpost-draincandidateidentitycheck. CLIunit2notstarted beforeunit1verification. Reusedisolatedcheckout resetto8096 onlyafterprovingpreviousstagedpatch matchedcommitted8429963 andnostrayedits; ignoredlogs/cachespreserved.

Unit1red changes-wyeqdf9v observed exactly the new same-binary/same-environment six-case probe timeout under old one-process grouping; other integration tests passed (274ran,1optionalcompilerjourneyskipped), cleanup passed. Added actual command/PID assertions to existing probe to preserve multi-case unit/native/performance/serial batching, not copied list-shape logic. Minimal productionhelper now expands only independent boundary rows into singleton RustGroup jobs; globalfour-workerexecutor, chain failure/drain/log behavior unchanged. SourceCodeHealth10.0; CLIunit2untouched. Finaltestbytes andgreencheck next.

Unit1green changes-21z5zs_c passed all6schedulerregressions including samebinary6cases and retainedbatch PID/args assertions, but entire plan again failed existing QualitySession timeout: os.killpg PermissionError masked timeout as cleanup-unavailable. Occurred before newRustprobes under unchanged Python serialrunner, outsideSandbox; 274Pythoncases1optionaljourneyskip, categorycleanup passed. Same failure had occurred in85green; previous retries passed, so stopped blindreruns and investigating group lifecycle transition. Rootnativeinterruption also reportedlatekillpermission but recordedPIDgone andsafetycleanupretrypassed. Allstrictcleanup/timeoutsretained. No appenginechangesafter8096.

Cleanupdiagnosis primarysource: AppleXNU kern_sig.c lines1612-1621 filtersSZOMB inprocess-group iteration andreturnsPOSIXEPERM whennofound members, consistentwithobservedlate-killrace (currentcasecauseinference). Deterministic red changes-85c0aoag failedonly forcedlateSIGKILL-EPERM afteractualTERM/reapedgroup; liveSIGTERM-ignoringchilddenial stayedfailclosed; all6schedulerprobespassed. Testfixturecooperativereapnowconfig-gatedonlyforcedcase; originalnormalorphan-transition scenario preserved. Sharedverification/process_groups.py fallbackrequiresreapedleader, PGID>1, complete successfulnative /bin/ps parse withobserverPID, waitsbounded1sec untilZEROgroupmembersincludingzombies; unknown/nonzero/timeout/malformedfailclosed. MCP/Runner callonlyafterPermissionError; initiallive-denialremainsstickyfalse. Operation/signal/errno/probestate diagnostics added. GroupStateenum andstandalonequality.py siblingimportreview fixes applied. Production3filesCodeHealth10.0; strictparser, unavailableproof andexternalcwdCLIhelpprobesbeingcompletedbeforegreen. No timeoutreductions, blanketEPERMignore or appenginechanges.

CombinedUnit1+cleanupfix intermediate green changes-vtkg8yuf passed19/19stages (all6schedulerprobes, strictparser, timeoutnormalorphan/lateEPERM/live-denial/unknownproof, CLIexternalcwdstartup,CodeHealth). Rootpost-greenreview found testfixtureownershipbug: liveignoredchild cleanupregisteredby mutablePIDfiles, subsequentunknownprobe overwrote filesbeforecleanupcouldrun. This green is intermediate, not finalcheckpoint. Fixing immediateper-probe try/finally cleanup withimmutablechild/leaderidentity, PGIDownershipcheckandactualchild-exitwait beforeotherprobe canoverwritefiles. No productionbusinessbehaviorchanges; finalstabletoolingselector rerun required. Cleanupfailure/live-denial guarantees unchanged; no blindretry or testweakening.

Final reviewed Unit1 checkpoint changes-3v963x29 passed all19 selected stages after immutable-PID finally cleanup correction:81unit and274integration passed; one optional existing CLI/TypeScript journey explicitly skipped (no candidate environment), not claimed passed. All6 scheduler probes, retained-batch PID/arguments, normal orphan transition, forced exited-group EPERM, live ignored-TERM denial, unavailable native proof and real standalone CLI startup passed. CodeScene passed; both category cleanup reports errors=[] and retainedbytes0. Unit2 CLI concurrency and matched application timing remain pending; TASK86 remains InProgress.

Unit1 checkpoint committed ba5e715 after7/7eligible source safeguard passed. CLIUnit2 now extracting a cohesive ordered-case ownership helper first, preserving sequential production behavior for meaningful scheduling red. Five real top-level fixtures retain within-case mutation/recovery sequences; separate Client records/environment/workroots, bounded4 workers, deterministic merge and failure draining. Selector registered CLI gate adds engine acceptance; final comparison will include committed engine baseline via --base8429963, without manual scope override.

CLIUnit2 meaningful red changes-le2d34i9 observed both new real child-command scheduling probes fail at four-case rendezvous under sequential helper (only1case started); otherintegration checks passed,1optionalCLIjourneyskip.9stagespassed1failed16laternotrun; selectedcategorycleanup passed. Earlier changes-0xqva283 was testsetup failure (missing workroot), corrected; not credited as behavioralred. Enabling boundedparallel after thisred, retaining actual5case compilerjourney as acceptance evidence. Green comparison planned --base8429963.

CLIUnit2 code and independent review complete: ingestion-local stdlib executor <=4, worker-set failureEvent before future error, bounded submission, pending cancellation and context-manager drain; per-case Client/env/work ownership, deterministic reports and all5 original callbacks/assertions retained. Candidate SHA check afterdrain on success/failure; secondary identity failure attached withoutmasking originalfailure. Production/test sources CodeHealth10.0 and diffcheckpassed. Preparing final matched --base8429963 verification with baseline environment, no project-specific Maven overrides; actual measuredgreen/preservedidentity comparison pending.

User steering: stop further optimization after current measured checkpoint, add agent guidance to favor parallel-safe test design, commit and resume remaining continuous-analysis slices. Root AGENTS.md now requires per-case mutable resource/environment/process ownership, no order dependence, coherent sequential recovery scenarios, explicit supported serialization metadata when necessary and runner-owned budgets/native-performance exclusivity. Documentation-only content/path/diff verification pending with checkpoint; no extra runtime suites for guidance.

Final matched verification changes-wmwd6yi3 (--base8429963) retained every baseline1241testregistration and all12gates; added357toolingregistrations, none removed. All common app integration, new scheduling probes, packaging, MCP independent install, Rustacceptance, automaticrefresh and CSharpcleanup gates passed. Rustintegration384.107->370.544s (~3.5%); Rustacceptance234.938->172.927s (~26.4%). Compiler times essentially identical (unit66.228->66.252, integrationCLI28.661->28.808, integrationRust119.013->119.054s). ToolingPython and optionalrealCLIjourney passed (282tests0skips). No CLIpool timing claimed: desktoplocked. Stop raced desktopgate startup:61interrupted14.686s duringfixturepreparation, beforethemedirectory/app.log/WindowLease/GUI launch;62CLIingestion andlaterchecksnotrun. Truthful finalstatuses62passed1interrupted13notrun. Initialcategorycleanup passed errors=[]; nativePSpositiveabsence acceptedlateSIGKILL-EPERM, ownedrunscanempty. Acceptance diagnosticfixture intentionallyretained96,291,410bytes at isolatedtarget/verification/changes-wmwd6yi3/acceptance/gate-csharp_desktop_acceptance/retained-fixture; Rustoutputsreclaimed. No claim wholeplan green. Userexplicitlyaccepts lockeddesktop and blockedlater checks deferredfinalSlice6. RootAGENTS guidance staticselector changes-oh7zvsib passed; exact3CLIpatch transferredroot; sourceCodeHealth10 androot safeguard2/2passed. Stop further optimization peruser, returnSlice3.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Scheduled independent Rust boundary tests and C# CLI acceptance cases within a four-worker budget. Preserved sequential recovery scenarios, ordered command records and failure draining, and added agent guidance for parallel-safe tests. Matching Rust integration improved by 3.5% and Rust acceptance by 26.4%; compilation times stayed unchanged. All runnable checks through C# cleanup passed. The desktop gate was interrupted before GUI launch; the real CLI parallel journey and later selected checks remain explicitly deferred to Slice 6 while the desktop is locked. Further test optimization is stopped as requested; Slice 3 is next.
<!-- SECTION:FINAL_SUMMARY:END -->
