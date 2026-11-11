# Double Shift search verification

The four Metal captures show the production modal with the explicit synthetic
fixture, in light/dark themes at 1050×680 and 1420×900. They were visually
inspected for horizontal centering, window bounds, input focus, readable result
context, selection, and reachable pagination. They do not claim physical keyboard
or VoiceOver validation.

Reproduce after building the desktop:

```sh
./target/debug/codegraph --scene fixture --script evidence/ui/search-modal/search.json \
  --snap /tmp/search.png --tree /tmp/search.tree.json \
  --theme dark --size 1050x680 --scale 1 --at 2 --realtime
```

Use `--theme light` and `--size 1420x900` for the remaining matrix entries.
Metal access is required. Both the graph and flowchart workspaces use the same
modal composition; production Rune tests cover opening and activating from each.

The search runtime regressions cover Double Shift interruptions, actual input
focus and typing, dismissal and focus restoration, category filtering, hidden
class/method reveal, navigation history, architectural layers, and stale queries
and graph replacement. Core regressions cover substring/rank behavior, Unicode,
literal punctuation, overloads, duplicate class scopes, aliases, and paging.

Validation results and any unavailable acceptance fixtures are recorded on
TASK-37. The upstream observers are recorded in the pinned, checksum-verified
`patches/rune-input-observers.patch`; dependency migration tests cover both prior
patch checkpoints.

## Validation results

- Desktop check and all-target tests passed; the desktop binary has 178 passing
  normal tests, including 11 modal regressions and five shortcut-detector tests.
- Core check, all 46 core tests, and strict workspace Clippy passed.
- Full workspace acceptance ran with `--include-ignored --no-fail-fast`. Its five
  initial failures were prerequisite-related: three benchmarks require release
  mode, and two billing tests require explicit private-fixture inputs. All five
  passed in their appropriate reruns: the three release benchmarks used an isolated
  saved-workspace copy; billing source and compiled-metadata acceptance used an
  isolated fixture copy with freshly rebuilt outputs (10,827 declarations and
  17,331 relationships). The stale copied compiled outputs initially could not
  resolve `ListRecurring`; rebuilding the fixture resolved it without code changes.
- Python packaging tests: 76 passed. Dependency-patch migration tests: seven
  passed. Rune input-observer integration: four passed.
- `tools/package.py` built and signed the app and ZIP. A real-time Metal replay of
  the packaged app confirmed input, modal mounting, and completed search results.
- CodeScene reviewed affected project and upstream sources; project implementation
  and test modules score 10.0 (the export-only core module has no numeric score).
  The safeguard passed for all 23 eligible files. Existing upstream complexity
  findings were left outside this feature. Sonar is inactive: this repository has
  no `.sonar/config.json`.

The plain debug full-suite command still cannot run release-only benchmarks in
its debug process. Passing release reruns are recorded separately; no benchmark
or missing-fixture check is represented as passing without execution.

## Result activation follow-up (TASK-38)

Class hits now focus the selected class card, with class-level connections, rather
than drilling into separate method nodes. Existing class-card member previews are
preserved. Method hits explicitly switch to Method grouping and select the found
method. Both reuse the layout-completion centering operation.

`class-result.png` and `method-result.png` were captured from the rebuilt packaged
app and visually inspected. Reproduce with the corresponding JSON script and
`--realtime --scene fixture --theme dark --size 1420x900 --scale 1 --at 2.5`.

Twelve modal runtime regressions pass, including camera coordinates within 0.1
points of the graph viewport center, selected target identity, switching from
method to class and back, and one history entry per activation. The normal desktop
suite passes 179 tests. Desktop check, strict workspace Clippy, packaging, and
CodeScene reviews/safeguard pass; both changed Rust files score 10.0. Independent
review found no remaining issues.

The required debug full-workspace command again stops at the three existing
release-only benchmark guards (183 other desktop tests pass). All three benchmark
cases pass in the separate release run against the isolated saved-workspace copy.
Initial normal-suite prerequisite errors were resolved by rebuilding the standalone
engine after the build directory had been cleaned.
