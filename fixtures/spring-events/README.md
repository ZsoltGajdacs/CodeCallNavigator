# Compiler-bound Spring event fixture

Analyze `pom.xml` with JDK 25 and the pinned CodeGraph Java adapter. The project
compiles Java 17 source against actual Spring context/transaction 6.2.5 libraries;
the versions and Maven plugins are pinned in the POM. All dependencies are already
available in the local Maven cache. No private application build is involved.

`Publisher.publishPayload` and `publishPayloadTwice` publish the plain qualified
`fixture.spring.events.Created` payload. Spring wraps it in
`PayloadApplicationEvent<Created>`, so payload, explicit-class, envelope and broad
ApplicationEvent listeners are candidates. Conditional, transactional and async
listeners are possibilities with their respective qualifiers, not synchronous
execution guarantees. The twice-publishing method must retain both source sites
without multiplying a single site's invocation count.

`publishApplicationEvent` publishes `Started` directly and can reach the typed
ApplicationListener implementation and the broad ApplicationEvent listener.

Negative controls retain ordinary compiler bindings but must not create inferred
Spring edges: an unrelated same-simple-name payload type, a same-named custom
publisher API, a custom EventListener annotation, an unannotated method, and
Object-erased/type-variable payloads. Missing DI registration, transaction state,
event conditions and async executor configuration cannot be proved by source
analysis; inferred targets never claim guaranteed runtime execution.

The HTTP fixture also compiles against actual Spring Web 6.2.5. `HttpController`
provides literal GET/POST handlers, a single-segment parameter and a class prefix.
The same path with a different verb is independent. Custom same-simple-name
`GetMapping` is unrelated. `ConstrainedController` retains both same-path handlers
with differing `produces`, and both the unconstrained and query-constrained
`/api/constrained` handlers: unsupported constraints must remain blockers so a
unique-looking surviving candidate cannot create a false HTTP relationship.
