# Public CLI graph queries

## Git graph comparison

Both `ccnavigator` and `codecallnavigator-engine` support saved-graph comparison
through `query` and `query-session`. For example, send this JSON to
`ccnavigator query`:

```json
{"version":2,"project":"/checkout","query":{"kind":"compare","base":"main","head":"WORKTREE","offset":0,"limit":100}}
```

`base` must resolve to a commit. `head` defaults to `WORKTREE`, the current
saved graph for the checkout, and also accepts a commit, branch or tag. The
query compares endpoints directly using the local graph library. It performs
no merge-base calculation, checkout, fetch, compilation, or library write.
Git commands have a 30-second deadline and share the query process group.

For graphs produced by analysis, the working graph’s `status` result includes
`effective_analysis` with the selected targets, analyzer backends and versions,
analysis modes, logical roots, and observed dependency identity.

With the `local_snapshots` capability, `snapshot_status` takes a `revision`
and returns the resolved commit, clone directory in the library, and verified
variants. Supply `base_snapshot` or `head_snapshot` to select a variant by ID
in `compare` and `flow_comparison`. Commit endpoints include a `snapshot` record
with variant ID, effective build and analyzer selection, and manifest digest.
If several variants are available, implicit lookup
reports `snapshot_ambiguous`.

To prepare an absent snapshot programmatically, send a version-2
`prepare_snapshot` request to `serve-once` with `project`, `revision`, and
optional project-relative `targets`. Keep stdin open for progress and send
the normal `cancel` request to cancel the operation. The terminal
`snapshot_prepared` event carries the retained snapshot.

Engines advertising `local_snapshots.exact_targets` also accept a nonempty
`exact_targets` list of objects with `path`, `kind`, and `language`. Leave
`targets` empty when using this field. Paths are project-relative. Use `.`
for a Maven or Gradle build at the project root. Each target must resolve
to the specified build in the requested revision.

The version-1 result contains `base` and `head` endpoint metadata (`kind`,
`requested_ref`, resolved `commit`, absolute manifest `path`, `schema`, exact-byte
BLAKE3 `digest`), `comparison_id`, `graph_equal`, `metadata_equal`, per-section
`summary`, and `changes`. Summary keys are JSON Pointer section locations such as
`/graph/nodes`, `/graph/analysis`, or `/extension-name` for an artifact extension.
Summary counts are `changed_entities`, `records`,
`added`, `removed` and `replaced`; operation counts count records, not entities.
Analysis provenance and top-level artifact extensions are metadata. Other graph
sections contribute to `graph_equal`. Formatting-only differences affect digests
and comparison binding but leave both equality flags true.

Each change has `scope` (`graph` or `artifact`), `section`, `entity`,
`operation` (`add`, `remove`, `replace`), an
entity-relative JSON Pointer `path`, and applicable `before`/`after` values.
Absent sides are omitted; JSON null remains an explicit value. Nodes match exact
stable IDs, so identity-changing moves, renames and signature changes are
removal/addition. Relationships group all variants by `(from,to,kind)`; certainty,
provenance, counts and evidence remain intact. Flows match function symbols and
architecture files match `(build,project,file)`. Boundary facts, diagnostics and
entry memberships use exact-value groups. Changed ordered arrays are replaced
whole; control-flow and opaque extension array order remain meaningful. Unordered
collections follow the existing artifact canonicalization policy.

Pages have `offset`, `limit`, `returned`, `total`, `more` and `next_offset`.
`limit` defaults to 100 and accepts 1–500. For subsequent pages, use the returned
`next_offset` and pass the first page's `comparison_id` as `expected_comparison`:

```json
{"version":2,"project":"/checkout","query":{"kind":"compare","base":"main","offset":100,"limit":100,"expected_comparison":"<64-character comparison_id>"}}
```

Every page rechecks refs and artifact bytes. Moving a ref, even to a commit with
identical graph bytes, invalidates the binding. `expected_generation` optionally
binds the target artifact digest. The envelope generation always identifies that
target. Working targets include normal source freshness and permit stale saved
graphs. Verified commit targets have null freshness and need no working artifact or
linked checkout. Historical source freshness is not inferred.

The complete JSON envelope is capped at 24 MiB. Pages may contain fewer records
than requested; use `returned`/`next_offset`. Values are never silently shortened.
A single oversized record fails with `response_too_large`. Other errors include
`comparison_mismatch`, `revision_unavailable`, `graph_missing`, `invalid_graph`,
`git_timeout` and the existing query errors. Queries exit 0 for a successful
comparison, including differences, and 2 for structured errors.

