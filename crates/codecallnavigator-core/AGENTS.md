# Shared core practices

Read the workspace practices and mandatory policies in
[AGENTS.md](../../AGENTS.md). This guide applies to `codecallnavigator-core`.

## Responsibilities

Core owns the shared graph model, method policy, graph/flow/architecture
operations, boundary resolution, artifact identities and publication, process
protocol types, discovery, fingerprinting, metrics and storage support.
Keep these usable without the desktop or a language analyzer. Core also contains
filesystem, SQLite and Graphviz integration; isolate those effects from graph
transformations rather than treating the entire crate as pure.

- Do not add dependencies on the root desktop package, `codecallnavigator-engine`, Rune
  or WGPU. Keep compiler/build-tool execution in the engine.
- Put cross-language contracts and reusable semantic decisions here. Keep
  language/framework fact extraction in its adapter and UI projection in the
  desktop. Shared boundary matching consumes normalized facts.
- Keep model, validation, transformation and persistence responsibilities
  separate. Prefer explicit types and focused operations over a general service
  abstraction; expose only what consumers need.

## Contracts and correctness

- Preserve the [canonical artifact and protocol contract](../../docs/standalone-engine.md).
  Relationships and flow references use stable identities, not array positions.
  Resolve every exported reference through the same identity mapping.
- Keep canonical output deterministic: sort unordered collections while retaining
  control-flow order. Exclude checkout-specific paths, temporary paths, volatile
  observations and timings; use logical repository/root mappings.
- Reject collisions, invalid references and unmapped owned paths before
  publication. Preserve the last successful artifact on failure or cancellation;
  publish atomically under the project lock.
- Treat the desktop SQLite store as a derived cache, not canonical graph history.
  Consider schema versions, old serialized data and cache invalidation whenever
  changing shared models or persistence.
- Preserve uncertainty and evidence during graph transforms and round trips.
  Unsupported facts can be ambiguity blockers: dropping one must not invent a
  unique relationship. See the [boundary contract](src/boundary/README.md).
- Keep graph algorithms testable with explicit inputs. Confine filesystem and
  database access to their owning modules, and return contextual failures.

## Testing strategy and verification

Follow [the shared strategy](<../../.backlog/docs/testing/doc-12 - Testing-strategy.md>). Unit tests should
carry most graph/flow algorithm, identity, canonicalization, transformation and
validation cases using explicit in-memory inputs. Test real production operations,
not copies of implementation logic. Fixture loading alone does not define category.

Use integration tests for filesystem discovery, Git revisions, SQLite persistence,
publication locks/atomic replacement and Graphviz. Exercise real temporary resources
and verify observable artifacts, failure recovery and unchanged prior data. Core
has no standalone application journey; engine, desktop and MCP consumers prove
assembled behavior. Shared contract changes require relevant consumer checks.

```sh
./verify-changes.sh
./verify-changes.sh --run
./verify-changes.sh --base origin/main --run
```

Run from the workspace root and inspect the report-only command for Git/SQLite/Graphviz and
other selected prerequisites. Category scripts clean owned Rust outputs and
temporary resources after success or failure, preserving evidence and downloaded
caches. `--keep-artifacts` is an explicit diagnostic exception.

Do not mock business behavior or fabricate successful dependency results. Narrow,
documented process/protocol fault probes are integration tests only. Explain why
new expensive tests cannot prove their risk at a cheaper layer. Before removing
one, map its assertions to retained coverage and demonstrate defect detection.
Roughly one minute of warm unit execution is aspirational, never a failure gate;
quality and unique boundary coverage take precedence. Follow root full-verification,
CodeScene and scoped-formatting requirements.

Agents must use `./verify-changes.sh` for verification selection and
`./verify-changes.sh --run` for all test and verification execution. Do not
substitute raw test commands or manually selected category/component scripts.
Execute the complete computed plan; full verification is selected automatically
by checked-in rules, with no manual scope override. Use `--base REF` to include
committed changes. Run automatically selected full verification outside the macOS
sandbox. Report unavailable prerequisites as not run, never as passing checks.
