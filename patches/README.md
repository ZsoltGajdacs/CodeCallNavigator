# CallAtlas assembly-qualified identity patch

`callatlas-assembly-identity.patch` applies only to upstream revision
`916c2419c0cdb91b5b86b84df4842dadb17bb6b4`.

SHA-256: `7b64792f14f02fafb4472402b0ab26ac53d9be6e496ef73964ad49791db1d08a`,
also pinned in `analyzers.lock.json`.

The pinned analyzer combines every solution compilation in one node dictionary,
while its original symbol IDs omit assembly ownership. Two independent projects
with `Program.Main` or the same qualified type/method can therefore silently merge.
Top-level statements additionally use one global ID and registration flag.

This patch preserves call binding and adds identity and entry-point metadata:

- Prefix method, field, property-initializer and delegate identities with the
  Roslyn-owned assembly name: `[AssemblyName]::original-symbol-id`.
- Preserve a single assembly prefix through local-function identity recursion.
- Register top-level entry points per compiler assembly rather than once per solution.
- Keep existing call binding unchanged. Export `declarationFactsVersion: "1"` and
  per-method source-declaration, implicitness, callable-kind and Object-signature
  facts from Roslyn. CodeGraph owns inclusion policy and prunes after validation.
- Preserve those facts when reading and writing CallAtlas snapshots.
- Export architecture file facts directly from the same Roslyn semantic models, including type/method attributes, namespaces, declaration kinds, base types and annotated interfaces. These facts do not change call binding or graph inclusion.
- Export `entryPoints.candidates` using the existing `CodeMapQuery.EntryPoints` query, including uncalled declarations.
- Export `entryPoints.applicationStarts` from `Compilation.GetEntryPoint`, mapping synthesized top-level starts to the existing top-level graph identity. These verified starts are distinct from ordinary entry candidates.

The application must reject independent projects sharing an evaluated
`AssemblyName` within one selected solution, because this identity format cannot
safely distinguish them. Separate selected builds remain qualified by build path.
Assembly names are opaque identity data; do not resolve calls by parsing names.

Apply after checkout and before the analyzer build:

```sh
git -C vendor/callatlas apply ../../patches/callatlas-assembly-identity.patch
dotnet build vendor/callatlas/src/CallAtlas.Cli/CallAtlas.Cli.csproj -c Release
```

Bootstrap verifies the exact revision and patch hash. It accepts only a clean
checkout, the exact current patch, the previous declaration-policy patch in
`callatlas-assembly-identity-policy.patch`, the earlier entry-metadata patch in
`callatlas-assembly-identity-entries.patch`, or the exact identity-only
patch preserved in `callatlas-assembly-identity-legacy.patch` with its pinned hash.
The legacy patch is reversed and replaced automatically; failure restores it.
Unrelated tracked or untracked source edits are refused and preserved. Ignored
build outputs do not affect this check.

Regression fixture: `fixtures/dotnet-collisions/Collisions.sln` contains four
independent applications with duplicate entry/type/member names and two top-level
entry points. The acceptance checker verifies every assembly scope, field/accessor,
initializer, delegate, operator and local-function identity, both top-level roots,
and absence of false relationships between the independent applications.

Verified against the untouched pinned analyzer in an isolated temporary checkout:
it reports 12 nodes and 10 relationships with zero diagnostics for this fixture.
The patched analyzer reports 26 distinct nodes and 20 relationships, preserving
all four assemblies. PacificFate now reports 20,494 nodes and 75,296 relationships
with zero diagnostics/unresolved calls; the original unqualified IDs had collapsed
722 declarations and 3,750 relationships.

The negative fixture at
`fixtures/dotnet-collisions/rejected/RejectedSameAssembly.sln` contains two
independent projects with the same explicit assembly name. The application's
MSBuild boundary must reject it before calling the analyzer.

Upstream limitation preserved by this patch: overloaded-operator declarations are
registered, but the analyzer does not report the binary operator invocation used
by this fixture. The identity check proves that the two operator declarations stay
separate; it does not claim that missing invocation coverage has been repaired.

# Rune platform warnings patch

`rune-platform-warnings.patch` applies to Rune revision
`7ed827a2b969ec737323c3e80d2bfc1b8032b609` (SHA-256
`ae0d3cc48c2c49e47f44e9a7e036bf41a0d901ab767488eee87d95a92a56074d`).
It scopes platform-specific dead-code allowances and the Windows-only focus method
to avoid warnings on macOS while preserving the shared state machines. Bootstrap
applies it after checkout; no other Rune patches precede it. Bootstrap refuses
unrelated Rune source changes rather than overwriting them.

# Rune oriented segment quads

`rune-oriented-segments.patch` replaces the warnings-only patch on Rune revision
`7ed827a2b969ec737323c3e80d2bfc1b8032b609`. Its SHA-256 is pinned in
`analyzers.lock.json`. It contains the existing platform-warning fixes followed
by tightly oriented vertex quads for line segments and a GPU pixel regression.
Apply the combined patch once to the pristine pinned revision. Do not stack the
warnings-only patch with it. Bootstrap recognizes the exact previous
warnings-only diff, reverses it, then applies this replacement; unrelated source
changes are rejected and preserved.

The segment endpoints, instance ABI, CPU axis-aligned bounds, antialias coverage,
round/butt caps, and fragment distance function are unchanged. Only rasterized
vertex bounds become a thin strip; the padding includes the same two device
pixels of conservative fringe as the original renderer. Zero-length round
segments retain a finite quad and their original circular coverage.

The pinned precompiled uber/Windfoil libraries intentionally retain their original
source fingerprints. Their hash guards reject the changed shader and compile
the patched WGSL at runtime; the native built-in table also keys on shader source.
Do not update fingerprint constants to match stale bytecode. The focused pixel
test verifies rejection of the stale uber library and compares the new renderer
with the original AABB vertex path at 1x and 2x, including clipping, fades,
transparency, isolated-layer projection, caps, reversed slopes and degenerate
segments. The upstream precompiled-library freshness tests require regeneration
of those libraries and are not claimed to pass for this runtime-compiled patch.

Patch migration tests: `python3 -m unittest discover -s tools -p test_dependency_patches.py`.

# Complete patch verification

Bootstrap and packaging compare the entire checkout diff with the verified patch
bytes. A patch may add source files: `git apply` leaves those files untracked,
so verification uses a disposable Git index to include them in the comparison.
The upstream checkout's real index is preserved. Extra untracked files or other
unrecorded changes make the diff differ and are rejected; no stash is used.

Added-file installation, repeat verification, previous-patch migration and
unrelated-file rejection tests:
`python3 -m unittest discover -s tools -p 'test_dependency*.py'`.
