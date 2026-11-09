# CallAtlas assembly-qualified identity patch

`callatlas-assembly-identity.patch` applies only to upstream revision
`916c2419c0cdb91b5b86b84df4842dadb17bb6b4`.

SHA-256: `a3981a0c07ef932a498f58113994c7858bd53242e0f0860355af9df56704e810`.

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
