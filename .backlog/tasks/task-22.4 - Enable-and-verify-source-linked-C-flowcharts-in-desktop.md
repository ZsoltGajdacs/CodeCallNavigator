---
id: TASK-22.4
title: Enable and verify source-linked C# flowcharts in desktop
status: Done
assignee:
  - '@codex'
created_date: '2026-11-14T11:49:00Z'
updated_date: '2026-11-14T21:39:00Z'
labels: []
dependencies:
  - TASK-22.3
references:
  - src/flow_workspace.rs
  - src/inspector.rs
  - src/flow_view/details.rs
  - src/ui_tests/function_flows.rs
  - docs/function-flowcharts.md
  - evidence/flowcharts/README.md
documentation:
  - docs/plans/task-22-function-flowcharts.md
parent_task_id: TASK-22
type: feature
ordinal: 22040
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The desktop already consumes CLI-built charts and owns layout, navigation and source highlighting. Finish C# availability and user-facing limitations through this path, without duplicating flow construction in the UI. This is the first complete user-visible milestone for TASK-22 and must be independently shippable before explanations, comparisons or recursive inlining.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Selecting a supported C# method in a real analyzed workspace opens its cached-source-linked chart through the installed CLI; step selection highlights the correct source and target navigation distinguishes overloads and uncertainty.
- [x] #2 Native acceptance verifies target navigation and Back, independent camera/scroll history, stale-generation rejection, older-cache refresh guidance and minimum-window layout.
- [x] #3 Partial/unsupported C# behavior and absent bodies are explained visibly; unavailable actions do not imply a complete chart is available.
- [x] #4 Packaged/relocated analyzer and Graphviz resources produce a real C# chart; record refresh/artifact overhead and selected-chart latency on named fixtures without benchmarking the live application store.
- [x] #5 Supported C# semantics and limitations are documented; full verification, change-specific light/dark native evidence and source Code Health checks are recorded with unavailable gates or baseline failures identified.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Record focused regression evidence and ./verify-full.sh results outside the macOS sandbox with isolated profiling inputs; report missing gates and baseline failures precisely.
- [x] #2 Review AI-modified source with CodeScene; run safeguard before commit and change-set analysis before PR, reporting unavailable checks.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Use current production C# capability and CLI artifact path; no redundant language switch. Add tests-first UI regressions for synthetic unlocated-step scroll/highlight behavior and known body-unavailable inspector actions; make bounded presentation fixes.
2. Add a real .NET BOM/CRLF/Unicode fixture with overloads, branches, cleanup, uncertain dispatch and unavailable/partial methods. Exercise actual CLI-backed controls, snapshot source, target/Back and independent history, stale generation, old artifacts and minimum-size layout.
3. Add opt-in bounded native acceptance following existing compositor pattern, ordinary-launch no-op. Require real native Drawn frames, normal isolated saved-descriptor boot and production CLI; use actual controls/input and capture only owned window. Keep input driver, reporting and external packaging harness separate.
4. Exercise relocated packaged engine/Graphviz with overrides scrubbed. Record named fixture refresh/artifact sizes and chart latency separately; do not infer overhead without comparable baseline. Inspect light/dark owned-window images and report unavailable native prerequisites honestly.
5. Review/refactor touched modules, CodeScene source/safeguard, full verification outside sandbox with isolated profile. Map each AC to focused/offscreen/native/package evidence before closing.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation timing clarification from current-code audit: production desktop already consumes CLI flow_languages capabilities generically. When TASK-22.2 truthfully adds C#, the inspector action becomes available without a separate desktop source gate. This task therefore verifies and refines the native C# workflow, unavailable states, labels and packaged resources after semantic validation; do not invent a redundant language switch merely to claim an enablement code change. Keep this task as the user-visible C# acceptance milestone and leave parent AC1 open until its evidence is complete.

