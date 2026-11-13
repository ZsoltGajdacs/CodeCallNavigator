# Portable graph JSON contract

CodeCallNavigator publishes one canonical artifact at `.codecallnavigator/graph.json`. External
consumers, including CodeQualityChecker, may read it without linking CodeCallNavigator's
Rust crates or invoking an analyzer. The wire shape is specified in
[graph.schema.json](graph.schema.json), using JSON Schema Draft 2020-12. The
[engine guide](standalone-engine.md) describes production and comparison commands.

## Compatibility policy

The envelope is `{ "schema": "codecallnavigator.graph.v1", "graph": { ... } }`.
`codecallnavigator.graph.v1` is a major compatibility boundary, independent of the desktop
release version and the engine process protocol version. Consumers must reject
unsupported versions explicitly. Within v1, existing field meanings, wire types,
and identity construction stay fixed. Adding optional properties is compatible;
removing/changing fields or their semantics, changing symbol identity construction,
or expanding a closed discriminator requires a new major contract version.

Ignore unknown object properties, including properties nested inside inventories
and evidence. Do not infer semantics from their names. The artifact representation
preserves unknown properties; converting to the runtime model and re-exporting
need not retain them. Unknown closed discriminators and unsupported inventory
versions are errors. Relationship `kind`, `certainty`, `provenance`, and node
`kind`/`language` are string vocabularies: tolerate unfamiliar strings, preserve
them, and treat unfamiliar certainty conservatively. The closed values currently
include visibility, evidence semantics, flow statement/expression/call-target
kinds, boundary kinds/frameworks, and analysis input kinds.

Legacy formatted v1 artifacts remain readable without regeneration. New
production output is automatically compact UTF-8 JSON, with one trailing newline.
Loading and verification do not rewrite a file; the next successful analysis
replaces it. Digests identify exact file bytes, so whitespace changes a digest
but does not change the graph. Use `codecallnavigator-engine compare --project PATH
--base REF --head REF|WORKTREE` for semantic comparison rather than raw line diffs.

The schema validates shape and known discriminators. CodeCallNavigator's semantic
validator additionally checks unique identities, reference integrity, portable
path safety, positive relationship counts, evidence constraints, and supported
inventory versions. JSON Schema alone cannot establish cross-reference integrity.

## Graph structure

| Section | Meaning and presence |
| --- | --- |
| `nodes` | Required array of symbol declarations. May be empty. |
| `edges` | Required array of directed static relationships. May be empty. |
| `diagnostics` | Required array of analysis explanations/limitations; not a completeness certificate. |
| `roots` | Required array of logical source-root paths used by analysis. |
| `entry_points` | Optional or null. Contains candidates, framework entries, and compiler/bytecode-verified application starts. |
| `flow_inventory` | Optional or null, version 1. Ordered source-derived function logic. |
| `architecture` | Optional or null, version 1. File/declaration facts, including source content. |
| `boundaries` | Optional or null, version 1. Routes and boundary uses. |
| `identity_roots` | Optional array of compiler context roots involved in identity. |
| `folder_project_builds` | Optional array of build owners whose project labels derive from directories. |
| `analysis` | Optional or null, version 1. Portable observed-input provenance for verification. |

Nodes have opaque unique `id` strings, source `file` and `line`, ownership
(`build`, `project`, `namespace`, `container`), presentation (`label`, `signature`,
`kind`, `language`), and `is_test`, `is_generated`, `body_unavailable` flags.
`visibility` and `container_visibility` may be absent or null, meaning unknown or
not applicable. Other scalar fields are required unless the schema says otherwise.
A zero node line or empty path can represent unavailable data. Source evidence
sites require positive lines, and optional positive columns. Source spans use
half-open UTF-8 byte offsets into the analyzed source snapshot.

Every edge's `from` and `to` is a node ID, never a node-array index. Multiple edges
can connect the same endpoints with different kinds, certainty, provenance or
evidence. Do not deduplicate solely by endpoints. `count` is positive and describes
static relationship occurrences, not runtime frequency. `provenance` may be absent
(default empty string), and `evidence` may be absent (default empty array).

`direct` and `proven-direct` certainty distinguish direct relationships; other
certainty values must be handled conservatively. References, deferred callbacks,
and possible invocations are distinct. Registration alone does not establish
execution. Evidence records preserve a versioned rule, invocation site,
optional registration site, supporting symbol IDs, semantics, optional channel,
and opaque string qualifiers. Qualifier text is not a symbol/path reference.

Entry point lists and flow call targets refer to the same symbol IDs as edges.
Flow inventories retain statements, expressions, source spans, and diagnostics;
these are source-derived logic rather than observed execution. Architecture facts
contain source text and should be treated as project source information when
sharing artifacts. Unknown, absent, or unsupported facts must not be converted
into claims that no dependency or quality problem exists.

## Identity, paths, and ordering

Treat IDs as opaque strings. They are stable across array reordering and checkout
relocation with equivalent logical root mappings and equal analysis inputs.
Unrelated edits to named declarations should not change their IDs. Ownership,
file moves, signatures, and anonymous-function bodies can change IDs. Ambiguous
anonymous/lambda attribution can retain less-stable native identity accompanied by
diagnostics. Do not interpret identity changes alone as quality regressions.

Owned paths use `@repo/relative/path` or `@NAME/relative/path` for additional logical
roots; `@repo/` denotes the output-owning project. Additional root names must be
consistent across checkouts. Empty paths can indicate unavailable ownership.
Do not assume logical paths exist locally without resolving their root mapping.
Absolute checkout paths and temporary paths are not portable path values.

