# Saved-edit graph freshness

This document records the TASK-59 feasibility recommendation and delivery contract.
TASK-63, TASK-60, TASK-61 and TASK-62 now deliver the C# input-coverage fix,
typed CLI observations, desktop/MCP adoption and opt-in automatic refresh.
The implemented observer uses verified polling; event-based monitoring and
cross-edit compiler reuse remain outside this delivery. Initial measurements
are in [the spike report](../evidence/freshness/README.md); delivered behavior,
publication budgets and verification are in
[the automatic-refresh report](../evidence/freshness/automatic-refresh/README.md).
The recommendations below retain the original design rationale.

## What current means

An artifact generation identifies the exact saved `graph.json` bytes. It is not
an editor revision, a compiler session or proof that the filesystem stayed fixed
after verification. Queries return facts from one immutable generation together
with a point-in-time observation of its recorded inputs.

Keep input validity (`match`, `stale`, `unverifiable`) separate from refresh
activity (idle, waiting, analyzing, failed). A failed refresh may leave a valid
older graph for navigation while its inputs are stale. Permission failures,
missing root mappings and unsupported provenance cannot become `match`.

Saved files are the first delivery boundary. Unsaved editor buffers are not
observed. Live source previews are not analyzed graph evidence. Source overlays
would require their own editor binding, snapshot identity and compiler-input
contract; filesystem monitoring cannot supply that feature.

## Recommended boundaries

| Owner | Responsibility |
| --- | --- |
| Core | Typed observation contracts, input fingerprints and verification, immutable artifact identities and atomic publication |
| Engine observer | Watch registration, input-manifest membership, missing inputs, linked roots, periodic reconciliation and contextual observation errors |
| Engine coordinator | Opt-in debounce, pending work, analysis subprocess ownership, retry and monitor shutdown |
| Query executor | Artifact loading, immutable index reuse and generation-bound response construction |
| Desktop | Background observation/adoption and presentation; no compiler or parallel freshness policy |
| MCP | Wire validation and owned CLI lifecycle; no filesystem watching, graph cache or graph interpretation |

Events only invalidate an observation or request reconciliation. Before reporting
`match`, verify the recorded content/membership inputs. Track containing
directories for additions, deletions, renames, atomic saves and previously missing
resolution candidates. Keep evaluated output exclusions and explicitly consumed
generated sources; ignoring every build directory would lose real inputs.
Reconcile after overflow, dropped events, watcher restart and root changes.
Canonical artifacts, inference-cache writes and unrelated build products must
not create refresh loops.

Observation freshness has its own lifetime. A generation digest alone cannot
validate a cached `match` after source changes. Preserve the current one-shot
verification guarantee unless a negotiated observation contract makes its
verification time, invalidation and limitations explicit. Volatile timestamps,
watcher state and scheduling metadata do not belong in canonical graph bytes.

## Refactoring needed for delivery

1. Replace the desktop `QueryContext::stale` boolean projection with a typed
   observation. Keep unknown status values and transport failures explicit at
   the boundary; project to a simple visual badge only in presentation code.
   Add capability negotiation without changing existing query version behavior.
2. Keep filesystem observation separate from `Loaded` artifact/index ownership.
   Reuse core verification and input manifests instead of maintaining another
   source-extension whitelist or a second graph invalidation algorithm.
3. Separate monitor shutdown from individual analysis cancellation. Current
   analysis entrypoints use a process-global flag with a one-request invariant.
   A monitor can initially own existing one-shot analysis child processes,
   retaining their current cancellation/descendant cleanup. Do not reset the
   global flag and run repeated analyses in the same process.
4. Split desktop generation adoption from workspace reset. Existing
   `install_graph` clears history, closes source and invalidates view state;
   `poll_analysis` requests camera auto-fit. Automatic adoption must retain valid
   stable symbol navigation, history and camera position, while dropping deleted
   selections and rejecting old epoch/generation jobs. Project changes keep the
   explicit reset path.

These are extractions beside their first consumer, not prerequisites for a broad
rewrite. Preserve the existing query executor and process supervisors where their
contracts already fit. Use concrete inputs/results rather than a generic watcher
plugin registry.

## Scheduling and publication

Start with explicit opt-in automatic full refresh and manual refresh retained.
Own at most one analysis per project; coalesce bursts into one pending refresh.
Avoid cancelling and restarting for every keystroke. Recheck inputs before
publication and again when adopting the generation. If inputs changed during
analysis, keep the last artifact and schedule the latest pending input state.

Invalid intermediate code leaves the prior generation intact with a stale/failed
observation. Retry after a relevant new saved change or explicit request, rather
than continuously compiling the same invalid state. Explicit user cancellation
and shutdown must not be interpreted as an automatic retry request.

Reuse the project publication lock across desktop, agents and command-line
refreshes. Lock contention is pending/busy, not an empty successful graph. A new
artifact requires consumers to reload a snapshot. Requests bound to the old
digest continue to reject mismatches; never silently merge old IDs with new
query results. An artifact observation and source observation must be checked
independently because either can change without the other.

## Compiler invalidation boundaries

| Route | Reuse candidate | Conditions requiring full preparation/reanalysis |
| --- | --- | --- |
| TypeScript | Project compiler programs and resolution state across saved edits | Config/reference closure, ownership, packages/compiler version, missing probes, directory membership and ambient/global declarations; dependencies can affect unchanged files |
| C# | Evaluated workspace/compilation within the owning adapter | Restore/assets, project graph, SDK/options, references, generators, partial declarations and cross-assembly dispatch |
| Java/Maven | Build-tool incremental compilation plus reusable class/source inventories | Effective POM/profiles/settings/toolchain, generated sources/resources, ordered classpaths, module/source-set membership and source/bytecode attribution |
| Java/Gradle | Build-tool incremental compilation plus evaluated metadata | Settings/build scripts/plugins/properties, task graph, generators, outputs, ordered classpaths and module/source-set ownership |

Existing inference-decision caching reuses boundary-routing decisions after fresh
compiler facts are available. It is not incremental compiler analysis or a basis
for skipping cross-file invalidation. Start with full graph extraction and
composition even where a build tool skips unchanged compilation. Promote
persistent compiler work only after an isolated experiment demonstrates both a
useful gain and semantic equivalence against full analysis.

## Acceptance boundaries

The first objective is one-second p95 saved-edit stale detection on each measured
supported workload. Report both detection and verification overhead; small
fixture event latency does not establish large-project compliance. Publication
latency is language/workload dependent and must have measured budgets before
delivery promises. Twenty warm samples provide a coarse nearest-rank p95, not a
statistical service-level guarantee.

Required delivery journeys include missed events, invalid code, linked-root
changes, concurrent readers/writers, input changes during analysis, cancellation,
monitor shutdown, generation replacement and desktop context preservation.
Measurement-only probes do not establish those production behaviors.
