# Reading this graph

`graph.json` is a manifest. Validate it with `graph.manifest.schema.json`.
`graph.schema.json` describes the reconstructed graph, not compressed objects.

1. Read each manifest part from `objects/<digest>.structure.json.zst` or
   `objects/<digest>.flows.json.zst`.
2. Verify its compressed byte count and BLAKE3 digest, then decompress with
   Zstandard and verify the uncompressed byte count.
3. Parse the structure as the graph envelope. Parse flows into
   `graph.flow_inventory` and remove `graph.flow_catalogue`.
4. Validate the resulting envelope with `graph.schema.json` (JSON Schema
   Draft 2020-12). Reject unsupported schema versions.

Symbol IDs are opaque; edges reference IDs, not array positions. Source paths use
logical roots such as `@repo/`. Relationships describe static facts with explicit
certainty, not observed execution. Schema validation does not establish reference
integrity or freshness.

These companion files are generated and replaced on successful analysis or
refresh. Graph reads do not create or repair them. Copy the manifest and its
referenced objects together.

See the [full graph contract](https://github.com/ZsoltGajdacs/CodeCallNavigator/blob/main/docs/graph-contract.md)
for field semantics and compatibility rules.
