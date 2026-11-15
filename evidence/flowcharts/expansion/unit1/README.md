# Invocation-fact contract checkpoint

Tests first observed four failures: new facts were discarded, invalid enums were
ignored, and inconsistent isolation/direct-dispatch claims were accepted. Typed
facts now default to uncertainty and omit unknown/default values during serialization.
The shared owner-aware walk validates positive claims at publication, source, checked
chart and direct builder boundaries. Dispatch survives target filtering without promotion.

Independent review approved all 17 frozen hashes and the unchanged staged baseline.
Independent tests passed 209 distinct cases (151 core, 18 engine comparison, 40 desktop
consumers), with no failures or ignored cases. The 16 new fact cases are counted only
within the core group. Exact legacy IR/chart/canonical-artifact bytes remain unchanged.
Workspace compilation and core Clippy pass; existing Rune warnings remain.

Both raw CodeScene reports pass: 13 scored files at 10, one declaration-only barrel,
and passing safeguards. `verification.json` discloses that the initial sandbox EPERM
log was overwritten by the definitive outside-sandbox run; no replacement raw failure
log was invented. Other original outputs are preserved in `raw-evidence.tar.gz`, with
hashes in `archive-members.json`; JSON copies are unchanged.

This unit does not populate exporters or implement expanded charts. Full verification
belongs to the completed task; CLI/desktop expansion delivery remains TASK-22.8.
