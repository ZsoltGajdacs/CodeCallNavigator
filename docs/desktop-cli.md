# Desktop query backend

The desktop obtains analyzed information through the engine executable's
[public query commands](cli-queries.md). It does not read canonical graph files or
SQLite graph generations to answer production queries.

## Ownership and data flow

The process client separates analysis/discovery lifecycle handling from a
serialized persistent `query-session` connection. Typed operation adapters obtain
search candidates, complete semantic render selections, full-graph filter/cycle
summaries, inspector relationships, function flow, source, architecture and
freshness. Graph queries use exact stable symbols and generation-local offsets
from the same returned snapshot. The desktop retains a `GraphIndex` for identity
lookup and rendering, rather than executing semantic traversal against it.

Layout, grouping, presentation-level group traversal, camera behavior, navigation
history and source highlighting remain desktop responsibilities. Focused group
views consume a complete filtered overview before grouping so group-hop behavior,
class counts and pagination do not depend on the agent response budget. The
CLI supplies code/layer membership and relationship classifications.

Function flow is built by the CLI and laid out by the desktop. Source previews
request live files through the CLI; function charts retain immutable analyzed
source. Explorer browsing requests live directory pages, including hidden and
ignored files, without recursively following directory symlinks.

## Project and generation addressing

A loaded graph owns an immutable query context containing its canonical project,
linked-root mappings and generation digest. Editing the folder field changes
build discovery and explorer browsing; it does not redirect queries or previews
for the graph that remains displayed. Each analyzed query is generation-bound.
A replaced canonical artifact produces an explicit mismatch and requires reload
or refresh before results can be combined.

Navigation/settings remain in the desktop store. Saved sessions separately retain
the editable folder, loaded graph context and profiling-cache marker. Startup
requests a CLI snapshot using the saved context, then retains only known navigation identities.
Malformed navigation can be recovered independently: a valid saved graph context,
or a legacy saved root, still addresses the canonical graph through the CLI.
Unaddressable legacy graph caches require a project selection/refresh; production
startup does not fall back to querying their SQLite contents.

Analysis completion supplies the publication digest and root mappings; the
desktop requests that exact CLI snapshot. It retains the returned CLI snapshot
in a write-only presentation cache for isolated profiling and rollback. This is never an alternate graph-query backend.
The canonical CLI digest remains the UI generation; a separate
`presentation_cache` marker records `source_generation` (that digest) and
`cache_generation` (the retained SQLite filename). Startup, background analysis
and the headless desktop command share this retention helper. Matching source,
physical project/root context and active cache metadata avoid republishing the
same snapshot. A retention failure reports the failed cache operation explicitly.
A failed/cancelled analysis preserves the displayed last successful graph.

## Responsiveness and failures

Requests run off the UI thread. The persistent CLI connection serializes queries;
waiting for the connection, writing its bounded request and reading its response
all observe cancellation and a deadline. Cancellation or transport failure
terminates and discards the connection, so an old reply cannot satisfy a later
request. UI jobs also reject obsolete epochs, revisions, search serials and source
preview serials. Large initial snapshots have a 120-second deadline.

Before starting a connection, the client verifies the public query version and
all required advertised desktop operations. A configured older engine reports an
explicit capability error. Layer render requests carry the classification
signature matching the displayed assignments; configuration changes preserve the
last view until classification and rendering agree again.

## Explicit fixtures and verification

Scene/test queries use the named `fixture_queries` adapter only after an explicit
fixture/scene choice. It is not a fallback for missing production context or CLI
errors. Saved-canvas benchmarks read the isolated copied profiling store through
that adapter, preserving the selected saved generation and navigation without
querying a live project's current artifact.

The process-backed Rune acceptance test is
`ui_tests::cli_backend::actual_cli_keeps_graph_context_across_edited_root_and_drives_search_source_flow`.
It analyzes a real TypeScript project, uses the production CLI backend, drives
Double Shift search and method selection, inspects relationships, changes the
editable folder, opens original-project live source, and opens/returns from
function flow. Set `CODECALLNAVIGATOR_CLI_UI_CAPTURES` to an output directory for
optional dark/light native flow captures; Metal access is required for captures.
