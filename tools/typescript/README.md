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

Framework relationships are static possibilities. Generic callback arguments are
references, not calls. Mutation through refs/maps, arbitrary higher-order return
values, dynamic keys, spread-prop delegation and framework routing configuration
are not reconstructed. Counts for unresolved bindings remain in graph diagnostics.

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
