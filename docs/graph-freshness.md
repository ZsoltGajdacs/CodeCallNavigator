# Saved-edit graph freshness

CodeCallNavigator observes saved analysis inputs through verified polling. Event-based
monitoring, unsaved editor buffers and cross-edit compiler reuse are outside the
supported observation contract.

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

## Observation and automatic refresh

The desktop observes the loaded project and shows checking, matching, stale,
unverifiable, unsupported and failed states separately from analysis activity.
A newer saved generation offers **Reload saved graph**; reloading reads saved
artifacts and does not run analysis.

**Enable automatic refresh** explicitly opts the currently loaded project into
compiler-backed refresh for this application session. It is off at startup and
after switching projects. Editing the pending folder does not move that authorization.
The engine coalesces saved changes and owns at most one analysis per project.
Invalid code, cancellation and changed inputs preserve the last successful graph.
After a failed refresh it waits for a relevant new saved-input revision or an
explicit request instead of repeatedly compiling the same invalid state.

**Cancel automatic refresh** cancels current/pending work while leaving monitoring
enabled. **Disable automatic refresh** returns to passive observation. Manual
refresh remains available. Upgrading an older compatible CLI may be needed for
freshness or automatic-refresh capabilities.

Queries remain bound to one artifact generation. A generation mismatch requires a
new status/snapshot before combining results; prior IDs cannot be silently reused.
Verification is point-in-time and covers recorded inputs, not the entire build
or runtime environment. Small-fixture detection measurements are not a latency
guarantee for every project.

See the [CLI observation and refresh contract](cli-queries.md#saved-input-freshness-observations)
and [MCP freshness tool](mcp.md#responses-refresh-and-cancellation).
