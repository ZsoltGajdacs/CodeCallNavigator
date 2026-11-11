# Maven build fixtures

The reactor includes a sibling library, an active-by-default profile module,
a local parent outside the reactor directory, custom source/output directories,
generated Java, an external resource directory, and a JUnit test that writes `tests-executed` if executed.
The inactive profile module must stay outside the analyzed graph.
The standalone fixture shares the external parent without joining the reactor.

Run the real acceptance checks with bootstrapped artifacts and JDK 25:

```sh
cargo test --test java_builds -- --ignored --nocapture
```

To exercise a pinned Maven distribution without replacing the installed Maven,
set `CODECALLNAVIGATOR_TEST_MAVEN` to its absolute `bin/mvn` path. The tests install a
fixture-local wrapper that delegates to that executable. Fixture copies use paths
containing spaces, and tests never compile these tracked directories in place.
