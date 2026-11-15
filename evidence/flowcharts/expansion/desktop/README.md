# Optional call expansion desktop delivery

TASK-22.8 exposes shared bounded call-site expansion through an opt-in CLI query
and desktop controls. Ordinary single-function Flow remains the default.

TASK-22.8 is verified on the corrected source: focused compiler/CLI/Rune,
packaged native acceptance, both Code Health variants and the mandatory full
run passed. The first full attempt remains recorded as a Clippy failure.
See the [acceptance map](acceptance-map.md) for requirement-level proof.

[Focused checkpoints](focused-checkpoints.json) preserve successful stages,
expected tests-first failures and the separate owned-cleanup recovery. Raw logs
remain in the named local output directories. Counts describe selected cases;
registrations and partial runs are not claimed as passing executions.

[Source hashes](source-sha256.json) bind the corrected executable/test scope.
The [default](quality-old.json) and [current](quality-modern.json) CodeScene
reports each cover 68 source reviews: 66 score 10 and two contain declarations
only. The extra comparison `pair.rs` review is unchanged source; the five-file
correction is described in [its record](clippy-correction.json). Both safeguards
passed. The prior [native source freeze](native-source-sha256.json) and raw
`native-quality-*.json` reports remain separate. Documentation and final
verification evidence are recorded separately from these source freezes.

[The first full attempt](full-first-results.json) records 77 passed stages,
one Clippy failure and 40 not run. Runtime categories did not execute. Its
[lint log](full-first-clippy.log) and [private input cleanup](full-first-private-input.json)
remain unchanged. Optional expansion storage was boxed at its owning Session,
and four test slices now borrow their input; strict all-target Clippy and 20
affected pure cases then passed with owned cleanup.

## Native acceptance and measurements

The [focused native result](native-result.json), [independent audit](native-independent-audit.json)
and [dark](native-dark.json)/[light](native-light.json) reports pass all 15 checks
per theme in an actual native Metal window at 1050 by 680 points. The harness
analyzes the public C# fixture, checks ordinary cached-source queries, removes
the copied source files, then proves expansion reads the immutable artifact.
Repeated call slots remain distinct; recursion stops visibly; collapsing one
child keeps its sibling; target/Back and collapse-all restore selection, camera
and source scroll. Artifact bytes and generation remain unchanged.

Root inspected both owned-window captures. Native title-bar controls and the
two toolbar rows are clear; the selected helper call highlights cached
`Helpers.cs` line 55, and the explicit target/expand/collapse details fit inside
the viewport. [Inspection scope](visual-inspection.json) and screenshot hashes
are recorded. PNGs remain under the local output path in the audit rather than
being duplicated in Git. Input is synthetic through production Runtime handlers;
this checks a real displayed workflow, not OS mouse delivery.

| Measured work | Observed values |
| --- | --- |
| Initial C# analysis | 17.877 s |
| Package stage, including compilation | 90.566 s (compiler: 56.22 s) |
| Native gate, both themes | 26.244 s |
| Query plus layout edits: queued pointer press to first new Ready Drawn frame | 75.15–109.82 ms |
| Collapse-all: queued pointer press to first restored Ready Drawn frame | 17.65–17.73 ms |
| CLI query, including cold negotiation and client teardown | 30.63–53.44 ms |
| Graphviz layout | 22.97–47.94 ms |
| Category cleanup | 2.075 s |

These are six readiness observations per theme, not latency guarantees or a
statistical benchmark. Timing is sampled before gesture-settling delay, and
reports retain step/link counts (14/13 to 41/45). Query/layout are absent for
collapse-all because it restores the retained ordinary chart.
[Native cleanup](native-category-cleanup.json) reclaimed 1,363,498,077 bytes of
owned outputs and retained only the allowed prior-package artifacts; query
process cleanup also passed in both themes.

## Final full verification

`verify-full.sh --base HEAD` passed **118/118 stages** outside the macOS sandbox
with isolated profiling inputs and a disposable private Maven project copy.
[Raw stages](full-results.json), [timings/selection](full-summary.json) and the
[independent audit](full-independent-audit.json) are retained. The full command
and input cleanup are in [the wrapper record](full-private-input.json).

All 1,476 unique selected test registrations and 12 gates were covered by
successful stages, with zero not-run selections. These are selection counts,
not an inferred total of framework-reported passing tests. Compiler-backed
integration, CLI/MCP/installed-agent acceptance, ordinary/comparison/expansion
native journeys, performance, packaging and CodeScene all passed.

| Full recorded stage time | Seconds |
| --- | ---: |
| Compilation | 357.488 |
| Setup | 10.432 |
| Test/gate execution | 1,457.376 |
| Cleanup | 48.670 |
| Other static/bootstrap/package work | 127.507 |
| Total recorded stages | 2,001.473 |

Package work includes its own compilation; the raw stage log and focused
package breakdown above retain that cost rather than treating it as test time.
The summary keeps every category's compile/setup/execution/cleanup separately.
No ordinary tests were repeated in debug and release; release profiling and
strict all-target static linting have separate purposes.

All five category/check workspaces passed cleanup and their removed paths are
absent. Reports are retained for [checks](full-checks-cleanup.json),
[unit](full-unit-cleanup.json), [integration](full-integration-cleanup.json),
[acceptance](full-acceptance-cleanup.json) and [performance](full-performance-cleanup.json).
Only allowed prior-package artifacts were retained. The private original's
fingerprint is unchanged; its owned copy was removed after inactivity checks.
All 72 [full-run source hashes](full-source-sha256.json) match before/after/current
bytes, including the intentional deleted encoder. The 71-path reviewed source
freeze additionally includes unchanged comparison `pair.rs`.

The final boxed-Mode package passed native expansion again:
[result](full-native-result.json), [dark](full-native-dark.json) and
[light](full-native-light.json). First-ready edits measured 96.92–118.71 ms,
collapse-all 16.80–17.82 ms, query work 43.03–58.40 ms and layout 23.20–32.69 ms.
These are twelve observed readiness samples across both themes, with the same
representative 14–41 steps; they are not worst-case performance guarantees.
Root inspected six final ordinary, expansion and restored-comparison captures;
[visual observations and hashes](full-native-visual.json) are retained.

SonarMCP is intentionally inactive because this checkout has no
`.sonar/config.json`. No required behavioral/quality gate was waived. The final
[direct MCP pre-commit safeguard](pre-commit-safeguard.json) passed after staging
the source: 67 eligible files checked, no issues. Only task/evidence metadata
changed afterward. PR change-set analysis does not apply to this local commit delivery.
