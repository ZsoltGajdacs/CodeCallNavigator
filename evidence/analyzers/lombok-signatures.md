# Lombok signature resolution — TASK-14

Validated on 2026-11-09 with JDK 25 and pinned Lombok 1.18.48.

## Regression and behavior

The original helper fails on the public fixture's `fixture.Value.ValueBuilder`
signature despite successful Lombok compilation. The fixed inventory keeps its
pre-attribution source declaration snapshot and resolves qualified error types
through a separate compiled-only compiler context. Compiler-qualified declaration
names handle generic `@SuperBuilder` error mirrors that retain type arguments even
after erasure. Binary names preserve nested-class dollar separators.

Unqualified unresolved names fail closed, including when an unrelated class of
the same name exists in the default package. Compiled lookup preserves the owning
classpath order and cannot load source files from that classpath. No processor is
run during inventory and no generated builder methods enter the authored graph.

The Rust coordinator writes inventory into a same-directory temporary file and
publishes it by atomic rename after process success and a cancellation check.
Tests cover partial failure, successful retry/cache reuse, running-process
cancellation, cancellation just before publication, and immediate cleanup after
rename failure even while the returned error remains alive.

## Public verification

- `cargo test --test java_lombok -- --ignored --nocapture`: 2 passed, real Maven
  and Gradle builds. Exact signatures cover record/class builders, inherited
  generic builders, builder arrays and generic bounds. Authored edges survive;
  only the seven authored methods remain.
- Existing `java_builds`: 8 passed; duplicate-context acceptance: 2 passed;
  `packaged_java`: 2 passed. The private Recurring test is run separately.
- `cargo test --lib analyzers::java::tests -- --ignored --nocapture`: 4 passed.
- `python3 -m unittest discover -s tests`: 37 passed, including four real-JVM
  inventory boundary tests.
- `cargo test --all-targets` and `cargo clippy --all-targets -- -D warnings`: passed.
- Changed Rust files pass `rustfmt --check`; `git diff --check` passes.
  Repository-wide `cargo fmt --all --check` reports pre-existing formatting in
  vendored Rune sources, which were not changed.
- Independent review findings resolved and re-reviewed. CodeScene reviews score
  all seven eligible changed source files 10.0; safeguard passes with no regressions.
- Sonar is inactive because this project has no `.sonar/config.json`.

An early acceptance attempt raced a helper distribution rebuild and failed to
load an anonymous helper class. The stable-artifact rerun of every affected public
suite passed. The first full private analysis completed but its newly added test
assertion incorrectly compared a short container name against a qualified name;
the assertion was corrected before the final acceptance run.

## Packaging

`python3 tools/package.py` rebuilt, signed and verified `dist/CodeGraph.app` and
produced the ZIP. The packaged helper SHA256 matches the tested development helper:
`94ea6b2da60e26ecd487f7974616ea67574a98f1546f5d33b3590edb787105c9`.

Private full graph JSON, logs and screenshots remain outside this repository.
No private source or credentials are included in this evidence.

## Full Recurring acceptance

The final production Maven acceptance run completed all 13 compile contexts and
passed: **10,800 declarations and 17,293 relationships**. Graph validation passed.
The authored `F3DefaultsServiceImpl.buildCommonDefaults` method occurs once and
has the exact compiled nested-builder return type. Existing assertions retain
both modules' duplicate generated declarations and source/generated ownership.

Reproduction command (requires the existing private checkout/dependency access):

```sh
CODEGRAPH_RECURRING_BILLING=/path/to/Recurring-billing \
  cargo test --test java_duplicates \
  recurring_billing_retains_duplicate_generated_declarations -- --ignored --nocapture
```

The final packaged executable independently analyzed the private project from
scratch and published the same **10,800 symbols / 17,293 relationships** into an
isolated temporary SQLite store. Read-only SQL checks confirmed the exact formerly
failing signature and ten generated classes in each of the two expected modules.
The same packaged executable loaded this generation using `--scene cached` and
rendered the real project graph through headless Metal (181 frames at 60 fps).
The screenshot was visually inspected: the successful-analysis status, project
nodes and relationships are visible, without an analysis error. Native physical
window and trackpad interaction were not repeated for this backend fix.