Known unordered collections are sorted deterministically, including nodes, edges,
evidence, entry points, architecture files and sets of declaration facts. Statement
bodies, expression sequences, branch bodies, and switch cases preserve source
order. Neither array positions nor JSON property order are identifiers. Unknown
extension properties and arrays are opaque and are not transformed or sorted.

## Observed-input provenance and verification

New production analyses add `graph.analysis` with:

- `version: 1`;
- `targets`: selected build targets as logical paths;
- optional `selection`: exact selected build identities, described below;
- `roots`: logical root names;
- `engine_versions`: available analyzer/toolchain version information;
- `inputs`: sorted records with logical `path`, `kind` (`file`, `directory`, or
  `missing`), `hash`, and optional `excluded_children`;
- `limitations`: records containing a nonempty `code` and positive `count` for
  inputs that cannot be represented/verified completely.

File hashes describe content; directory hashes describe observed child names with
build-output exclusions. Missing records carry an empty hash; file/directory
records carry lowercase 64-digit BLAKE3 hashes. Timestamps, Git HEAD, absolute
paths and runtime performance metrics are excluded. Matching provenance verifies
recorded observed inputs, not dependency availability or a complete reproducible
build environment. New files are detected through recorded directory listings;
inputs outside recorded discovery scope are not implicitly covered.

New analyses also record `selection: {"version":1,"complete":true,"targets":[...]}`.
Each target has portable `path`, `kind`, and `language`. This preserves distinct
Maven and Gradle selections at the same directory, including graphs with no
symbols. The existing `targets` path set keeps its original meaning. Selection
records are ordered deterministically; unknown extension fields remain intact.
An unmapped target makes `complete` false; omitted `complete` means false.

Automatic refresh requires a complete, supported exact selection. It never
reconstructs build intent from current discovery defaults, graph nodes, or the
legacy path set. Older artifacts need one explicit manual refresh to record it.
Future selection versions remain opaque to ordinary loading, querying and
round-tripping; automatic refresh reports them unavailable. Selection metadata
does not contain machine paths, monitor state, counters, or performance metrics.

Java analyses additionally record optional `refresh_inputs` version 1 with
`complete`, `binding_policy: "direct.v1"`, `owned_bytecode_directories`, and
`directories`. The output paths come
from evaluated compiler source-set class directories, including custom output
locations. Directory records reuse the input format to save filtered membership
only where an owned output is an immediate child. Each original input's exclusion
policy is retained; sibling generated sources and dependency bytecode remain
inputs. These hashes are stable provenance, not monitor state.

This metadata supplies an internal scheduling view. Full freshness verification
and its public `input_revision` still include the owned compiler outputs. Missing,
unsafe or future ownership metadata makes Java automatic refresh unavailable;
ordinary queries remain supported. Source/configuration overlap and ambiguous
mixed-language ownership require manual refresh rather than hiding real inputs.
Dependency classpaths must remain separate from owned outputs except for exact
aliases of another selected source set's own classes. The adapters retain raw
evaluated output bindings so symlink-bearing bindings can disable automation;
observation also rejects a later symlink substitution. These runtime binding
checks add no machine paths or volatile state to the canonical artifact.
Automatic refresh requires the known binding-policy marker. Older experimental
metadata without it and future policy strings remain readable by ordinary graph
consumers but require explicit manual refresh before automation can use them.

Run the read-only command:

```sh
codecallnavigator-engine verify --project /checkout --root shared=/other/checkout
```

The command reads the saved artifact and checks its recorded inputs without
running compilers or writing artifacts/caches. Its versioned JSON report has
`version`, `contract_valid`, `input_status`, `digest`, and `reasons`; reason records
contain a `code` and optional portable `path`. The digest covers actual saved file
bytes, including any formatting. A malformed/unsupported artifact fails contract
validation; artifact-read and command failures are operational errors.

| Input status | Meaning | Exit |
| --- | --- | --- |
| `match` | All recorded inputs match and no verification limitations remain. | 0 |
| `stale` | Observed content, listings, or missing/present state changed. | 1 |
| `unverifiable` | Legacy metadata is absent, mappings/inputs are unavailable, or recorded limitations prevent verification. | 1 |
| Invalid artifact or operational error | No successful verification. | 2 |

When both stale inputs and limitations exist, report `stale` and retain the
limitations in reasons. Changing filesystem inputs during analysis prevents
publication. Failed or cancelled analysis preserves the prior artifact; a saved
file's existence does not mean the most recent attempt succeeded. A consumer
requiring freshness must inspect verification results explicitly.

## Consumer acceptance checks

The independent Python consumer reads ordinary JSON, validates the published
schema, resolves edge IDs, and preserves certainty, evidence, counts, and
diagnostics without hydrating Rust models. It accepts retained compiler fixtures
and fresh output:

```sh
python3.13 -m venv /tmp/codecallnavigator-schema-check
/tmp/codecallnavigator-schema-check/bin/pip install 'jsonschema>=4.18,<5'
/tmp/codecallnavigator-schema-check/bin/python -m unittest discover -s tools -p test_graph_contract.py
/tmp/codecallnavigator-schema-check/bin/python tools/check_graph_contract.py fixtures/.codecallnavigator/graph.json /checkout/.codecallnavigator/graph.json
```

`jsonschema` is a test-only dependency. These checks complement CodeCallNavigator's
semantic, compiler-fixture, relocation, cancellation, and publication tests.
