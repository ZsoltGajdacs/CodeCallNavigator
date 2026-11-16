---
id: doc-22
title: Engine architecture and analyzer implementation
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:07:00Z'
---
# Engine architecture and analyzer implementation

Migrated 2026-11-16 from `docs/standalone-engine.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

## Adding another language

Keep new support within the existing explicit adapter dispatch:

1. Add language/build discovery and default selection rules.
2. Implement preparation and analyzer execution under the language adapter.
3. Normalize actual analyzer symbols, ownership and relationships into shared
   contracts. Supply semantic export keys where native IDs depend on positions.
4. Extend runtime resources, bootstrap locks and packaging together.
5. Add real-toolchain acceptance fixtures, invalid-input/cancellation checks and
   corpus comparisons covering flow and architecture facts where supported.

The shared coordinator owns lifecycle, composition, snapshots and publication.
Build tools stay separate from graph analysis. Framework inference belongs to
the relevant language adapter. This extraction introduces no dynamic plugin
framework and does not substitute name matching for compiler bindings.

Run `cargo test --workspace --all-targets` so moved package tests remain covered.
The frozen baseline and reproducible corpus are described in
`evidence/engine/README.md` and `evidence/engine/corpus.json`.


## C# evaluated build inputs

After restore, the engine inventories each selected/referenced project's evaluated
imports through MSBuild preprocessing in normal evaluation and the pinned Roslyn 5.9.0
.NET design-time context. It queries TargetFramework/TargetFrameworks in both modes and
includes each inner framework when required. Source/reference/additional-file metadata
uses the same contexts, retaining design-only source scopes. Configured normal-root
metadata remains authoritative for source ownership/classification; design-time and
inner-framework metadata only fill missing file/assembly owners. Incompatible
fallback-only classifications fail rather than letting evaluation order choose a result.
Solution opens propagate the absolute SolutionDir with its trailing separator to every
evaluation and metadata query. It snapshots those explicit files, including
NuGet-generated `obj` imports and SDK/external imports, and repeats preprocessing under
that snapshot before reading source metadata. It checks the same inventory and contents,
including each context's exact import membership, after metadata and CallAtlas
extraction; changes reject publication and preserve the previous graph. The existing
authored-source/directory snapshots remain in force. Build outputs are excluded from
directory scans, while evaluated import files remain explicit inputs, so ordinary output
creation does not trigger refresh loops. No source/relationship normalization changes
with this policy.

The inventory uses strict SDK 11 XML comment framing, including balanced import end
frames without parent labels. MSBuild escapes `--` as `__` in these comments; the engine
resolves existing filesystem components and rejects ambiguous spellings instead of
guessing. Unsupported framing, missing evaluated imports and ambiguity fail with project
context. Evaluated lexical aliases are preserved.

MSBuild command-line property values are encoded literally, escaping percent before
semicolon and comma. This prevents a SolutionDir or dynamic target-framework value from
introducing additional global-property assignments. A real SDK round-trip preserves the
full value and the pinned design-time flags.

The pinned native Roslyn/CallAtlas integration has a separate limitation in the retained
delimiter/percent solution-path probe: a SolutionDir-expanded Import produces only the
unconditional method, while a relative design-time Import produces the expected
conditional method as well. Both imports are observed by the preflight inventory. This
change fixes the CLI property boundary and does not repair that native import-expansion
behavior; see the
[probe](<https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/13b0de13f3ddf904145f93119816c33b8a5b3e55/evidence/freshness/csharp-import-fix/literal-property-probe.json>). Ordinary
SolutionDir-expanded imports remain covered by real compiler acceptance.

The JSON version report in `analysis.engine_versions` records `csharp_input_policy` as
`msbuild-preprocess.v2`. The contexts follow the pinned [Roslyn BuildHost
source](https://github.com/dotnet/roslyn/blob/35d9211b841e7613c1d2f8f5af6d628ace696c4c/src/Workspaces/MSBuild/BuildHost/Build/ProjectBuildManager.cs#L25-L52).
The .NET design-time host uses the NonExistentFile sentinel and does not set
BuildingInsideVisualStudio. Revisit this policy when upgrading the Roslyn dependency.
Earlier v1 and legacy C# artifacts remain readable, but verification reports
`legacy_csharp_input_policy` and `unverifiable` until explicitly refreshed. Known
changed inputs still take precedence as `stale`. This policy check applies to C# nodes
or C# build targets, including valid empty graphs; Java/TypeScript artifacts are
unaffected.

Freshness verifies observed inputs only. SDK/external inputs outside declared logical
roots produce `unmapped_input` and `unverifiable`; use explicit `--root` mappings when
those inputs must be verifiable. Imports skipped by a false condition and absent
conditional-resolution candidates are not recorded probes. The repeated evaluation
detects membership changes during analysis, including normal/design imports swapping
while their union stays unchanged. Creating an unobserved conditional input after
publication is outside this guarantee. An empty external Compile glob returns no source
item from which to derive a directory scope; its first later file is likewise unobserved
unless a known project/import or recorded source scope covers it. This differs from an
empty referenced project, whose directory is explicitly observed. No arbitrary MSBuild
expression/glob parser is used to claim otherwise.


### Shared call-site expansion (library)

The shared `flow::expand` API accepts immutable borrowed `FunctionSource` pairs
and an explicit `ExpansionRequest`. CLI and desktop controls are delivered in
TASK-22.8; the existing single-function query remains unchanged.

Optional `FunctionFlowIR.execution` and `CallSite.invocation` facts describe
compiler-owned whole-body execution and final invocation activation separately.
Unknown facts are omitted and remain unknown when reading older artifacts. The
canonical flow inventory stays version 1. Fresh Java resources require
`kind_v9`; fresh TypeScript preparation requires adapter 7. Rebuild matching
resources after upgrading; historical graph and ordinary flow reads remain valid.

Only a proven isolated synchronous invocation into a synchronous body can be
expanded. Receiver and arguments precede that final invocation in the supplied
IR. A virtual or candidate target requires an exact explicit choice and retains
its certainty and provenance. References, HTTP/navigation relationships, deferred
regions and unsupported or unknown execution remain stopped. Return types and
the number of retained targets do not establish activation timing or dispatch.

Each occurrence retains its complete caller/site/target path. Original IR fields,
call slots and typed cleanup routes distinguish equal spans, repeated calls and
cleanup copies. Source snapshots retain build/project/file/language ownership,
exact UTF-8 bytes and digests. Step origins bind every displayed call and span to
the correct occurrence. A stopped ancestor has a real marker; requested descendants
are recorded separately as unevaluated `blocked_selections`, never invented bodies.

Defaults are depth 8, 64 reached selection evaluations, 1,500 assembly steps,
6,000 links and 16 MiB of unique-owner source bytes. Hard depth/request ceilings
are 32/128; the other defaults are also hard ceilings. Zero depth or request work
is valid and produces typed stops. Graph/source allowances must be positive.
The independent request envelope permits 128 selections, occurrence depth 32 and
1 MiB of encoded identity data. Assembly limits apply before unreachable pruning.

Limits never silently cut off caller continuation or cleanup. The same assembler
first proves that the complete stopped root fits. Private attempts then demote an
active or latest accepted root expansion on budget exhaustion and retry, at most
once per demoted selection. This deterministic policy does not maximize the
expanded subset. A mandatory root that cannot fit fails explicitly. Sources stay
borrowed until complete success; cancellation publishes no partial result.

Query generation, artifact digest and revision binding belong to the consuming
query boundary, rather than an arbitrary snapshot string inside the core result.
Expansion does not modify graph relationships, stored inventories or ordinary
single-function chart construction.



## Incremental inference

Each run still extracts and validates fresh compiler facts. Observer, callable,
Spring and MediatR inference currently recompute from those facts. The boundary
pass reuses dependency-keyed routing decisions in the disposable project-local
`.codecallnavigator/inference.sqlite3`; it never loads prior graph edges as evidence.
The SQLite file and its exact journal/WAL/SHM names are excluded from discovery,
source fingerprints and refresh triggers. Continue committing only the single
graph file and authored configuration, not this derived cache.

A decision depends on its current source use, configured application scope,
toolchain, complete matching route/blocker membership and supported symbols.
Negative lookups track empty membership, so adding a matching handler invalidates
a cached miss. An automatic implementation digest invalidates decisions when the
rule implementation changes. Hits rebind current symbols and construct fresh
edge evidence and flow targets; unrelated node index changes cannot replay old
indexes. Flow attachment always checks the current exact source call site.

Cache reads are optional and read-only. Missing, incompatible or malformed data
falls back to full inference and reports a volatile warning. Unowned, corrupt or
nonregular cache files are preserved rather than overwritten; remove the derived
cache to rebuild it. Successful input verification and atomic graph publication
happen before the cache replaces its current rows in one transaction under the
same project lock. Cache persistence failure cannot turn a published graph into a
failed analysis. Failed or cancelled analysis preserves the successful graph and
previous cache. The cache stores no graph history.

Use `codecallnavigator-engine clear-cache --project /checkout` or the desktop's
**Clear cache** button beside analysis to discard the inference database and its
exact `-wal`, `-shm` and `-journal` sidecars. Clearing holds the same project
analysis lock and fails while another analysis owns it. Nonregular or hard-linked
cache paths and a symlinked `.codecallnavigator` directory are rejected before deletion.
Corrupt regular cache files can be removed so the next analysis recomputes all
inference decisions. Clearing preserves `graph.json`, the displayed graph,
navigation, authored configuration and build outputs; start analysis separately
after it completes.

Set `CODECALLNAVIGATOR_DISABLE_INFERENCE_CACHE=1` to run the full reference path without
reading or writing the cache. Canonical graph bytes are identical in both modes.
Cache hit/miss/rejection counters and warnings use progress/performance reporting,
not canonical graph content.

## Build process orchestration

One-shot build/analyzer subprocesses receive closed standard input, so launching
through a terminal cannot suspend Gradle on background terminal reads. TypeScript's
internal worker keeps its dedicated protocol pipe.

Analysis reuses classpath and source inventories only within the current refresh.
Gradle ownership discovery remains separate; compilation and final metadata export
share one invocation. TypeScript retains its compiler for a single selected target;
multi-target analysis bounds memory by recreating one verified worker at a time.
All content snapshots and per-context classpath rules remain enforced.


## Component ownership

`codecallnavigator-core` owns shared graph/flow/architecture contracts and pure graph
operations. `codecallnavigator-engine` owns the coordinator and compiler-backed language
adapters. The desktop owns presentation and workspace caching and invokes the
engine executable rather than calling analyzer functions.

`analyze-native` is an internal acceptance oracle retained for comparison with
the pre-extraction SQLite implementation; it writes no persistent graph file and
is not the supported project export interface.
