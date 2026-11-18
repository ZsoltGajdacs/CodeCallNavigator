# Saved-edit graph freshness

CodeCallNavigator observes saved analysis inputs through verified polling.
Continuous analysis reuses validated compiler work across saved edits.

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

Choose **Enable automatic refresh** in Analysis details to follow the loaded
graph's build selection. For a fresh checkout or a changed selection, choose the
builds first and select **Analyze with automatic refresh**. The engine starts
analysis and watches saved edits before the first graph is available. Authorization
lasts for the current application session. Selecting a different project or build
set for continuous analysis requires a new opt-in.

The engine coalesces saved changes and reuses valid compiler preparation. A
relevant change during analysis queues another attempt. Invalid code and
cancellation preserve the last successful graph. The engine waits for a relevant
saved change after a compiler failure.

**Cancel current attempt** leaves the session available for later edits.
**Disable automatic refresh** stops the session and returns to passive
observation of a loaded graph. Manual analysis remains available.

Queries remain bound to one artifact generation. A generation mismatch requires a
new status/snapshot before combining results; prior IDs cannot be silently reused.
Verification is point-in-time and covers recorded inputs, not the entire build
or runtime environment. Small-fixture detection measurements are not a latency
guarantee for every project.

See the [CLI observation and refresh contract](cli-queries.md#saved-input-freshness-observations)
and [MCP freshness tool](mcp.md#responses-refresh-and-cancellation).

## Input scanning policy and recovery

Broad scans skip dot folders and symlinks; explicitly required build/compiler
inputs remain tracked. Older graphs need one manual **Refresh** before automatic
refresh can use the new policy. The previous graph is preserved on failure.

When required inputs cannot be observed, open freshness details for paths, causes,
and recovery instructions, or use **Copy details**. **Retry freshness** checks
inputs again; it does not rebuild the graph. See the
[CLI input policy and recovery contract](cli-queries.md#input-scanning-policy-and-recovery)
for compatibility and complete-report instructions.
