# TASK-52 direct CLI delivery acceptance

2026-11-12; macOS arm64; codex-cli 0.166.0 and Claude Code 2.1.306.

These are real model conversations using temporary copies of the canonical skill, not evidence of finished host package installation. All graph queries used the real compiler-produced fixture through `ccnavigator` on PATH. No MCP server was configured.

## Backend and public command

Independent tester passed 20 real CLI operations at `target/agent-integrations/task52-direct-tester.json`: capabilities, graph_missing/no publication, two ambiguous methods, directed sections, truncation, flow_unavailable, stale inputs, unchanged bytes under queries, explicit analysis/generation mismatch, and missing executable. Initial default-root freshness was unverifiable, with reasons preserved.

The PATH regression in `crates/codecallnavigator-engine/tests/public_command.rs` failed before the public command existed and passed afterward. It compares capabilities with the internal engine, checks public help and verifies the structured exit-2 missing-graph response.

## Host preparation and limits

A separate fixture explicitly mapped `observed-inputs=/` solely to represent compiler-observed ancestor manifests and dependency inputs, allowing real freshness=match. Analysis still targeted only its temporary tsconfig; the filesystem root was not scanned. This broad test-only metadata mapping is **not** a recommended installation or skill default. No graph or freshness envelope was fabricated.

Codex used temporary CODEX_HOME with a temporary reference to the existing login, a trusted temporary project, and read-only sessions except the explicit refresh test. Claude used its existing login with `--setting-sources` empty, strict empty MCP configuration, a session-local plugin directory, and no session persistence. Terminal tools were authorized only for these requested fixture checks; normal host trust was not bypassed. Production configuration was not changed.

Exact reusable prompts/expected outcomes are in [shared journeys](../../integrations/shared/journeys.md). Raw JSONL transcripts remain under `target/agent-integrations/task52-{codex,claude}-*.jsonl`.

## Codex observations

### explicit

11 completed terminal operations. Final graph-context output:

Two `start` functions match. Which should I inspect for possible callees?

- [entry.ts:2](/private<temporary-project>/entry.ts:2) — `start(flag: boolean): number`
- [other.ts:1](/private<temporary-project>/other.ts:1) — `start(): number`

Queries included `observed-inputs=/`. Generation: `d976d366d31289c81e2aab7bf241f341945c1393118534d3ff614bd6730cbe82`.

Freshness is **stale**, with `input_changed` at `@repo/`, so results describe the earlier analyzed state. No refresh or file edits were performed.

### callers

5 completed terminal operations. Final graph-context output:

`leaf(): number` at [entry.ts:1](/private<temporary-project>/entry.ts:1) has one caller in the graph:

- [`start(flag: boolean): number`](/private<temporary-project>/entry.ts:2) → `leaf`, with one direct call relationship reported by `typescript.compiler`. This is source-derived evidence, not observed execution.

The Incoming query used depth 1, CLI default filters, and bounds of 100 nodes/300 edges. No results were omitted; no diagnostics were reported.

Freshness: `match`. Generation: `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. Every query used `observed-inputs=/`. No refresh or file edits were performed.

### repeat

1 completed terminal operations. Final graph-context output:

Unchanged (generation `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`, freshness `match`); see the previous caller summary.

### flow

5 completed terminal operations. Final graph-context output:

`Contract.run(): void` at [entry.ts:3](/private<temporary-project>/entry.ts:3) is an interface method declaration with no executable body. Its signature accepts no arguments and returns `void`; there is no implementation flow to explain.

The flow query returned `flow_unavailable`: “No source body is available for this declaration.”

Freshness: `match`, generation `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. Every query retained `observed-inputs=/`. No refresh or edits were performed.

### stale

1 completed terminal operations. Final graph-context output:

Current impact is uncertain: the graph is now **stale** because `@repo/other.ts` changed (`input_changed`).