The older standalone `compare --project ... --base ... --head ...` command keeps
its original output and exits 0 for equal, 1 for different graphs. Use the query
operation for detailed, paged results and metadata.

The engine is the backend for desktop and external clients. Query operations do
not compile, refresh, publish or write graph caches. Refresh is an explicit
`analyze` command. Failed or cancelled analysis preserves the last successful
artifact. The existing `serve-once` analysis protocol remains available.

## Changed classes, methods and files

Engines advertising `comparison_changes.version: 1` accept a compact, read-only
comparison of saved snapshots through `query` and `query-session`.

```json
{"version":2,"project":"/checkout","query":{"kind":"comparison_changes","base":"main","head":"WORKTREE","selection":{"kind":"list","limit":50}}}
```

The result includes the same endpoint metadata, comparison ID, and page fields
as `compare`. Rows contain a stable `id`, readable `name`, `file`, `kind`
(`class`, `method`, or `file`), `status` (`added`, `removed`, or `modified`),
and `changes_in`. Method rows include the exact `symbol` usable with
`flow_comparison`, `flow_available`, and `class_id` when a class owns the method. Entry-point
references report whether each saved graph records a direct or possible route.
A null side means no route was found on that side. The
`coverage.entry_point_search_complete` flag indicates whether reachability
search completed. Empty entry-point lists leave the change visible.

Example excerpt from `result.changes` with abbreviated IDs:

```json
[
  {
    "id": "class-…",
    "kind": "class",
    "name": "Checkout",
    "file": "Checkout.cs",
    "status": "modified",
    "changes_in": ["members"],
    "entry_points": [{"id": "entry-id", "name": "Submit order", "before": "direct", "after": "direct"}]
  },
  {
    "id": "method-…",
    "kind": "method",
    "name": "ValidateOrder()",
    "file": "Checkout.cs",
    "status": "modified",
    "changes_in": ["flow", "source"],
    "entry_points": [{"id": "entry-id", "name": "Submit order", "before": "direct", "after": "direct"}],
    "class_id": "class-…",
    "symbol": "symbol-id",
    "flow_available": true
  }
]
```

Use `next_offset` to continue and pass the first page's `comparison_id` as
`expected_comparison` on every follow-up. Keep the same refs and snapshot
selectors. To fetch saved source, send `selection: {"kind":"source_diff","id":"<change id>"}`
with that binding. A ready result contains a file and unified hunks with old and
new line ranges and typed context, deletion, and insertion lines. An unavailable
result includes a reason, such as `missing_saved_source` or
`saved_source_too_large`. An added or removed file can return a ready diff with
changes on one side. This query uses the saved source.

## Class flow comparison

Use `class_flow_comparison` to load the class-to-class call graph for a class
row returned by `comparison_changes`. The engine advertises version 1 in
`capabilities.class_flow_comparison`.

```json
{"version":2,"project":"/checkout","query":{"kind":"class_flow_comparison","base":"main","head":"WORKTREE","expected_comparison":"<comparison_id from the list>","id":"<class row id>"}}
```

Use the same refs and snapshot selectors as the list. `expected_comparison`
is required. The response includes the bound endpoints and a `graph` with
the `root` class ID, `nodes`, and directed `links`. Nodes include the
class ID, a display name, and `before` and `after` presence flags. For the
root these flags describe existence in the saved revisions. For other
nodes they describe participation in the root's one-hop call neighborhood.

Each link contains its caller (`from`), callee (`to`), and `before` and
`after` objects mapping recorded resolution to call count. An empty object
means the link is absent on that side. Node and link status identifies
`added`, `removed`, or `unchanged` records. A link is `modified` if its
resolution counts changed. These are analyzed relationships and counts,
with uncertainty preserved. Same-class calls stay inside function and
method-level views. The engine returns graph facts without display layout.

### Saved class node source

The optional version 1 `class_comparison_source` CLI query retrieves source
for a node from a loaded class graph. Pass the root class ID, the hovered
node ID and the requested `before` or `after` side.

```json
{"version":2,"project":"/checkout","query":{"kind":"class_comparison_source","base":"main","head":"WORKTREE","expected_comparison":"<comparison_id from the list>","id":"<root class id>","node":"<class node id>","side":"after"}}
```

