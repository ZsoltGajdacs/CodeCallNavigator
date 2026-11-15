# TypeScript execution facts checkpoint

Unit2C passed independent review and 140 distinct tests: 120 JavaScript adapter,
2 real preparation gates in release mode, one new real CLI facts/pruning/source
case, one existing CLI query and 16 shared compatibility cases. Definitive groups
had no skips. Clippy was clean. No full/native GUI/package command was run.
See [independent.json](independent.json).

The version-7 helper proves bounded ordinary calls from compiler bindings and
explicit evaluation order, retaining conservative unknown facts for unsupported
forms and reassigned bindings. Generator defaults are an immediate prefix before
the deferred body, so their combined IR remains unknown. Historical canonical
inventory stays version 1; fresh preparation rejects old/missing/future helpers.

The final immutable-v6/relocated-v7 comparison preserves 16 native nodes, one edge
and 11 function IR bodies apart from execution/invocation and adapter version.
All 72 copied files, including 70 JavaScript files, match frozen source. Earlier
checks covered 61 production JavaScript files only; a stale test-copy checkpoint
and its mismatched file remain preserved. Final full-copy equality uses
[projection-relocation-frozen.json](projection-relocation-frozen.json).

Both final CodeScene reports passed: 14 files scored 10, safeguards checked
140 eligible files of 156 total. Original test-walker findings remain in the
archive; the focused 22-case rerun and independent full suite verify its refactor.
The independent report also records initial ignored/default and debug-targeted
preparation attempts; only definitive release results count for those gates.

All current and prior manifests, staged baseline and immutable helper hashes
stayed unchanged. The archive retains original reports/logs/maps/probes and the
stale test file byte for byte; its index records sizes and SHA-256 hashes.