Read-only preparation while22.3full runs: production C# capability already enables inspector; no duplicate language gate. Reuse ui_tests/cli_backend real analyze_existing/desktop/settle for actual C# CLI artifact. Two bounded UI refinements: selecting unlocated0..0 synthetic steps must preserve source scroll and show derived/unlocated explanation; disable Open flowchart only for known node.body_unavailable with reason, leaving old/ambiguous inventory decisions in CLI. Existing --snap/--tree captures are GPU offscreen even with --native (Rune headless policy), so do not label them displayed-window proof. Real native acceptance can follow pan_profile opt-in Compositor<State> pattern on actual Frame.rt, queue pointer events via controls, boot isolated saved descriptor, use --native --trace without snap/tree, capture owned OS window if available. Verify packaged relocation with scrubbed engine/resources overrides. Check BOM/CRLF/Unicode source, overload targets, Back/camera/scroll, zero spans, unavailable/old caches, stale cancellation and minimum-size light/dark; distinguish focused/offscreen/native/package evidence explicitly.

Concrete native API audit: App::on_ready is not invoked by headless tooling and provides WindowHandle::window_number (owned macOS CGWindowID), placement and surface_stats presents. Capture that owned window with screencapture -x -l during held-ready phase; require Drawn/presents plus image, not on_ready alone. Compositor<State>::render draws frame.draw_ui then queues public Runtime pointer/key/wheel input for next native frame; Runtime find/elements expose real text/help/visible/parent/fill. With no acceptance env/config, attach returns App unchanged. Native invocation uses --native --trace --theme light|dark --size1050x680, no snap/tree/video/strip and no synthetic scene for saved-descriptor boot. No upstream UI API patch or Quartz process/window enumeration needed.

TASK22.3 completed after full-bkd4h8bc passed34stages and committed as verified cleanup milestone. Starting desktop refinements and C# acceptance. Native API/normal boot audits prepared by planner/explorer are read-only and identify public window_number/on_frame_drawn APIs; no upstream Rune patch needed.

First bounded presentation unit: tests-first11tests initially9pass/2expectedfail (unlocatedscroll120→0; knownbody-unavailable action enabled), then11green. Four touchedsourcefilesCodeHealth10; independentreview no blockers. ActualCLI fixture UTF8BOM/CRLF/Unicode preflightgreen: exactsource and correctcompilerqualified Run/Leaf overload IDs; firsthelperred was bool/int alias lookup corrected to System.Boolean/System.Int32. HeadlessrealCLI UIbatch2pass/1testexpectationred: authoredabstract node has body_unavailable=false (field denotes emptyfile), so testcorrected to enabledrequest→CLIflow_unavailable, preserving CLIdecisionownership. Pre-feature6071db0 CLI+pinnedanalyzer rebuilt in isolatedtmp from recordedpatch;3alternatedwarmrefreshsamples afterwarmup preservecanonicalgraphprojection. Baseline median6.767s/artifact17288B;candidate bff3d27 median7.772s/artifact24169B. Completefeaturedelta ~1.006s/+6881B on namedtinyfixture, no significance/generaloverheadclaim; raw samples/identities/commands in evidence/flowcharts/csharp/desktop/refresh-overhead.json. Nativeprobe/harness work remainsunderreview; no nativepassclaimed.

Frozen finalfocusedwork: realCLI C#desktop3/3releasegreen; controlledlatepublisher3testsgreen (sixisolatedReady/Error mismatches afteractualcompletionlanded plusacceptedpositivecontrol), sentinel targetcontrolretained. Independentreview approvesallunits/harness/probe/tests/benchmark. All17sourcefilesfreshCodeHealth10, safeguard17/17noissues. Nativefirst launch timedout all2displaysasleep; temporarycaffeinateawake retry resolveddisplaylink butsessionlocked preventsDrawnworkflow. LiveSDKdiagnosis savednative-prerequisites.json; noheadlesssubstitution/falsepass. Userasyncaskedunlock, requirednativegatepending. Firstfull-nsdys2gp failedwhitespaceonly becauseintentionalCRLFfixtureafter-text; specificwhitespace=cr-at-eol correctedwithoutchangingfixturebytes (stagedSHA15a596...ed784 exactBOM+139CRLF). Finalfull-qiaiih4v running43stages outside sandbox isolatedprofile; formatting/build/Clippy/ordinaryRusttests passedsofar. NoDoneor22.4commitclaimed.