The `source` field has kind `diff` with unified hunks and a file, `excerpt`
with saved text, a file and a one-based starting line, or `unavailable`
with a reason. Excerpts use the requested side. In shared files, diffs use
matching methods with verified saved spans. The class node must be present on
the requested side of the graph. Reuse the class graph's snapshot selectors.

## Changed-symbol impact preview

Run `ccnavigator impact-preview --project /checkout --base main` or send the
following to `ccnavigator query` or `ccnavigator query-session`:

```json
{"version":2,"project":"/checkout","query":{"kind":"impact_preview","base":"main","head":"WORKTREE"}}
```

The optional `impact_preview` capability advertises version 1. The base must have
a retained snapshot in this clone. Use `base_snapshot` (CLI `--base-snapshot ID`)
to choose a variant when several exist. Prepare a missing baseline explicitly.
The head defaults to `WORKTREE` and may also be a local ref. The preview compares
the endpoints directly. Pass an explicit merge-base commit when you want the
branch's changes since it diverged.

The engine reads the baseline bundle and Git's old-side source ranges. The
result identifies both endpoints and includes project-relative change records,
baseline symbols, and recorded caller chains with edge certainty and provenance.
Caller paths run from the indicated caller to the changed symbol. The preview
describes potential impact in the baseline and does not compile the head,
check out source, publish a graph, or change the cache.

Coverage is restricted to tracked project paths. Newly added files, pure
insertions, unsupported changes, and old lines without a matching saved symbol
range have explicit unresolved reasons. A record can contain both mapped symbols
and an unresolved remainder. Untracked working-tree files and paths outside the
project are outside this preview. If a result reaches its path, range, chain, or
byte budgets, `coverage.truncated` is true. Counts then describe observed
coverage only. A saved route represents a static relationship in the baseline.

## Saved-revision function flows

Check the optional `capabilities.flow_comparison.version` (currently 1) before
using `flow_comparison`. It reads the same saved endpoints as `compare`, without
requiring a loaded desktop graph or a current working artifact when both refs
are commits. It does not analyze historical source. Start with a catalog:

```json
{"version":2,"project":"/checkout","query":{"kind":"flow_comparison","base":"main","head":"WORKTREE","selection":{"kind":"catalog","text":"Service","offset":0,"limit":50}}}
```

The result has `kind: "catalog"`, `version: 1`, `base`, `head`,
`comparison_id`, endpoint availability, `complete`, and the usual paging fields.
Endpoint metadata retains the requested ref, resolved commit, artifact path,
exact-byte digest and observed schema (null when it cannot be identified).
Each record identifies an exact saved symbol and its `membership`: `both`,
`added`, `removed`, or `unknown`. Its sides are `present`, `missing`, or
`unknown`; present sides contain declaration metadata and flow availability.
`flow.status: "recorded"` means owned source/IR facts are available, not that a
chart has already been built. The catalog includes saved nodes even when their
flow is unavailable. Text matches symbol, label, signature, project or file,
using ASCII case-insensitive substring matching. Ordering is by exact symbol ID.

Catalog pages default to 50 records and accept 1–500. Pass the returned
`comparison_id` as `expected_comparison` on subsequent searches, pages and detail
requests; it is mandatory for nonzero offsets. Every request rechecks both
endpoints. A moved ref or changed saved bytes returns `comparison_mismatch`,
including a moved ref whose new commit contains identical artifact bytes.

```json
{"version":2,"project":"/checkout","query":{"kind":"flow_comparison","base":"main","expected_comparison":"<64-character comparison_id>","selection":{"kind":"detail","symbol":"<exact symbol ID>"}}}
```

The detail result contains `before` and `after`, each tagged `ready`, `missing`,
or `unavailable`. A ready side contains its node, source IR, semantic chart,
analyzed source and endpoint-local target summaries. Source is the exact cached
UTF-8 snapshot, including BOM and line endings; live files and linked checkouts
are not used to reconstruct charts or locations. Call targets retain certainty
and provenance; endpoint-local target summaries include body availability.
A known declaration with absent,
ambiguous or unsupported flow/source remains unavailable with an explicit reason.
An incompatible artifact makes its membership unknown rather than establishing
an addition or removal; the other endpoint can still be available. An unreadable
or missing revision/blob instead fails the request. Strict whole-artifact
validation still applies, including to unselected metadata.

