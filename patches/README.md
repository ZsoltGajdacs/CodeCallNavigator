# CallAtlas assembly-qualified identity patch

`callatlas-assembly-identity.patch` applies only to upstream revision
`916c2419c0cdb91b5b86b84df4842dadb17bb6b4`.

SHA-256: `ec723795ee841b98c2efbf49a7f010cd3f22652889c0e5c34866461b0137c574`.

The pinned analyzer combines every solution compilation in one node dictionary,
while its original symbol IDs omit assembly ownership. Two independent projects
with `Program.Main` or the same qualified type/method can therefore silently merge.
Top-level statements additionally use one global ID and registration flag.

This patch changes **identity serialization only**:

- Prefix method, field, property-initializer and delegate identities with the
  Roslyn-owned assembly name: `[AssemblyName]::original-symbol-id`.
- Preserve a single assembly prefix through local-function identity recursion.
- Register top-level entry points per compiler assembly rather than once per solution.
- Keep the existing JSON schema and every call binding/classification unchanged.

The application must reject independent projects sharing an evaluated
`AssemblyName` within one selected solution, because this identity format cannot
safely distinguish them. Separate selected builds remain qualified by build path.
Assembly names are opaque identity data; do not resolve calls by parsing names.

Apply after checkout and before the analyzer build:

```sh
git -C vendor/callatlas apply ../../patches/callatlas-assembly-identity.patch
dotnet build vendor/callatlas/src/CallAtlas.Cli/CallAtlas.Cli.csproj -c Release
```

Bootstrap should verify the exact revision and patch hash, and accept an already
applied patch only when a reverse `git apply --check` succeeds. It must not reset
unrelated local dependency edits.

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
