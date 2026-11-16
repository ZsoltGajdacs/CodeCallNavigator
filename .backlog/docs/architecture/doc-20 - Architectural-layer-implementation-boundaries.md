---
id: doc-20
title: Architectural layer implementation boundaries
type: guide
created_date: '2026-11-16T07:06:00Z'
updated_date: '2026-11-16T07:16:00Z'
---
# Architectural layer implementation boundaries

Migrated 2026-11-16 from `docs/architectural-layers.md` at project revision `13b0de13f3ddf904145f93119816c33b8a5b3e55`. This is the canonical internal documentation; public user and wire contracts remain under `docs/`.

## Cache and module boundaries

Project graph artifacts retain compiler-owned per-file declaration facts **and source
text** so content rules can be reevaluated without mixing current files
with an older graph. This differs from older versions, which did not cache source
text. Source popups request current files through the CLI. Refresh after source changes
to update facts; older artifacts remain usable in ordinary groupings and require a
refresh before Layer mode can classify them.

The shared `architecture` library separates native validation, review-format import,
pattern compilation, classification and filesystem loading. Analyzer adapters export
facts only. Layer assignments are disposable presentation data. Native workspace
jobs request CLI reloads and classification; projection owns aggregation and band metadata; Graphviz owns
geometry. No layer rule changes graph symbol identities or analyzer call bindings.