Automatic correspondence uses the exact symbol ID on both sides. Renames, moves
and signature changes remain separate removal/addition records. To compare a
chosen counterpart explicitly, supply `selection.head_symbol`; differing IDs
produce `correspondence.kind: "manual"`, with both requested IDs retained. There
is no heuristic rename matching.

The normal envelope generation identifies the head artifact digest. Committed
heads have null freshness; WORKTREE uses the existing read-only input freshness
observation. This observation may read live paths, but never replaces cached
source in the result. Old artifacts remain graph-readable and explicitly report
missing flow/source metadata rather than compiling or refreshing automatically.

Refs and symbol IDs are limited to 16,384 bytes, catalog text to 1,024 bytes,
and each analyzed source to 16 MiB. The complete response envelope, including
escaped JSON and its newline, is limited to 24 MiB. Catalogs page whole records;
details return the complete pair or `response_too_large`. Source and sides are
never silently truncated. Cancellation and Git deadlines follow `compare`.

## Bounded call expansion

`flow_expansion` is an opt-in version-1 operation. Ordinary `flow` requests retain
their existing response and single-function behavior. Use the generation from a
saved snapshot; expansion requires nonempty `expected_generation` and fails with
`generation_mismatch` if its artifact changed.

```json
{
  "version": 2,
  "project": "/path/to/project",
  "expected_generation": "GENERATION_FROM_SNAPSHOT",
  "query": {
    "kind": "flow_expansion",
    "expansion_version": 1,
    "symbol": "EXACT_SYMBOL_FROM_SNAPSHOT",
    "selections": []
  }
}
```

The result is the core `ExpandedFlow`: `version`, `root_symbol`, `chart`,
`occurrences`, `step_origins`, deduplicated `sources` and optional
`blocked_selections`. Zero selections returns the ordinary root body plus its
ownership/address metadata. Geometry and result-local step indices are not
structural identities.

For a selected step/call, take its address from `step_origins[step].calls[call]`,
its caller path from that origin's occurrence, and the complete chosen target
record from the chart call. Append a selection
`{"caller": CALLER_OCCURRENCE_PATH, "choice": {"site": SITE_ADDRESS, "target": FULL_TARGET}}`.
Include every selected ancestor explicitly. Send the full target record unchanged,
including certainty, provenance and relationship kind; stale or invalid choices
fail with `invalid_request`. Collapse an occurrence by removing its selection and
all selected descendants. Unsupported but valid choices produce explicit core
stop markers; requests below a stopped ancestor are reported as blocked.

Source owners use portable logical paths such as `@repo/Helpers.cs`, unlike the
hydrated paths in ordinary working-generation `flow` answers. Read the exact
source table and its digest; never substitute a current disk file. Expansion
reads only retained artifact facts and does not hydrate a graph index, refresh,
compile or write the artifact. The desktop shares these semantics through the CLI.

Optional `limits` contains `depth`, `requests`, `steps`, `links` and `source_bytes`.
Defaults/hard ceilings are 8/32 depth, 64/128 reached requests, 1,500 steps,
6,000 links and 16 MiB of unique owned source. Depth and reached requests may be
zero; graph/source allowances must be positive. The independent request envelope
permits 128 selections, occurrence depth 32 and 1 MiB of encoded identities;
limits and envelope are validated before decoding selected function facts.
Both query transports retain their 4 MiB request limit. A complete response,
including escaped JSON, envelope and newline, must fit 24 MiB; otherwise it fails
with `response_too_large`, without a partial successful result.

