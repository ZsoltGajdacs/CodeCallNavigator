# Portable graph contract hardening — 2026-11-11

TASK-39 implements the approved CodeGraph-only contract plan. See
[consumer contract](../../docs/graph-contract.md) and
[Draft 2020-12 schema](../../docs/graph.schema.json).

## Implementation and review log

1. Recorded Draft DRAFT-1, promoted to TASK-39 and activated with the approved plan.
2. Characterized legacy relocation, ordering, references, evidence and publication;
   introduced an explicit portable graph model, typed transformations, domain
   validation with a shared endpoint index, and structure-scoped canonical ordering.
3. Added compact serialization and opaque encoded artifact buffers shared by digest
   computation and atomic publication. Legacy loading does not rewrite files.
4. Added portable observed-input provenance, content verification and CLI reporting;
   removed machine-specific compiler context/version-report paths. Added independent
   schema/consumer tools and documentation.
5. Independent review found and resolved reserved envelope extension collisions,
   raw-byte publication bypass, manifest ordering idempotence, nullable metadata,
   and unavailable-root classification. CodeScene findings drove domain helper
   extraction; all reviewed source files reached 10.0 and the safeguard passed.

## Passing verification

- `cargo check --locked -p codegraph --all-targets`.
- `cargo test --locked -p codegraph --all-targets` (normal ignored-test policy).
  An initial concurrent run had a process-test PID parsing race; the complete rerun passed.
- `cargo test --locked -p codegraph-core --all-targets`: 55 passed.
- `cargo test --locked -p codegraph-engine --all-targets`.
- `cargo test --locked -p codegraph-engine --test verify`: 2 passed.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`.
- `python3.13 tools/package.py --engine-only`.
- Both real Java and C# `linked_sources` ignored acceptance tests passed when run
  outside the sandbox to permit Maven/NuGet cache locks and compiler host pipes.
- Seven independent Python schema/consumer tests passed using test-only
  `jsonschema>=4.18,<5` in a temporary virtual environment.
- CodeScene file reviews: all modified/new source files 10.0. Final pre-commit
  safeguard passed with 20 eligible files checked. No Sonar configuration exists.

## Real TypeScript consumer acceptance

Analyzed a temporary copy of the existing TypeScript corpus, with explicit logical
roots for compiler dependencies and observed ancestor package-manager inputs.
The retained mixed-language fixture (179 nodes, 120 edges) and fresh production
artifact (75 nodes, 54 edges, 29 evidence records, 289 input records) passed the
published schema and independent JSON consumer.

Fresh output was 251,159 bytes, compact JSON plus one newline. Repeated production
analysis produced identical bytes. Publication, saved-file and verification digests
agreed (`21bddf1b2b6e00bc74b50d7d3b438e05262efe6eb12d822cb6d78cd2679b1517`).
Full project-file mtime/size/SHA256 snapshots showed no verification writes.
A same-size source edit with its original timestamp restored produced exit 1 and
`stale`; restoring source bytes returned exit 0 and `match`, without changing the
artifact. The packaged engine also verified the artifact successfully.

Without a logical root covering observed ancestor inputs, verification correctly
reported `unverifiable`. This is deliberate: observed-input provenance does not
certify inputs that cannot be mapped portably.

## Full-suite limitations

`cargo test --workspace --all-targets -- --include-ignored` was run and failed:

- Three canvas performance tests explicitly require `--release` rather than the
  documented debug invocation.
- The detached-process cancellation acceptance test reported a surviving child
  under the sandbox; it is explicitly marked as requiring external process access.

A separate engine run with ignored tests reached the unavailable private recurring
billing fixture (`Set CODEGRAPH_COMPILED_METADATA`). Skipping that fixture exposed
sandbox Maven lock/compiler host pipe failures in linked-source tests; rerunning
those two tests with escalation passed. The entire ignored workspace suite is not
claimed passing. No unrelated performance/process behavior was changed.

The user explicitly accepted the documented full-suite limitations and requested
TASK-39 completion and a commit. The acceptance applies to the release-only
performance tests, sandbox process-cleanup failure, and unavailable private
compiler fixture described above; those checks are not claimed passing.
