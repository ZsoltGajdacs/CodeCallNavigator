# Existing Java bytecode analyzer adapter

This driver configures `java-callgraph2` 4.1.0 and respects its boolean result.
The bytecode driver adds no call-resolution, DI or framework logic. A separate
`SourceInventory` helper uses the JDK compiler APIs to inventory source declarations
in their owning compile context, before compiler-created declarations are inserted.
BCEL is pinned
to 6.13.0 to support Java 25 class files. Both raw and engine-reported actual
targets are retained in the engine output.
The engine's default equals/hashCode omission is disabled using its existing
`ECE_PARSE_IGNORE_METHOD` configuration so completeness validation includes project
overrides. CodeGraph subsequently excludes Object-method signatures, constructors,
initializers, inherited placeholders and members without source declarations.
The existing `ECE_PARSE_IGNORE_METHOD_CALL_TYPE_VALUE_CALLER` switch disables
type/value inference only for exact `equals(java.lang.Object)` and `hashCode()`
signatures, including authored overrides. Their declarations and bytecode call
targets remain in analyzer output. This avoids symbolic frame snapshot exhaustion
in branch-heavy generated equality methods. Every other method, including
`equals(String)` and `hashCode(int)` overloads, retains full inference. Snapshot
limits and incomplete output still fail analysis; there is no automatic fallback.
Authored lambda implementations survive synthetic-bytecode filtering, including
builds without debug line tables. Generated source files remain eligible for the
existing generated-source visibility toggle.

Build with an existing Gradle 9.4 installation and JDK 25:

```sh
gradle --project-dir tools/java-bytecode installDist --write-locks
```

Run after the target project's own build succeeds:

```sh
tools/java-bytecode/build/install/codegraph-bytecode/bin/codegraph-bytecode \
  /absolute/path/to/analysis-output 25 /absolute/path/to/project/build/classes/java/main
```

Additional class directories or jars may be passed as additional arguments.
Only pass selected project outputs for the project-only acceptance run. Include
test outputs explicitly when testing those, and track the input-to-project
mapping outside the analyzer.

The analyzer emits its documented tab-separated text files in a child directory
of the output root. `analysis-status.properties` records that directory, engine
success, adapter completeness and any frame snapshot limit. Exit 0 means the engine returned success
with method declarations and without reporting that limit. Exit 1 means engine failure or empty output, 2 means invalid inputs
or an exception, and 3 means the engine reported limited analysis.

Retain all rows from `method_info.txt` to preserve disconnected declarations;
use `method_line_number.txt` for available line ranges and `method_call.txt` for
reported relationships. Preserve call type and enabled flag. Bytecode does not
provide precise source columns; abstract or generated declarations may lack
line information. Class ownership follows analyzer input provenance, not package
prefixes. No runtime target is inferred by the adapter.

Two adapter metadata files are written directly under the output root using
BCEL's existing `SourceFile`, method flags, annotations and line-table attributes:

- `class_metadata.tsv`: input index (one-based), input path, class entry, class
  name, SourceFile attribute, class flags, annotation descriptors.
- `method_metadata.tsv`: input index, input path, class entry, class name,
  SourceFile attribute, engine-style method signature, return type, method flags,
  minimum line, maximum line, annotation descriptors.

Both have a header row. Backslash, tab, newline and carriage return in fields
are escaped as `\\`, `\t`, `\n`, `\r`. No line table is represented by zero line
numbers. Missing SourceFile attributes retain BCEL's unknown value; the adapter
does not invent file locations. Method identity includes return type when joining
to engine output, preserving bridge methods that share a name and argument list.
Annotation descriptors include class-retained generator markers where present.

Upstream output documentation:

- https://github.com/Adrninistrator/java-callgraph2/blob/main/docs/file_format.md
- https://github.com/Adrninistrator/java-callgraph2/blob/main/docs/call_type.md
- https://commons.apache.org/proper/commons-bcel/changes.html

Source discovery selects files represented by compiled class metadata, so files
excluded by the build do not enter attribution. Without `SourceFile` debug
attributes (or with nonstandard source layouts), a separate parse-only pass
matches top-level declaration names to compiled types; exact dollar-containing
class names take precedence over nested-class prefixes. Package metadata does
not declare methods. Unmatched stale/processor outputs have no source declaration
and are excluded. Ambiguous ownership, malformed selected sources, and unresolved
method signature types fail inventory explicitly. When source attribution cannot resolve a
qualified signature type generated by an annotation processor (such as a Lombok
builder), a separate compiler context resolves its binary name from the owning
classpath with an empty source path. It never runs processors or adds generated
members to the parsed declaration snapshot. Unqualified unresolved names fail
closed rather than binding to an unrelated default-package class. Inventory cache
files are published atomically only after successful, uncancelled generation;
failed or interrupted writes are removed before retry. Body-only attribution errors
from disabled annotation processing do not change successfully resolved source
method identities; the actual build and bytecode completeness checks remain
mandatory.