Check `capabilities.flow_expansion` and the advertised operation/version before
using it. Older engines still support ordinary flows. In-progress expansion and
owned Graphviz layout support cancellation; no partial expanded chart is published.
See [desktop controls and semantic limits](function-flowcharts.md#expand-calls-inside-a-chart).

## Commands and lifecycle

`codecallnavigator-engine capabilities` returns JSON describing the supported
query contract version, operations and limits without requiring a project.

`codecallnavigator-engine query` reads one JSON request from stdin, emits one JSON
response and exits 0 on success or 2 on a structured error. No logging is written
to stdout. A request is at most 4 MiB.

`codecallnavigator-engine query-session` accepts the same request and response
objects as newline-delimited JSON. Requests are serialized; one response is
flushed for every request. EOF exits cleanly. This CLI-owned mode retains parsed
graph data for repeated queries, but checks current artifact bytes and observed
inputs before responding. Project, root mappings or graph changes invalidate the
retained generation. Cancellation terminates the owned process; callers restart
a session when they need another request. The one-shot command and session share
the same query operations and generation contract.

The CLI accepts configured project roots, independently of any MCP implementation.
Clients must check contract compatibility before querying. All normal graph
queries, including C# callers/callees, are supported independently of the desktop.
C# internal function flow is available after refreshing with the current analyzer.
Flow queries return compiler-bound source structure, overload-aware targets and the
exact analyzed UTF-8 snapshot, including a UTF-8 BOM. Existing artifacts without
flow metadata stay graph-readable and return `flow_unavailable` with refresh
guidance. Accepted non-UTF-8 C# files retain their graph and compiler-decoded source
facts; exact-byte flow remains unavailable with an explicit analysis diagnostic.
Unsupported C# control/evaluation constructs appear as diagnosed opaque regions,
rather than guessed execution. Opening a flow never compiles the project.

## Request and response

```json
{
  "version": 2,
  "project": "/checkout",
  "roots": [],
  "query": {
    "kind": "search",
    "text": "create",
    "category": "methods",
    "offset": 0,
    "limit": 50
  }
}
```

`roots` contains `[name, path]` pairs for linked source roots. Relative mappings
are resolved from the project. `repo` is reserved for `project`. The optional
`expected_generation` is the digest obtained from a previous response. A changed
saved graph returns `generation_mismatch`, rather than mixing results. Ambiguous
names return separate search candidates; subsequent queries use their exact
stable symbol IDs, not display names.

Every response contains `version`, `generation`, `freshness`, `result` and `error`.
Success has `error: null`; failure has `result: null` and an error with `code` and
`message`. Failure before loading a graph has null generation/freshness. The
generation is the BLAKE3 digest of the exact saved artifact bytes. Freshness uses
the existing observed-input verifier (`match`, `stale`, `unverifiable`), not a claim
about unsaved buffers or the complete build environment. Its check is an
observation during the request, not a guarantee that source cannot change later.
Stale graphs remain queryable with explicit freshness metadata.
Query freshness includes at most 50 reasons and an 8 KiB encoded reason-array
budget, with `reasons_total` and `omitted_reasons`. The standalone `verify` command
retains its complete report.

Errors include `invalid_request`, `unsupported_version`, `project_unavailable`,
`graph_missing`, `graph_unavailable`, `invalid_graph`, `generation_mismatch`,
`symbol_missing`, `source_unavailable`, `flow_unavailable`, `outside_project` and
`cancelled`. Callers should display the message and use the code for decisions.

## Operations

| Query kind | Fields | Result |
| --- | --- | --- |
| `status` | none | Node/edge counts and diagnostics. |
| `snapshot` | none | Full hydrated `GraphData` for generation-local desktop rendering. |
| `search` | `text`, optional `category`, `offset`, `limit` | Ranked class/method candidates, stable IDs and `more`. |
| `symbol` | `symbol` | Declaration metadata and source location. |
| `source` | `symbol`, optional `origin` | Exact analyzed source by default, or live preview text. |
| `live_source` | `path` | Live preview by file path, including files without a graph symbol. |
| `explorer` | `path`, optional `offset`, `limit` | One live directory page, directories first, symlinks as leaves. |
| `architecture` | Optional `selection`, `previous_signature`, `configuration_root` | Rule selection/signature, change flag and changed layer assignments. |
| `selection` | `selection`, optional `filter`, `limits` | Nodes, stable-ID relationships, roots and omission counts. |
| `render_selection` | `selection`, optional `filter`; requires `expected_generation` | Complete semantic membership for desktop grouping/pagination; traversal safety caps remain. |
| `neighbors` | `symbol`, optional `filter`, `limits` | Both incoming and outgoing depth-one selection. |
| `inspect` | `symbol`, optional `filter` | Complete incoming/outgoing IDs and root-incident relationships; requires generation binding. |
| `flow` | `symbol` | Source IR, semantic chart without layout geometry, analyzed source and diagnostics. |
| `flow_expansion` | `expansion_version: 1`, `symbol`, optional `selections`, `limits` | Generation-bound expanded chart, structural call addresses, occurrences and exact owned source snapshots; see above. |
| `flow_comparison` | `base`, optional `head`, `expected_comparison`; `selection: catalog` or `detail` | Bound saved-revision catalog or paired flows and cached source; see above. |

Search categories are `all`, `classes`, `methods`. Search defaults to 50 results
and accepts 1–200. Class hits carry their build/project/container identity and
`member_count`, without expanding every member identity. Fetch members through a
class `selection` when navigating; the desktop uses generation-bound
`render_selection`. Method hits carry one symbol ID.

`source.origin` is `analyzed` or `live`. Analyzed source is the immutable cached
generation text and preserves byte offsets/line endings. Live preview reads a
regular UTF-8 file with a 16 MiB limit and normalizes CRLF, preserving existing
desktop hover behavior. Its text may differ from analyzed locations.
`explorer` includes hidden, ignored and non-source files, with a default page of
1000 entries and maximum 10000. Live access is restricted to configured roots;
directory entries are not recursively followed. Graph information and live
filesystem observations are explicitly distinguished by `origin`.
Unbound explorer queries also work before the first analysis; their generation
and freshness are null. Supplying `expected_generation` requires a matching saved
graph, as for graph queries.
The same rule applies to unbound `live_source` queries.

Architecture selection accepts `None`, `Spring`, `React`, or `Repository`.
Omitting selection automatically detects repository rule files. The default
configuration root is the analyzed project; `configuration_root` preserves the
terminal client's explicit layer-root override. Classification uses immutable
analyzed facts and current configuration, cached inside the CLI by generation,
configuration root and rule signature. An unchanged `previous_signature` yields
`changed: false` and null assignments. Invalid rules return an error, preserving
the last valid consumer view.

Review-app configuration is converted in memory; architecture queries never
create or persist native configuration files.

`render_selection` accepts an optional `layer` object with `selection`, `tab`,
`configuration_root` and `expected_signature`. This applies layer membership in the CLI and returns
`architecture_signature`. A mismatched rule signature returns
`architecture_changed`; consumers reload assignments before combining results.

Selection kinds:

```json
{"kind":"overview"}
{"kind":"entry_points","direction":"Outgoing","depth":2}
{"kind":"neighborhood","symbols":["stable-symbol-id"],"direction":"Incoming","depth":2}
{"kind":"path","from":"stable-from-id","to":"stable-to-id"}
{"kind":"file","file":"/checkout/src/Example.java"}
{"kind":"class","build":"/checkout/pom.xml","project":"Example","container":"Example.Service"}
```

Directions are `Incoming`, `Outgoing` and `Both`. Maximum depth is 64. Selection
limits default to and cannot exceed `{"nodes":1500,"edges":6000}`. Traversal
retains roots first; path nodes remain in path order. An absent directed path is
an empty successful selection. Overview/file/class results use stable-ID order.
Results report `omitted_nodes`, `omitted_edges` and `truncated`. Never interpret a
bounded result as a complete workspace. The explicit `snapshot` operation is the
full-data exception for consumers that need complete presentation data.

Render results additionally carry `snapshot_nodes` and `snapshot_edges`, exact
offsets into the generation-bound snapshot, and `summary` with full-graph
`cyclic_symbols`, `hidden_nodes` and `hidden_edges`. These offsets are local to
that exact generation and must never be persisted as semantic identities.
Inspector results also carry `snapshot_edges`, preserving parallel relationships
without guessing identity from endpoints. Unlike bounded `neighbors`, `inspect`
includes only root-incident relationships, not induced edges between neighbors,
and leaves pagination to the consumer.
For semantic presentation membership, `render_selection` returns complete
filtered overview/file/class results so desktop group counts and pagination are
not limited by the agent response budget. It shares the same selector as bounded
`selection`; paths retain path order and neighborhoods retain GraphIndex's
intrinsic traversal caps and omission counts. This operation requires an exact
`expected_generation` and is intended for desktop rendering, rather than agent
context injection.

Filters use the shared graph filter contract: code inclusion (`include_tests`,
`include_generated`, `include_unavailable`), independent `relationship_kinds` and
`target_resolutions`. Omit `filter` to use shared core defaults. Explicit filters
require `version: 2` and both relationship selection arrays; legacy saved-filter
upgrade/reset behavior is not applied to public requests.

Status, selection and inspector responses summarize global diagnostics with at
most 50 messages and 8,192 returned characters. `diagnostics_total`,
`omitted_diagnostics` and `shortened_diagnostics` make the limits explicit.
Per-message prefixes end in an ellipsis when shortened. Full snapshots retain
every original diagnostic.

Public semantic graph results use symbol strings for edge endpoints and retain
kind, certainty, provenance, evidence, count and classification. Snapshot edges
use numeric offsets only within that snapshot generation; never persist or send
those offsets as symbol identities. Flow chart step/link indices are local to
the returned function chart, while call targets use stable graph symbol IDs.
Flow describes possible source execution, not observed runtime behavior.
Flow source ranges must match the exact cached UTF-8 source and its 16 MiB limit.
Chart serialization excludes geometry; clients may deserialize and lay out the
returned semantic chart without rebuilding its control flow.

## Example shell use

```sh
codecallnavigator-engine capabilities
codecallnavigator-engine query <<'JSON'
{"version":2,"project":"/checkout","query":{"kind":"status"}}
JSON
```

Clients should keep graph selection and provenance policy in the CLI. Camera,
layout, rendering, source highlighting and navigation history belong to the
consumer. An MCP adapter uses these commands and must not inspect graph storage
or reproduce semantic query logic.

## Saved-input freshness observations

Check `capabilities.freshness` before using observation version 1. The one-shot
request is `{"version":2,"project":"/checkout","query":{"kind":"freshness","observation_version":1}}`.
Its normal response envelope contains a typed `result` with `generation`,
`input_status` (`match`, `stale`, or `unverifiable`), bounded contextual `reasons`,
`input_revision`, `coverage`, `mode`, and `verification_milliseconds`. Existing
query envelopes and the `status` operation keep their existing behavior. An
optional `expected_generation` rejects observations of another saved artifact.

`ccnavigator observe` reads that request as one newline-delimited JSON line and
keeps observing while stdin remains open. It emits NDJSON events with `version`,
monotonic `sequence`, `kind` (`initial`, `change`, `heartbeat`, or `failed`), and
an `observation` or structured `error`. An initial observation is followed by
changes to its state and a heartbeat after five seconds without a notification.
The CLI rechecks saved artifact bytes each pass; replacing an artifact changes
its generation. A generation-bound monitor fails when that generation changes.
Failures terminate the monitor and never attach an observation from another
generation. EOF, signals, and broken output pipes terminate the owned process.

Version 1 explicitly reports limited `verified_polling` mode. Reconciliation is
sequential at a 250 ms minimum interval between starts; expensive passes extend
the interval and never overlap. Passive observation has no native filesystem
watcher or analysis trigger. Each observation verifies actual bytes rather than trusting timestamps.
The revision additionally hashes file contents recursively below recorded
directory scopes, including new TypeScript files and later repairs, preserving
each scope's own output exclusions. Explicit generated inputs remain observed.
Generated graph outputs, excluded build directories and Maven's generated local
repository marker do not trigger changes. Volatile revisions and timings are
never written into canonical artifacts.

`coverage.complete` means the currently recorded observable scopes were readable
and traversable in that pass. Missing mapped roots, unreadable/nonregular inputs,
and nested directory symlinks that were not explicitly recorded as scopes prevent
complete coverage. Regular symlink targets and link identity are observed;
nonregular targets such as devices and FIFOs are rejected without reading them.
The Unix implementation uses a nonblocking open and verifies the opened file's
type before hashing to prevent a raced symlink target from blocking observation.
Regular-file hashing checks shutdown between chunks. `certifies_build` is always
false: recorded inputs do not certify unsaved buffers, unrecorded dependencies,
absent conditional imports, or external empty globs. Legacy incomplete C# input
policies remain unverifiable until an explicit refresh.

Notifications represent current state and are not a durable change journal.
A bounded output queue coalesces state under backpressure while reconciliation
continues. Intermediate changes can be omitted. Consumers should adopt the latest
observation by generation and revision, and restart after a failed event. EOF
remains bounded even if the client stops draining stdout. Passive observation
never compiles, publishes, or writes caches.

## Opt-in automatic refresh

Check `capabilities.automatic_refresh` before starting
`ccnavigator observe --auto-refresh`. Version 1 advertises `activity_version: 1`,
`initial_session_version: 1`, `debounce_milliseconds: 300` and
`cancel_command: "cancel_refresh"`. Keep stdin open and send an unpinned
freshness request to follow the saved build selection. To start with an
explicit selection, use the [initial analysis session envelope](standalone-engine.md#executables-and-process-protocol).

The engine owns one session for the project and coalesces pending revisions.
A shared publication lock protects manual and automatic work. The session
retains valid compiler preparation and checks the current generation and
compiler inputs before publishing. Contention and requeued input changes use
bounded backoff. An unchanged compiler failure or cancelled revision waits
for another relevant saved edit.

A saved-graph stream begins with an `initial` freshness observation. Explicit
initial analysis can report activity before publication, followed by the first
real freshness observation. The `kind: "activity"` frame carries the cumulative
`runs_started` and a tagged state. States include `watching`, `debouncing`,
`running`, `busy`, `published`, `failed`, `cancelled` and `unavailable`.
Published activity includes the resulting generation, effective root mappings,
attempt duration and compiler metrics. Running timing separates configured
debounce from queue wait. A running frame may include `progress_message`, the
latest stage reported by the shared analysis pipeline. Activity frames can
coalesce. Freshness verification
and generation-bound graph snapshots establish what clients can display.

Send a later line `{"version":1,"command":"cancel_refresh"}` to cancel the
current attempt and pending revision. EOF, SIGINT, SIGTERM and broken output
pipes stop the session and clean up owned processes. The engine reports any
cleanup it cannot confirm.

Saved-graph automation requires a complete, supported exact build selection.
Java uses evaluated compiler input and bytecode output bindings. Generated
sources and dependency bytecode remain inputs. A changed owned class output
can trigger graph extraction without invoking the build. The public freshness
observation still verifies the recorded outputs. Missing or unsupported
selection or binding metadata can be regenerated with explicit analysis.

## Flow inventory residency

Query contract version 2 separates structural snapshots from flow inventory.
Search, navigation, architecture, source inspection, status and freshness do not
load function bodies. `flow` and `flow_expansion` load flows automatically;
`query-session` reuses them until unload, generation replacement or shutdown.
One-shot queries release all resources on exit.

For a persistent client, send a generation-bound operation:

```json
{"version":2,"project":"/checkout","expected_generation":"<manifest-digest>","query":{"kind":"flow_resources","action":"unload"}}
```

Actions are `load`, `unload` and `status`. Load and unload are idempotent; status
never loads. The result contains `loaded`, `functions`, `retained_bytes` (an IR
allocation estimate, not process RSS) and `load_milliseconds`. The CLI owns no
idle policy. The desktop requests unload after all flow views have been hidden
for 60 seconds, clearing hidden charts and expansion caches at the same time.
Reopening a flow transparently reloads it.

Structural snapshots contain no flow bodies. Errors loading flow objects affect
flow requests; validated structural navigation remains usable. All related
requests retain the same manifest generation binding.

Ordinary-query freshness `contract_valid` covers the manifest and structural
content validated for that query. Flow integrity and references are checked when
flows load. Use the explicit `verify` command to validate both saved objects.

## Input scanning policy and recovery

New analyses record the `scoped_v1` input-observation policy. Broad discovery
and recursive directory observations skip dot-prefixed directories and symlinks;
dot-prefixed files remain eligible. Compiler/build-declared inputs and explicitly
selected roots remain tracked, including required inputs beneath hidden folders
and required link targets. Changes to ignored entries do not trigger refresh.

Graphs without this policy remain readable using their original fingerprint
semantics. Automatic refresh requests one manual **Refresh** to regenerate input
records; clearing the cache is unnecessary. Read-only queries never upgrade graphs.

Incomplete observations retain the last successful graph and block automatic
scheduling. Reasons include a stable `code`, optional `path`, and optional
`operation`, `detail`, and `recovery` fields. Older clients may ignore the additional
fields. Reasons are deduplicated and blockers precede ordinary changed-input notices;
bounded responses report total and omitted counts. Run
`ccnavigator verify --project /absolute/project/path` for the complete read-only
report. The desktop exposes scrollable, selectable details and **Copy details**.
**Retry freshness** retries observation; **Refresh** reruns analysis. Restore missing
required paths, link targets, permissions, or root mappings before retrying; when
inputs were intentionally removed by a build change, perform a manual Refresh.

A known deletion or rename is an observable stale state (`input_missing`), so it
can trigger reanalysis. It is distinct from an unreadable input or unresolved
required symlink. If the compiler still requires a deleted input, analysis fails
and preserves the previous graph. Coverage refers to recorded observable scopes;
unmapped build-environment limitations remain explicitly `unverifiable` and
`certifies_build` remains false even when those scopes are completely observed.
