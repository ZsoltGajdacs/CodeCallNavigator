# Public CLI graph queries

## Git graph comparison

Both `ccnavigator` and `codecallnavigator-engine` support saved-graph comparison
through `query` and `query-session`. Graph refresh and committing the graph are
separate operations. For example, send this JSON to `ccnavigator query`:

```json
{"version":1,"project":"/checkout","query":{"kind":"compare","base":"main","head":"WORKTREE","offset":0,"limit":100}}
```

`base` must resolve to a commit; `head` defaults to `WORKTREE` (the saved graph,
including uncommitted graph changes), or accepts another commit, branch or tag.
This compares endpoints directly, without merge-base calculation. Git blobs are
read at the project's repository-relative `.codecallnavigator/graph.json` path;
no checkout, fetch, compilation, publication or cache write occurs. A renamed
artifact path in history is not searched automatically. Git commands have a
30-second deadline and share the query process group for cancellation.

The version-1 result contains `base` and `head` endpoint metadata (`kind`,
`requested_ref`, resolved `commit`, repository-relative `path`, `schema`, exact-byte
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
{"version":1,"project":"/checkout","query":{"kind":"compare","base":"main","offset":100,"limit":100,"expected_comparison":"<64-character comparison_id>"}}
```

Every page rechecks refs and artifact bytes. Moving a ref, even to a commit with
identical graph bytes, invalidates the binding. `expected_generation` optionally
binds the target artifact digest. The envelope generation always identifies that
target. Working targets include normal source freshness and permit stale saved
graphs; committed targets have null freshness and need no working artifact or
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
C# internal function flow remains unsupported and returns `flow_unavailable`.

## Request and response

```json
{
  "version": 1,
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
{"version":1,"project":"/checkout","query":{"kind":"status"}}
JSON
```

Clients should keep graph selection and provenance policy in the CLI. Camera,
layout, rendering, source highlighting and navigation history belong to the
consumer. An MCP adapter uses these commands and must not inspect graph storage
or reproduce semantic query logic.
