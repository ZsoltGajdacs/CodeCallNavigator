# Architectural layers

Choose **Layer** in Graph Studio to group methods by architecture. Use **Spring**,
**React**, or **Repository configuration** to select rules. **None** disables layer
classification. The ordinary Project grouping remains the startup default.

Layers expand into projects, classes and methods. Configured bands run top to
bottom; arrows retain caller-to-callee direction, even when a call runs against
layer order. Cycles and uncertain calls remain visible under the existing filters.
Focused methods and shortest paths keep their normal traversal layouts. Tabs show
separate architectural areas; counts report graph elements outside the active tab.
Support layers occupy side lanes, and named flows occupy parallel columns.

Hover layer aggregates to list their source files. The inspector shows a selected
method's layer, matched rule and configuration origin. The preset selection lasts
with the saved workspace; the active tab participates in navigation history.

## Native configuration

Commit `.codecallnavigator/layers.yml` at the selected repository root. Configuration edits
are detected in the background and applied to the cached compiler facts without
compiling the repository again. Invalid edits keep the last valid classification
and display an error. Choose Repository configuration after adding a native file
if the workspace is currently set to None or a preset.

```yaml
version: 1
languages:
  csharp:
    layers:
      - name: Controller
        matchers:
          - annotation: [Controller, ApiController]
          - suffix: Controller
      - name: Service
        matchers:
          - namespace: '*.Services.*'
          - suffix: Service
      - name: Persistence
        matchers:
          - base_type: IRepository
      - name: Models
        support: true
        matchers:
          - kind: record
          - path: '**/Models/**'
  typescript:
    layers:
      - name: View
        matchers:
          - path: '**/pages/**'
      - name: Component
        matchers:
          - path: '**/components/**'
```

The supported language keys are `java`, `csharp` and `typescript`. Each section
accepts `tests`, `other`, and ordered `layers`. A layer has `name`, `matchers`, and
optional `tab`, `flow`, `support`, and `beside` layout settings. A layer name shared
by languages must have the same layout settings. Tests and Other are reserved
trailing layers. `beside` belongs to support lanes and names another lane in the
same tab; chained beside references are rejected.

Matchers accept these fields (string or string list unless noted):

| Field | Meaning |
| --- | --- |
| `path` | Glob against the repository-relative source path; supports `**` and brace alternatives. |
| `path_base` | `repository` (default), or `build` to match paths relative to the owning build/configuration. |
| `namespace` | Glob against a namespace/package; a trailing dot is also tried for review-style package patterns. |
| `suffix` | Suffix of a filename without its extension, or any declared type name. |
| `annotation` | Type annotation, C# attribute or TypeScript decorator. |
| `method_annotation` | Method annotation, attribute or decorator. |
| `interface` | Boolean: the file declares an interface. |
| `kind` | Compiler declaration kind, such as class, interface, record, struct, enum, annotation or type. |
| `base_type` | Implemented interface or base type reported by the compiler. |
| `annotated_interface` | Annotation/attribute on an implemented interface. |
| `content` | One regular expression matched against the analyzed source text. |

Fields within one matcher must all match. Alternatives in a list and separate
matchers use OR. The first matching layer wins. Analyzer-owned test files and
`tests` rules take precedence, followed by `other` rules, then configured layers.
Anything unmatched goes to Other. Semantic facts combine declarations in a file;
all methods of that file share a layer. Project/build-qualified ownership keeps
identically named types in different modules separate.

Qualified semantic names match exactly; simple names can match the final name
segment. C# attribute names accept either spelling with or without `Attribute`.
Missing facts never establish a match. TypeScript decorators apply only where the
compiler supports them; interfaces have no runtime decorators. Java source
annotations come from the existing javac source inventory, including SOURCE
retention; annotations unavailable in dependency bytecode cannot be recovered.

Content expressions use Rust regex syntax. Unsupported expressions, including
lookaround and backreferences, produce a configuration error rather than changing
the last valid view. Rules classify project source files represented by the method
graph; standalone configuration/resource files do not become new graph nodes.

## Import from Interactive Review

If `.codecallnavigator/layers.yml` is missing and the selected repository contains
`.interactive-review/layers.yml`, Repository configuration automatically converts
and saves it once. An existing native file always wins, including an invalid one;
fix it explicitly instead of expecting a fallback. Import does not overwrite a
concurrently created native file.

The importer translates Java/TS sections, matchers, layer order, tabs, flows and
support lanes. `package`, `methodAnnotation`, `implements` and
`implementsAnnotated` become `namespace`, `method_annotation`, `base_type` and
`annotated_interface`. `extends` resolves bundled `spring`/`react` presets or YAML
files relative to the importing file, with cycle detection. Child language
sections replace inherited language sections, matching the review app's behavior.
Repository selector lists are unnecessary because the native file already belongs
to the selected repository. Coverage and unknown fields produce actionable errors;
no native file is written on conversion failure. Imported path rules set `path_base: build`, preserving paths relative to the
owning build/configuration when a package is nested inside a repository. Native
rules default to repository-relative paths.

## Cache and module boundaries

New graph generations retain compiler-owned per-file declaration facts **and source
text** in SQLite so content rules can be reevaluated without mixing current files
with an older graph. This differs from older versions, which did not cache source
text. Source popups still read current files from disk. Refresh after source changes
to update facts; older caches remain usable in ordinary groupings and require a
refresh before Layer mode can classify them.

The shared `architecture` library separates native validation, review-format import,
pattern compilation, classification and filesystem loading. Analyzer adapters export
facts only. Layer assignments are disposable presentation data. Native workspace
jobs handle reloads; projection owns aggregation and band metadata; Graphviz owns
geometry. No layer rule changes graph symbol identities or analyzer call bindings.

## Headless classification

The analysis CLI can print the same file-layer classification before publishing
its graph. `--layer-root` defaults to the current directory; pass it explicitly
when analyzing a repository elsewhere. `--layers` accepts `none`, `spring`, `react`
or `repository`.

```sh
cargo run --bin codecallnavigator -- analyze --layers spring --layer-root /path/to/repo /path/to/repo/pom.xml
```
