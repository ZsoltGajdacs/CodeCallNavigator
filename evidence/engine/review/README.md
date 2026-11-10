# Standalone engine review against master

The reference is local `master` at
`fb54f5f789d94c6cbc5fe86bf20010edea815b33`. Review fixes stay on
`codex/standalone-graph-engine`; no merge or push is authorized.

## Confirmed defects and fixes

- Canonical Java/C# identities nested on hydrate/export. Reuse only a verified
  canonical identity with the same complete owner; preserve native binding.
- Malformed portable paths and unsupported nested flow/architecture inventory
  versions were accepted. Validate them before hydration or comparison.
- Checkout names, nested TypeScript configurations, external compiler source
  aliases and escaped Java context paths changed exported metadata or identities.
  Normalize producer-owned path fields and folder-derived display names while
  preserving opaque compiler names, source literals and semantic applications.
- Java synthetic lambda ordinals changed after unrelated insertions. Export
  compiler-source fingerprints only for uniquely proven bytecode attribution.
  Ambiguous cases retain native IDs and an explicit warning. The user deferred
  exact ambiguous correlation to TASK-32.
- Unnamed TypeScript default classes, computed names and regex-containing
  anonymous bodies had unstable identities. Use compiler declaration ownership
  and parsed token boundaries, excluding trivia.
- Mutation or structural escape of the global Set constructor could manufacture
  observer invocation edges. Invalidate intrinsic proof on those operations.
- Custom ASP.NET HTTP metadata, endpoint providers and applied MVC conventions
  could invalidate inferred routes. Compiler-qualified unknown rewrites now
  block affected application owners. Actual .NET fixtures include negative
  controls and awaitable handler results; no runtime hooks are executed.
- A crashed engine could leave inherited stdout open or continuously emitting,
  hanging the desktop; prompt EOF could leak descendants. Observe leader exit
  independently, bound stream draining and reap the owned process group.
- Git index flags and external diff commands could conceal unrecorded upstream
  edits. Verify through a private index with flags cleared and raw diff enabled;
  the real upstream indexes remain byte-identical. The complete CallAtlas change
  is recorded in the pinned replacement patch and prior-checkpoint migration.

## Verification

- Final workspace: 379 passed; 77 opt-in tests excluded from the default run.
- Strict workspace/all-target Clippy passed.
- Python: 58 project tests and 19 tooling tests passed.
- TypeScript adapter: 104 tests passed.
- Matching Java helper, standalone engine and desktop app distributions built.
- Pinned dependency-source verification passed for all three upstream checkouts.
- Modified-source CodeScene reviews scored 10; pre-commit safeguard passed.

- Real-language opt-in checks: 51 passed, covering Maven/Gradle ownership,
  duplicates, Lombok, Java flow/lambda/Spring/HTTP, C# policy/entries/ASP.NET/
  MediatR and the TypeScript production pipeline and boundaries.
- Final packaged/incremental/linked-source integration checks: 8 passed.
- All seven fixture graph files were refreshed with the final packaged engine.
  C# and MediatR files remain byte-identical. Maven/Gradle/Spring add explicit
  portable compiler-root provenance. TypeScript/mixed graphs update logical
  display paths and compiler-qualified React signatures plus their consistently
  translated symbol references. Node and edge counts remain unchanged.

- Desktop process protocol/roundtrip/cancellation/crash cleanup: all 8 passed.
- All seven workloads passed repeat, reversed-target and relocated-checkout
  byte equality; see [determinism results](determinism.json).

The [full native comparison](native-preservation.json) against the frozen
pre-review `b839ba3` distribution covers all seven workloads. Native identities,
complete edges, entries, flows, architecture, boundary facts, diagnostics and
content/membership freshness data are unchanged. Six TypeScript node signatures
now qualify React return types; corresponding portable keys change consistently.
Remaining differences are analyzer provenance and explicit Java root/folder
metadata. No unexplained binding or topology changes remain.

The original foundation equivalence against `master` is documented in the parent
engine evidence. Subsequent inference stages deliberately add reviewed possible
relationships; this review does not equate their output with pre-inference data.

Final matched performance results are recorded separately after completion.
One negative MediatR test initially failed during NuGet restore; its isolated
retry passed, including the expected compiler diagnostic. No analyzer failure
was observed on retry.

The complete CallAtlas suite has 262 passing tests and the same 29 failing test
names as the pre-review baseline. See [the exact comparison](callatlas-tests.json).
Those existing upstream failures are not claimed to pass. Private recurring
billing checks require unavailable private inputs. GPU performance experiments
are outside this engine review.
