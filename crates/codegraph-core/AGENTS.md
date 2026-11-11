# Shared core practices

Read the workspace practices and mandatory policies in
[AGENTS.md](../../AGENTS.md). This guide applies to `codegraph-core`.

## Responsibilities

Core owns the shared graph model, method policy, graph/flow/architecture
operations, boundary resolution, artifact identities and publication, process
protocol types, discovery, fingerprinting, metrics and storage support.
Keep these usable without the desktop or a language analyzer. Core also contains
filesystem, SQLite and Graphviz integration; isolate those effects from graph
transformations rather than treating the entire crate as pure.

- Do not add dependencies on the root desktop package, `codegraph-engine`, Rune
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

## Verification

From the workspace root, begin with:

```sh
cargo check --locked -p codegraph-core --all-targets
cargo test --locked -p codegraph-core --all-targets
```

For model/artifact changes, cover validation, serialization round trips,
deterministic output and invalid inputs. Publication/storage changes also need
failure recovery checks against temporary files or databases. Verify affected
desktop and engine consumers with workspace checks; shared contract changes can
compile locally yet break protocol or acceptance behavior. Follow the root guide
for scoped formatting, CodeScene and full validation expectations.