User explicitly answered native prerequisite question: Keep native verification pending. This is a request to retain an open verification gate, not a native pass or waiver for Done. Continue the authorized independent CLI work after this full run while preserving22.4 native AC/status pending. No further display wake/unlock attempts unless user signals availability.

User later unlockeddesktop, superseding pending-nativeprerequisite. Realnative-unlocked passedsource/navigation/Back butactualviewport1147x1344largerthanCLIrequest, so NOTminimumproof; firsttimingalsoincludedgesture-settlefloor. Addedtests-firstrequiredviewport andnativeChromeInset guards, ownedresize+stableDrawn gate andfirstReadyDrawn meter beforeinputbusy. Native-minimum correctlyfailedAeroSpace delayedretiling aftercreation. NarrowPID+CGIDwindowsetup handshake nowfloatsONLYownednewwindow viaAeroSpace, no globalprefs/rules/existingwindows;9Pythonownership/cleanuptests and26Rustfocuscases pass, independentreviewapproves. FINALnative-minimum-managed passesBOTHthemes atEXACT1050x680content, actualDrawnsourcehighlightline116,intoverload/certainty/provenance,target/back,camera/sourceindependence,graphrestoration;imagesinspected. Rootfirstready136.4/119.6ms,callee54.8/53.0ms,2observationsnotguarantees. Zero-spanabsentfixture→nullnativecheck, separateUItestcoversit. RelocatedoffscreenrealCLIworkflow11.2spassedandimagesinspected, separatelylabeled. BothCodeSceneversionsclean aftercohesivequalityrefactor; finalfullrerunrequired beforeDone/commit. Evidence README preservesallfailed/correctedgates.

FINAL full-0tid44xs PASSED46/46 stages outside sandbox, isolatedprofile: Rust/Python/TS, actualcompileracceptance+C#32casetraces, releaseCPU/GPU/storagebenchmarks, packaging/signature, installedCLI/MCPjourneys,20sourceCodeScene/safeguard+cleanup. AC1=native-minimum-managed+realCLIUI3;AC2=native target/Back/camera/scroll/exactmin +separatelylabeledrealCLIoldcache/epoch andcontrolledlatepublisher;AC3=knownbodydisabled/CLIauthoredabstractunavailable/asyncpartial+unlocatedtests;AC4=actualrelocatednativebundledpaths+refresh-overhead.json+firstReadyDrawn;AC5=supportmatrix,inspectednativeimages,20sources10bothversions/full46. Sonarinactiveunconfigured. NoPRrequested→nochange-setPRanalysis. Nativeearlierpendingresolvedbyuserunlockandsuccessfulownedwindowrun, no waiver. Earlierfailed/static/quality/environmentrunsretained. Finaldocumentationlinks/syntax/diffchecked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Delivered source-linked C# desktop workflows through the existing CLI capability, with correct overload/certainty labels, cached source and independent target/Back history. Fixed unlocated-step scroll behavior, known unavailable-body actions and native title-bar overlap. Real .NET/CLI compatibility, controlled late replies, relocated resources and native light/dark1050x680 workflows pass. Recorded measured refresh/artifact delta and queued-input-to-first-Drawn latency with limits. Full46-stage verification and20sourceCodeScene reviews/safeguard pass. Native tests use synthetic Runtime input in real owned windows; hardware input/VoiceOver are not claimed.
<!-- SECTION:FINAL_SUMMARY:END -->
