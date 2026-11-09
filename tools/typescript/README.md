# TypeScript compiler adapter

The Rust coordinator invokes `node analyze.js prepare CONFIG OUTPUT` to evaluate
configuration and input ownership, then `node analyze.js analyze CONFIG OUTPUT`
to obtain the `codegraph.typescript.v1` graph. The project must already have
TypeScript 6.0.x installed and Node must be version 24 or newer. No packages,
project scripts, application code, compiler plugins or framework configs run.

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
`graph.js` owns identities, edge aggregation and serialization.

Framework relationships are static possibilities. Generic callback arguments are
references, not calls. Mutation through refs/maps, arbitrary higher-order return
values, dynamic keys, spread-prop delegation and framework routing configuration
are not reconstructed. Counts for unresolved bindings remain in graph diagnostics.

Run the real-compiler fixture suite with:

```sh
CODEGRAPH_TEST_TYPESCRIPT_ROOT=/path/to/node_modules/typescript node --test tools/typescript/adapter.test.js
```

The fixture suite uses the installed TypeScript, React types and Next.js package;
`CODEGRAPH_TS_NODE_MODULES` can alternatively select the modules directory;
when neither variable is present it uses the sibling PacificFateMapEditor installation.
Tests create temporary projects and link those dependencies without modifying them.
