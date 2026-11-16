---
id: doc-24
title: Function flow implementation boundaries
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:07:00Z'
---
# Function flow implementation boundaries

Migrated 2026-11-16 from `docs/function-flowcharts.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

## Implementation boundaries

The Roslyn adapter, javac inventory and TypeScript compiler worker export detached,
versioned statement trees during refresh. They reuse compiler contexts and call
binding. The shared flow library owns completion semantics, binding cleanup and
Graphviz geometry. Source spans are half-open UTF-8 byte ranges over the exact
cached text, converted from compiler UTF-16 positions at export.

Flow metadata is separate from method nodes and inter-method relationships. Graph
composition and symbol pruning preserve those original relationships. The UI owns
background jobs and separate navigation, rejects obsolete generation/method results,
and shares source-row rendering and connector drawing with existing views. Layouts
share Graphviz process execution while keeping their layout policies separate.

See [verification evidence](<../history/doc-7 - Function-flow-semantics-and-delivery-history.md#source-evidence-flowcharts-readme-md>) for real compiler
traces, graph-invariance comparisons, cache/refresh measurements and UI captures.
