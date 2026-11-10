# Incremental inference verification

Baseline: verified boundary checkpoint `3f8b652` and its frozen matching engine
and resources. The incremental pass caches pure boundary matching decisions;
compiler extraction and the earlier language-specific inference passes remain
fresh/full. Cached graph edges, source evidence, flow trees and node indexes are
never replayed. This is deliberately narrower than caching compiler analysis.

Dependency keys include the automatic implementation digest, format version,
current use, explicit application configuration, toolchain and complete potential
candidate/blocker membership. Current graph validation proves symbol support;
the use and route facts carry those exact identities. An added candidate changes
a negative dependency key. Unrelated node changes do not invalidate every use.
Fresh flow attachment is outside the cached decision and always rechecks the
actual current source site.

The cache is read-only until graph publication succeeds. The existing project
lock covers both graph publication and the subsequent current-row transaction.
Missing/incompatible/malformed cache data falls back to full inference; SQLite
ownership markers and regular-file checks protect unrelated files. Corrupt or
unowned cache files are preserved with volatile warnings. Cache errors never
change canonical graph diagnostics or successful publication into failure.

Verification:

- All seven canonical project graphs are byte-identical to `3f8b652`: Maven,
  Gradle, collision C#, TypeScript/Next, Spring, MediatR/ASP.NET and mixed.
  `incremental-corpus.json` records the exact SHA-256 values.
- Packaged repeated/reordered/relocated TypeScript and mixed runs match byte for
  byte. Cold runs recompute 13 decisions; warm runs reuse all 13 with zero misses.
  `incremental-determinism.json` records the checks. Each temporary project owns
  one graph path; comparisons do not write history snapshots.
- Fourteen real-SQLite tests cover positive/negative dependencies, unrelated reuse,
  blocker changes, support deletion, index rebinding, stale derivation cleanup,
  format/code invalidation, malformed winners/misses and key-bound checksums,
  current-row replacement, unowned/corrupt files, database/sidecar links, immutable
  prepublication reads, unowned WAL preservation and bounded locked-cache writes.
- Three real CLI tests pass against the latest engine binary. Cache/full bytes
  agree after source/route additions, edits and deletions, client rewiring, Next
  config blockers and observer deregistration. Completed fingerprints remain
  fresh after publication. Compile failure, cancellation and a real graph
  publication failure preserve both graph and previous cache bytes.
- The final workspace all-target suite and strict workspace Clippy pass. Modified
  source/test files score CodeScene 10.0 (reexports have no scored findings).
  The signed desktop app and relocatable standalone distribution were rebuilt
  with matching analyzer resources. Upstream source did not change in this stage;
  the previously documented 29 CallAtlas baseline failures remain the limitation.

Initial isolated performance runs are retained in
`incremental-initial-*-performance.json`. Their JVM timing/RSS variation prompted
an additional controlled round: the old engine sees its original pre-cache file
layout, while the candidate retains its warm derived cache. Cache/sidecar backups
exist only in process memory between paired runs; graph files remain at their
single current path. The corrected reports describe this protocol explicitly.
Sampled process-tree RSS is a lower bound and can count shared pages repeatedly;
short JVM processes can show substantial sampling/heap variation. These fixtures
do not establish a general memory or speed improvement.

Controlled paired median runtime (baseline → candidate): TypeScript
0.6448→0.6355s (-1.43%), Spring 2.6323→2.6136s (-0.71%), C#
3.3228→3.3183s (-0.14%), mixed 15.1471→15.0642s (-0.55%). No runtime
regression above 5% was observed. Sampled RSS medians respectively were
348,471,296→346,570,752; 303,808,512→213,942,272;
293,502,976→293,552,128; 373,571,584→352,468,992 bytes. Sampling completed
without errors. The very different Spring RSS observations between rounds
illustrate JVM/sampling variability, not a demonstrated cache memory improvement.
The cache avoids repeated matching decisions, not compiler execution; these small
workloads do not establish a meaningful speedup.