The source inventory's `kind_v5` format adds Spring event candidate pairs, a
supported source-listener catalog and framework diagnostics. `SourceFramework` first collects source-owned listeners,
then checks compiler-bound `ApplicationEventPublisher.publishEvent` sites using
live javac `Types` assignability, including `PayloadApplicationEvent<T>` wrapping.
The Rust adapter maps exact source declaration keys through the existing bytecode
identities and requires a matching UTF-8 flow call site before adding a possible
`spring-event` edge. Recognized listeners remain framework entry points when
there is no publisher; registration alone creates no invocation edge. The bytecode
driver's native call relationships are unchanged.

Direct Spring `@EventListener`, `@TransactionalEventListener`, `@Async` and typed
`ApplicationListener<T>` are supported. Evidence retains conditions, transaction
phase/fallback/default-execution values and async boundaries. These are static
possibilities; DI registration and runtime execution are not proven. Only parsed
method publishers and listeners in the same source inventory are paired. Erased,
raw or directly parameterized payload runtime classes remain unresolved because
javac unchecked assignment and source generic arguments do not prove Spring runtime
resolvable types. Concrete nongeneric subclasses retain compiler generic-supertype
metadata. Unresolved payloads, lambda publishers, custom composed annotations and
cross-context wiring remain unresolved, with diagnostics. No joins use simple
payload names, display labels or unrelated selected-build facts.

`kind_v6` retains the event facts and adds compiler-qualified Spring Web HTTP route
facts. Direct `@RequestMapping` and GET/POST/PUT/PATCH/DELETE shortcuts combine
class prefixes, paths and verbs. Each attributed controller method remains an
entry point. Header/query/media constraints, unknown composed mappings, inherited
mappings and unsupported path patterns remain inventory blockers; unbound methods
also survive as blockers. Fake annotations with the same simple names are ignored.
Property-dependent paths remain owner-wide blockers. Route facts do not prove bean
registration, servlet deployment context paths or custom global MVC configuration.
The current engine requires matching v8 resources; legacy inventory formats remain
readable for historical fixtures but cannot silently satisfy the current runtime.

The current `kind_v8` source inventory retains portable lambda export keys and
adds verified compiler/source correlation. Within named type owners, the source
key still contains the logical enclosing method/initializer, syntactic role,
normalized AST fingerprint and deterministic sibling occurrence. Singleton key
bytes are unchanged from v7, and Rust adds the owning compile-context prefix.

Before lowering mutates the trees, inventory snapshots source positions, keys and
flow IR. An optional compiler pass then generates classes in temporary storage,
with annotation processing disabled. Its lowered LambdaMetafactory sites retain
source offsets and exact implementation handles. A BCEL symbolic class comparison
verifies those generated classes against the actual build output, including
method descriptors, executable instructions, exception regions and bootstrap
arguments. Debug attributes and constant-pool ordering do not establish identity.
Bounded debug modes cover both separate and compiler-shared implementations;
conflicting successful candidates are rejected. No association is inferred from
synthetic names or ordinal positions alone. Project sources and compiled outputs
are never rewritten by correlation.

Verified same-line, overlapping/nested and missing-line singleton lambdas receive
source-derived export identities and exact source-flow bindings. When javac merges
several source expressions into one implementation, the existing single graph
node receives a separately tagged group identity containing the sorted complete
collection of source keys. All members must be accounted for and have portable
keys. Adding, removing or editing a member can change that group identity. A
shared implementation does not acquire an arbitrary member's source flow or span;
a separate diagnostic explains that source-flow ambiguity.

Indistinguishable separate siblings keep distinct deterministic occurrence IDs.
There is no individual continuity guarantee when indistinguishable siblings are
swapped or another identical sibling is inserted before them. Formatting and
unrelated insertions do not affect otherwise distinguishable source identities.

Anonymous/local type owners retain compiler-native export identities. Unsupported
compiler configurations, processor-dependent generation, incomplete evidence and
bytecode mismatch also retain diagnosed fallback. A rejected proof cannot be
overridden by line matching. Compiler renumbering can still affect fallback IDs.
Native bytecode nodes, IDs and call relationships remain unchanged by enrichment.

Inventory correlation records are scoped to the owning compile context. A
`lambda-correlation` row records a class owner's verified, unavailable or rejected
outcome and reason. A `lambda-binding` row records the exact implementation key
(including return type) and its complete source-symbol membership. Rust validates
the records before assigning identities or binding flows. Current runtime helpers
require v8; historical formats remain readable for existing fixtures. Rebuild the
matching analyzer resources when upgrading. Compiler-internal module exports are
configured for adapter compilation, direct helper launches and installed scripts.
