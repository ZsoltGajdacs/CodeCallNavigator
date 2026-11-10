# TypeScript compiler adapter

The Rust coordinator starts `node analyze.js worker CONFIG METADATA_OUTPUT` and
requests extraction over its stdin after validating ownership and fingerprints.
For one selected target the worker retains compiler programs between phases.
For multiple or mixed targets, preparation workers are released immediately;
each target reopens one worker for extraction and checks its complete metadata
against the original snapshot. This bounds live compiler workers to one and
keeps compiler heaps out of unrelated Java/C# analysis. No cache survives the job.
The standalone `prepare` and `analyze` modes remain available for adapter checks.

The worker verifies the file contents, missing resolution probes and directory
queries observed while constructing its programs before and after extraction.
The project must already have TypeScript 6.0.x installed and Node must be version
24 or newer. No packages, project scripts, application code, compiler plugins or
framework configs run.

`config.js` evaluates the configuration-reference closure and ownership.
`compiler.js` loads the project compiler and constructs its programs. A closed,
non-emitting compiler watch host enables the public source-reference redirect
API without creating watchers or requiring emitted declaration files.
`inputs.js` records consumed inputs, missing lookup candidates and directory
membership; `dependencies.js` owns package manifests and version provenance. `bindings.js` resolves project-owned callable declarations; it never
uses the origin of a synthesized call signature as a runtime target.
`accessors.js` classifies property reads, writes and updates.
`extract.js` walks callable scopes, recording compiler-bound calls, accessors and
callable argument references. Conditional and propagated prop targets are possible.
`react.js` and `props.js` add versioned static framework rules.
`next.js` recognizes standard App Router exports and server actions.
`source-facts.js` discovers callable kinds and matches instance member parameter
signatures against the compiler's standard Object type. `method-policy.js` owns
which facts exclude a symbol; `graph.js` serializes the excluded identity list
alongside the complete graph. The Rust adapter validates that graph, invokes the
shared pruning transformation, then recalculates TypeScript entry candidates.
Constructors and field/static initializer scopes remain distinct during extraction,
so excluding their nodes never assigns their calls to a module or enclosing method.
Authored nested callbacks retain their own callable scopes.
`graph.js` owns identities, edge aggregation and serialization.

Framework relationships are static possibilities. Passing or returning a callback
does not invoke it. Mutation through refs/maps, arbitrary higher-order wrappers,
dynamic keys, spread-prop delegation and framework routing configuration are not
reconstructed. Counts for unresolved bindings remain in graph diagnostics.

Run the real-compiler fixture suite with:

```sh
CODEGRAPH_TEST_TYPESCRIPT_ROOT=/path/to/node_modules/typescript node --test tools/typescript/*.test.js
```

The fixture suite uses the installed TypeScript, React types and Next.js package;
`CODEGRAPH_TS_NODE_MODULES` can alternatively select the modules directory;
when neither variable is present it uses the sibling PacificFateMapEditor installation.
Tests create temporary projects and link those dependencies without modifying them.

`observer.js` adds possible invocation targets for append-only callback `Set`
channels after ordinary bindings and React props are collected. The collection,
registrar and callback are joined by compiler declarations, never matching names.
`Bindings.callTargets` supplies the same symbol, certainty and provenance to graph
extraction and function-flow export. Registration arguments remain references;
only an actual callback call in a direct `for...of` body creates an invocation.
Evidence records that call and every supporting registration site.

Supported storage is a module-local `const` empty built-in `Set`, a private static
readonly field on a module/namespace class, or a private readonly field of a closed
singleton class. Singleton classes are nonexported, top-level module classes
without inheritance, with exactly one `new` initializing a top-level `const`.
Instance registration and dispatch receivers must originate at that constant,
an immutable alias, or the same class method's `this`. Mutable aliases do not
establish identity. Static storage access requires its lexical class or an
immutable exact-class alias; dynamic static `this` is deferred.
Constructor aliases, factories, constructor metadata access, explicit untyped
instance escapes and multiple instances are deferred.

Callback values must be declarations, inline functions or immutable aliases.
Method values require static class ownership or a const object-literal receiver;
mutable, cast and call-valued receivers are deferred. Decorated storage owners,
fields, registrars, dispatchers and callback methods are deferred because source
symbols do not prove that a decorator preserved their runtime implementations.
Mutable parameters, collection replacement, clearing/deletion, collection escape
and computed collection access are deferred. Known project writes to the built-in
constructor, add or iterator operations disable affected inference. Contextual
type-changing object aliases, typed parameters, containers and returns are deferred
because they can hide receiver method replacement behind different declarations.
Extracted or rebound field-backed dispatcher methods are deferred as well.

Collection uses are initially limited to direct `.add` and direct `for...of`;
`Set.forEach`, extracted registrar methods and nested deferred callback calls are
outside this rule. Recognized unsupported channels retain diagnostics. These are
closed-source possibilities; reflection and external mutation are not reconstructed.

`callable.js` adds possible calls for immutable `??`/`||` aliases, identifier
parameters invoked directly, and simple project-owned function-return expressions.
The strict resolver is separate from legacy compiler/React prop binding sets;
those sets cannot prove new incoming callable values are immutable. Direct project
call sites contribute finite parameter target sets until they stop growing.
Per-call return environments keep `identity(a)` separate from `identity(b)`.
Return summaries support concise bodies or const aliases followed by one return;
conditional function references are possible alternatives. Logical evaluation
retains short-circuit semantics and distinguishes nullish from other falsy values.

New callable facts reject mutable aliases and parameters, property/registry values,
dynamic keys, spread/rest forwarding, destructured parameters and decorated owners.
Returned closures capturing factory parameters or local aliases are deferred;
capture-free returned callbacks are supported. Return summaries exclude
async/generator factories, optional factory calls and arbitrary bodies.
Inference diagnostics describe
unsupported value paths; explicit possibly-nullish default arguments are deferred
because the abstract value does not distinguish null from undefined. Omitted
defaults and proven non-nullish arguments remain supported. Existing compiler-bound
calls retain their old behavior.
Graph and flow use the same invocation descriptors, with possible certainty,
`codegraph.callable.v1` provenance and source-backed rule evidence. Callable flow
does not replace observer targets or convert argument references into invocations.

`boundary.js` exports versioned route/use facts for the shared boundary resolver.
App Router APIs must bind through `next/navigation`; Link must bind through
`next/link`. Pages Router, local reexports and unrelated same-name declarations
are outside this subset. Only immutable nonescaped router receivers are accepted.
Link `as` and spreads defer inference. Builtin fetch accepts literal root-relative
URLs and supported literal options/methods; compiler-owned global-object writes
and structural escapes invalidate that proof. Class initialization regions are
not attributed to enclosing functions.

Next route groups retain competing destinations, single dynamic segments become
shared `{name}` templates, and catchalls/parallel/intercepted routes remain blockers.
GET contributes implicit HEAD only without an explicit HEAD export. Configuration
or middleware/proxy presence conservatively blocks routing rather than executing
project configuration. Metadata tracks present and absent configuration files.
The Rust entry point requires boundary capability v1 and a present inventory;
stale adapter distributions fail explicitly.
