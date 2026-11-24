---
id: doc-13
title: Native saved-flow comparison acceptance
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-24T13:20:00Z'
---
# Native saved-flow comparison acceptance

Migrated 2026-11-16 from `docs/testing/flow-comparison-native.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

Run this opt-in harness only after building the reviewed desktop candidate. It launches a relocated copy in the current macOS session, uses the bundled CLI and Graphviz, and captures the actual owned window in both themes at a 1050×680 content viewport.

```sh
python3.13 tools/flow_comparison_desktop_acceptance.py \
  --app /absolute/path/CodeCallNavigator.app \
  --output /private/tmp/comparison-native-new-output \
  --timeout 180
```

The helper requires Python 3.10 or newer, Git, the .NET 11 SDK for fixture preparation, an unlocked desktop with an active display and native Metal session, and permission to capture its window. Run outside the macOS sandbox so MSBuild host pipes, process inspection and descendant cleanup are available. The GUI gets a private Git-only PATH followed by `/usr/bin:/bin`, with no engine override or SDK PATH.

The compiler produces two saved artifacts from [the C# fixture](<../../../fixtures/flow/csharp-comparison/Comparison.csproj>). Expectations and portable IDs come from the bundled public CLI. Every Ready source is checked byte-for-byte against the original compiler inputs. `Unchanged.cs` stays byte-identical; the historical Legacy artifact deliberately loses exactly one cached source row while nodes and flow IR stay unchanged. A later empty commit has identical artifact bytes and a different endpoint binding. The working artifact and live C# sources are removed before launch.

The version 2 probe uses the Before and After revision pickers, the file-first Changes sidebar and endpoint snapshot controls. It covers source-only browsing, attributed class and method rows, source highlights, independent cameras, shared class depth, call drill-down and return, exact commit selection, and cancellation of an acknowledged owned Git read. A later Worktree journey checks missing snapshot discovery, preparation cancellation, and successful binding to the generation published for the normal graph. The probe observes actual Drawn callbacks, visible source rows, highlights and control bounds. The ordinary version 1 native probe/config/report remain supported by [the existing C# helper](<../../../tools/csharp_desktop_acceptance.py>).

Two captures per theme show an unavailable historical flow and the restored caller pair. Each uses the same PID/window lease as the existing helper. Fixed sequenced requests and acknowledgments bind external operations to their version, checkpoint and owned fixture.

Failure cleanup first disarms only the gate's token and terminates verified child
query-session groups. PID, parent, group, start time and command must still match
before a signal; transient scheduler state is observed separately. Before stopping
the app, each group must be absent or contain only zombies, explicitly recorded as
awaiting parent reaping. Stopped or unknown states cannot establish termination.
The helper then stops and waits for its owned app, and requires every leased group,
including zombies, and private query session to disappear before reporting cleanup
complete. Successful UI cancellation keeps the stronger requirement: the query must
be fully reaped while the GUI remains alive.

Group liveness uses checked numeric process-table snapshots. Inconclusive ownership
causes no unverified group signal and preserves the fixture/relocated app for
investigation. Failed runs retain `retained-fixture.json`; successful runs remove
their temporary fixture only after query cleanup is certified.

The output contains `result.json`, compiler source/artifact hashes, CLI command records, per-theme configuration/report, request/ack files, native trace, window lease audit, query cleanup audit and PNG captures. Inspect the captures before accepting the native gate. Unit tests and preparation preflight alone do not establish native acceptance.
