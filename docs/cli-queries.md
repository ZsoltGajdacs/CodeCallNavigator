# Public CLI graph queries

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
| `explorer` | `path`, optional `offset`, `limit` | One live directory page, directories first, symlinks as leaves. |
| `selection` | `selection`, optional `filter`, `limits` | Nodes, stable-ID relationships, roots and omission counts. |
| `render_selection` | `selection`, optional `filter`; requires `expected_generation` | Complete semantic membership for desktop grouping/pagination; traversal safety caps remain. |
| `neighbors` | `symbol`, optional `filter`, `limits` | Both incoming and outgoing depth-one selection. |
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

Public semantic graph results use symbol strings for edge endpoints and retain
kind, certainty, provenance, evidence, count and classification. Snapshot edges
use numeric offsets only within that snapshot generation; never persist or send
those offsets as symbol identities. Flow chart step/link indices are local to
the returned function chart, while call targets use stable graph symbol IDs.
Flow describes possible source execution, not observed runtime behavior.

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