In the earlier analyzed state, `start(flag: boolean): number` directly called `leaf(): number`. Editing `leaf`’s return value or behavior could therefore change `start`’s result; changing its signature could require updating that caller.

That historical Incoming query used depth 1, default filters, and bounds of 100 nodes/300 edges, with no omissions. It does not establish the current caller set or wider impact.

Generation: `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. The status check used the same explicit project and `observed-inputs=/` mapping. No refresh or edits were performed.

### refresh

1 completed terminal operations. Final graph-context output:

Refresh failed: `ccnavigator analyze` returned `Operation not permitted (os error 1)` in this read-only environment.

The graph remains stale because `@repo/other.ts` changed. Its earlier caller summary showed `start(flag: boolean): number` directly calling `leaf()`, but that is historical evidence, not a verified current caller list.

### bounded

4 completed terminal operations. Final graph-context output:

The bounded Outgoing query for `entry.ts:2`’s `start(flag: boolean): number` returned only `start` itself. The one-node limit excluded the callee: **1 node and 1 edge were omitted**, and `truncated: true`. This does not mean there are no callees.

Earlier historical evidence identified a direct call to `leaf()`, but this bounded result does not display it.

Scope: depth 1, CLI default filters, maximum **1 node / 1 edge**. The graph is **stale** (`input_changed: @repo/other.ts`), generation `cd66088bf8170fd774401dcef256260acb431e6805d95bca12309957c6682ed4`. All queries retained the same explicit project and `observed-inputs=/` mapping. No refresh was performed.

### refresh-authorized

11 completed terminal operations. Final graph-context output:

Refreshed using the discovered `tsconfig.json` target and explicit root `observed-inputs=/`. No source was edited.

Generation: `1cfb84f22f0fda393474a50692143383c52a0e658cb887c0b157803cd750a5e8`
Freshness: **match**.

`leaf(): number` at [entry.ts:1](<temporary-project>/entry.ts:1) has one identified caller: `start(flag: boolean): number` at line 2. The TypeScript compiler resolved one direct call from `start` to `leaf`; this is structural evidence, not observed execution.

The Incoming query used depth 1, CLI default filters, and bounds of 100 nodes/300 edges. Nothing was omitted or truncated; no diagnostics were reported.

### missing

4 completed terminal operations. Final graph-context output:

I couldn’t identify callers of `leaf` in the requested project: `ccnavigator` returned `graph_missing` (“No such file or directory”). Generation and freshness are both unavailable.

Source fallback found no files. This does not establish that `leaf` has no callers. No analysis or refresh was run.

## Claude observations

Explicit invocation independently presented both start candidates without selecting a first hit (`task52-claude-explicit.jsonl`). The three-turn callers/repeat/flow conversation reported match freshness and used the following observable behavior:

### Turn 1 (4 tool calls)

`leaf` has one caller in the graph: **`start`**.

**Graph state:** project `<temporary-project>`, with the root mapping `observed-inputs` → `/` used in every query. Generation is `cd66088bf817…682ed4` and freshness is **`match`**, meaning the graph agrees with the inputs the tool could check, with no reasons listed. The whole graph has 6 nodes and 1 edge. I did not refresh or edit anything.

**Target:** the search found only one `leaf`, so there was nothing to choose between. It is `leaf(): number`, a function declared at `entry.ts:1` and built from `tsconfig.json`.

**Callers** (looking one step up, at what calls `leaf`; no filters added, limits of 100 nodes and 300 edges):

| Caller | Location | Edge |
|---|---|---|
| `start(flag: boolean): number` | `entry.ts:2` | direct call, seen once, found by the TypeScript compiler |

- Nothing was cut off (truncated: false, 0 omitted nodes or edges) and there were no diagnostics.
- Neither function is a test or generated code.
- Only one level of callers was checked, so I didn't look for anything that calls `start`.

**Limits:** this edge comes from reading the source, not from watching the program run. `start` takes a `flag: boolean`, so the call to `leaf` might only happen on one branch. The caller list doesn't show that. A `flow` query on `start` would answer it if you want.

### Turn 2 (1 tool calls)

Nothing has changed (generation `cd66088bf817…682ed4`, freshness `match`), so the previous caller summary still applies.

### Turn 3 (4 tool calls)

The graph can't show a flow for `Contract.run`. It is declared in an interface, so it has no body to trace.

**Graph state:** same project, root mapping `observed-inputs` → `/`, generation `cd66088bf817…682ed4`. Freshness is **`match`**, with no reasons listed. I did not refresh anything.

**Target:** the search for `run` found only one method, so there was nothing to choose between:
- `run(): void` at `entry.ts:3`. It is declared as a method signature inside the interface `Contract`, built from `tsconfig.json`.
- The symbol details say `body_unavailable: true`. It is not a test or generated code.

**Flow result:** the `flow` query returned the error **`flow_unavailable`**: "No source body is available for this declaration." (exit 2, no result). This doesn't mean the flow is empty. An interface method only declares a type and contains no code. The actual flow would be inside whatever classes or objects implement `Contract`.

To build the requests I wrote a temporary file in `/tmp`, then deleted it. Nothing in the project was changed.

**Possible next steps** (I haven't run these):
- Look at incoming relationships for `run` to see what calls it or implements it, then run `flow` on an implementation that has a body.
- Read `entry.ts` directly. The source would show what implements `Contract`, but that isn't the same as graph-based flow.

## Iteration and remaining package gates

The first Codex trial guessed the linked-root JSON shape incorrectly; the canonical workflow now includes exact array-of-pairs and selection examples. An early repeat correctly skipped the section request but restated caller content; the refined instruction and retest returned only a reference plus generation/freshness. The read-only refresh trial explicitly reported its permission failure; the subsequent workspace-write, explicitly authorized refresh succeeded and returned a new match generation. Diagnostic transcripts are retained separately.

These representative conversations are behavioral acceptance, not a guarantee of all future model outputs. Full installation/uninstallation and implicit selection belong to TASK-53/54. Desktop UI, other platforms and hook behavior are not claimed.

## Missing-command fallback

A separate real Claude session without the test binary directory on PATH reported command-not-found/exit 127, no graph generation/freshness, no installation or analysis, and clearly labeled its caller answer as source-derived. Raw transcript: `target/agent-integrations/task52-claude-unavailable.jsonl`. The caller evidence was correct, but the model also misplaced an unrelated interface declaration in its prose. The skill now requires filename/line-preserving source fallback instead of anonymous concatenated file content; this model-output limitation is not hidden as a protocol success.

## Independent transcript review

The tester independently confirmed generation binding in analyzed callers/flow/bounded queries, one status-only repeat per host, preserved flow_unavailable and no implicit analysis in stale/missing cases. Codex repeat was 227 UTF-8 bytes; all checked final graph-context outputs were below 8 KiB. The authorized-refresh trial omitted the test root mapping on its first status attempt, then restored it for analysis and subsequent queries; it does not establish perfect mapping retention on every initial attempt. The final source-fallback wording awaits its package-level retest.

## Final verification

`./verify-full.sh` passed all 28 stages in `target/verification/task52-direct-full/`: 552 ordinary Rust tests, 642 release tests including ignored acceptance, 164 Python project tests, 26 Python tool tests and 112 TypeScript tests; storage/GPU benchmarks, packaging/signature verification, independent MCP installation, CodeScene and cleanup passed. The newly packaged ccnavigator then passed all 20 direct CLI journey operations via PATH (`target/agent-integrations/task52-packaged-cli.json`), exercising adjacent bundled analyzer resources. All seven Code Health eligible changed source files passed the pre-commit safeguard; reviewed files scored 10.0. Sonar is inactive because this checkout has no `.sonar/config.json`.
