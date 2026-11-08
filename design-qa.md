# Graph Studio Rune Kit design QA

final result: passed

## Visual truth and comparison

- Source: `evidence/ui/graph-studio/reference-graph-studio.png`, the third displayed concept selected by the user (Graph Studio).
- Implementation: `evidence/ui/graph-studio/real-dark.png`, captured by the native Rune Metal renderer from the production view.
- Source pixels: 1487 × 1058. Its intended desktop frame was 1440 × 1024; the generated output has a slightly different raster size. Implementation: 1420 × 900 logical points and pixels at 1×, retaining the approved native window default.
- `comparison.png` uniformly normalizes source width to 1420 without stretching; vertical excess remains visible beside the shorter native viewport. This is an approved layout interpretation, not a pixel-identical clone.
- State: Search open, method selected, Callees, depth 2. The concept uses Java data; implementation uses a real cached PacificFate C# neighborhood. Existing graph topology, supported directions and data semantics take precedence over invented mock content.
- Full-view evidence: `evidence/ui/graph-studio/comparison.png`.
- Focused evidence: `comparison-controls.png` and `comparison-inspector.png` retain original pixels for control/typography inspection.
- Additional screenshots cover fixture and real-project states in dark/light at 1420 × 900 and 1050 × 680. All eight captures and element trees are under `evidence/ui/graph-studio/`.

## Findings and fidelity surfaces

No actionable P0/P1/P2 findings remain.

- **Typography:** shared Kit UI type uses 14-point primary controls, smaller secondary labels and system monospace for graph names/signatures. Namespace remains fully readable. Long identifiers and signatures wrap within the inspector; graph labels and search rows deliberately truncate, with full accessible labels and selected-method details available.
- **Spacing/layout:** 48-point rail, resizable drawer/inspector, dominant canvas, top graph navigation and bottom direction/depth/zoom. Minimum windows use extra control rows; pane budgeting keeps at least 448 points for the center. Inputs stay inside 12-point drawer padding; inspector content stays inside 16-point padding. Scrolling retains access to lower sections.
- **Colors/tokens:** coordinated graphite and light neutral appearances, blue selection, amber uncertain edges/cycle outlines and purple callbacks. Solid backgrounds replace the concept's decorative glow. Kit supplies focus, disabled and interaction states.
- **Assets:** no raster assets are needed in this developer workspace. Icons come from the pinned Rune OpenCode catalog and Kit glyphs; source paths are copied unchanged with attribution. The graph remains actual Rune shapes rather than a bitmap or decorative imitation.
- **Copy/content:** existing discovery, refresh, failure, provenance and graph-count messages remain. The unsupported tree and Both direction from the generated mock are intentionally absent, as agreed. Full source paths and actual relationship certainty remain inspectable.

## Comparison history

1. Review found that maximum pane widths could hide graph controls at minimum window size. Added a center-width budget, reapplication during live drags, and multirow controls. Runtime tests cover restored maximum widths and dragging after initial layout.
2. Initial captures showed a truncated Namespace label and fixed-width Kit inputs extending past drawer padding. Increased grouping width with corresponding center budget and set inputs to fill available width. The final screenshot matrix and explicit geometry regressions verify both corrections.
3. Empty-state runtime testing found its prompt was laid out after the canvas primitive and clipped. Positioned the empty state as a canvas overlay; actual empty/loading/discovery-error tests now pass.
4. Final full-frame and focused comparisons show no overlapping persistent controls or unreadable panel content. Long identifiers wrap; smaller fitted graph labels reflect the existing camera scale and remain zoomable.

## Behavior and accessibility

Real Rune runtime tests cover rail switching/collapse, Cmd-F actual editor focus, keyboard segmented selection, hidden-symbol search reveal, filters, depth selection and Escape priority, disclosures, disabled actions, pane resize/collapse/reset, persistence, camera/history preservation, appearance switching, reduced motion, empty states and actual asynchronous directory discovery errors. Existing graph/navigation tests remain.

This is a native application, so browser console checks do not apply. Metal captures and runtime interaction verification are complete; physical trackpad/window-shell interaction and VoiceOver use were not exercised.

## Follow-up polish

- P3: long method names can wrap within a camel-case identifier in a narrow inspector. Resizing the inspector reveals more of the name on a line; the full signature remains selectable and readable.

## Implementation checklist

- [x] Shared Rune Kit theme and focused components
- [x] Default/minimum dark/light visual inspection
- [x] Source/full-frame/focused comparison
- [x] Runtime interaction, responsive geometry and presentation compatibility checks
- [x] No actionable P0/P1/P2 findings
