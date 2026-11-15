# Compiler-backed cached expansion consumers

The final focused checkpoint passed **three distinct real SDK workflows**: Java
compiler integration, C# CLI acceptance and TypeScript CLI acceptance. All ten
prerequisite/setup/compile/test/cleanup stages passed. This is an exact inventory
subset, **not a whole engine category or full-verification pass**. Other tests and
gates are explicitly recorded as not run in the archived selections.

Each workflow consumes actual compiler output and the saved artifact's Ready
caller/callee views, verifies synchronous/direct/isolation facts, expands through
the production core API and checks connected call/return boundaries, source
ownership, exact bytes/digests and unchanged artifact bytes. C#/TypeScript also
retain their real public CLI cached-query journeys. No new public expanded-flow
CLI operation or native-window behavior is claimed.

Both owned categories used release workspaces and the production verification
selection/environment/prerequisite/execution/cleanup APIs. The acceptance targets
compiled together once. Integration compilation/test/cleanup took
24.418s/1.973s/1.047s; acceptance took 24.861s/14.450s/1.212s. Setup and prerequisite
times are separate in `verification.json`. All 45 reviewed source snapshot hashes,
including the four-file compiler-test freeze, matched before/after this run.

Owned builds and temporary workspaces were removed. Integration reclaimed
175,373,894 bytes and acceptance reclaimed 234,048,959 bytes. Acceptance's reported
178,727,250 retained bytes are the two previous-app rollback packages deliberately
preserved by cleanup policy, not leaked category outputs. No packages or binaries
are included here.

The first focused run remains failed historical evidence: Java passed, TypeScript
failed selecting a CallSite by the wrong label, and C# was not run after that
artifact failure. Both cleanups still passed. The corrected helper selects exact
**cached** source text at the compiler-owned span, then asserts the separate real
label. The precise TypeScript implementation signature was corrected from `leaf`
to the observed `(value?: number): number`; the latter diagnosis used retained
actual compiler data before reaching that assertion at runtime. The target was
an available implemented body, not an observed overload-binding defect. All three
workflows were rerun because their shared test helper changed.

The final negative core matrix kept the same test identity and now covers HTTP,
navigation, valid deferred activation and Deferred callee bodies with visible
ineligible stops, no child source/body and preserved full choices. Its refactored
category passed 174 distinct tests with owned cleanup. Old/modern scoped reviews
both score 10 after assertion-helper extraction. Earlier passing runtime and
failing old-CodeScene results remain preserved rather than relabeled.

`verification.json` is the final independent focused report;
`producer-verification.json` preserves the earlier producer freeze awaiting those
checks. Current and prior raw quality reports retain their original verdicts.
`category-report-index.json` hashes all 84 entries in `category-reports.tar.gz`;
every archived entry was read back and verified. The archive contains reports and
logs only, including first failure, final retry, both core checkpoints and the
actual TypeScript target observation.

This is a **prior source checkpoint**. A subsequent mandatory full run stopped at
a core `calls.rs` Clippy finding after cargo-check passed; later runtime stages
were not run in that attempt. Root owns the small correction and full retry.
These focused results do not claim that later whole-task sources are unchanged
or that full/native/package verification has passed.
