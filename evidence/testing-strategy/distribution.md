# Current test distribution

Counts are executable registrations, kept separate from standalone harnesses.
Acceptance means end-to-end coverage through a public application boundary.

| Component | Unit | Integration | End-to-end | Performance |
| --- | ---: | ---: | ---: | ---: |
| UI | 174 | 247 | 0 | 3 |
| CORE | 109 | 109 | 0 | 0 |
| ENGINE | 50 | 233 | 81 | 0 |
| MCP | 16 | 16 | 4 | 0 |
| TOOLING | 58 | 258 | 0 | 0 |
| **Total** | 407 | 863 | 85 | 3 |

| Standalone harness owner | Integration | End-to-end | Performance |
| --- | ---: | ---: | ---: |
| UI | 0 | 2 | 0 |
| CORE | 0 | 0 | 1 |
| ENGINE | 1 | 5 | 1 |
| MCP | 0 | 1 | 0 |
| TOOLING | 0 | 0 | 0 |

Total: **1,358 registrations and 11 standalone harnesses**.

A harness may contain many scenarios; its count is not comparable to a single test.
Rust lib/bin registrations and inherited Python cases count separately where they
are actually registered. Fixture loading alone does not determine the category.

The thirteen classification corrections and legacy OS simulation limits are
recorded in [the source review](classification-review.json). The placement mappings
preserve existing assertions; no tests were added merely to create a pyramid ratio.
The pyramid guides future responsibility, rather than making current ratios an
acceptance gate.

Inventory SHA-256: `15c74b6be5d4586be77f7ae60137c580457c263cbd6311fc7fa123f4ea8c1b93`.
