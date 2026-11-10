# HTTP and navigation boundaries

`GraphData.boundaries` contains versioned compiler-owned route and request facts.
Routes retain unknown paths, unsupported conditions, explicit order and missing
symbols as blockers. Dropping such a route could turn an ambiguous request into a
false unique match. Native call edges are never routing evidence.

A supported path consists of literal segments and `{name}` single-segment
parameters. The shared matcher supports unique matches and default static route
precedence over single-parameter routes within one application. Other precedence
or runtime behavior remains unresolved. ASP.NET literals use ASCII case-insensitive
comparison; Next and Spring use case-sensitive comparison. Literal requests must
have root-relative ASCII paths without percent escapes, backslashes, dot segments,
repeated separators or trailing separators (except `/`). Query and fragment text
are retained in source evidence but excluded from the path match. Conditions,
custom routes, catchalls and unknown order remain potential competitors.

Applications are identified by `(build, application)`. Application labels are
stable adapter-owned labels rather than checkout directory names. Next defaults
to its own application. Cross-language HTTP requires `.codegraph/boundaries.yml`:

```yaml
version: 1
http:
  - client: web/tsconfig.json
    # Optional when a selected client build contains more than one application.
    client_application: next-app
    servers:
      - api/pom.xml
      - target: services/Services.sln
        application: AppA
```

Targets resolve against the owning `--project` and must be selected for analysis.
Maven POM selectors normalize to the existing Maven directory identity. A plain
server string requires a unique analyzed application; use an explicit application
selector for solutions containing multiple applications. Configured servers extend
same-application Next routing. Candidates in different mapped applications remain
ambiguous; their route priorities are never compared. Invalid configuration fails
publication. Both missing and present configuration files are fingerprinted.

Boundary relationships have `navigation` or `http` kind, possible certainty, and
source/handler evidence. Actual call sites receive flow targets carrying the same
explicit kind; legacy flow targets default to `call` and retain their old JSON
representation. JSX links do not invent flow calls. Re-resolution discards owned
boundary derivations before using current route/request facts. One canonical graph
file remains the only project graph artifact; route facts are preserved in that
file and in the derived desktop SQLite cache.
