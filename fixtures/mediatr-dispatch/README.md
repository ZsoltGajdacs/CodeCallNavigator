# Compiler-bound MediatR dispatch fixture

Analyze `MediatRDispatch.sln` as one target. `Contracts` contains shared message
types; `AppA` and `AppB` each reference `Contracts`, without referencing each
other. Their handlers must remain separate even when payload symbols are shared.

The fixture uses .NET SDK 11.0.100 (latest patch), targets `net11.0`, and pins
the public MediatR package to exactly 12.4.1. Each project commits its NuGet lock file. The local
`NuGet.config` clears inherited sources and uses only nuget.org; no global NuGet
configuration changes are required.

```sh
dotnet restore MediatRDispatch.sln --configfile NuGet.config --locked-mode
dotnet build MediatRDispatch.sln --no-restore --configuration Release \
  --disable-build-servers -m:1 -p:UseSharedCompilation=false
```

The fixture compiles against the actual framework interfaces. It does not set
up dependency injection or claim that any handler is registered at runtime.
Inferred dispatch targets therefore describe possible framework relationships.

| Dispatch shape | Intended resolution |
| --- | --- |
| AppA `SendPing`, `NamedArguments`, `Conditional`, local `Invoke` | AppA `PingHandler.Handle` only |
| AppA `SendReset` | Explicit `IRequestHandler<Reset>.Handle` implementation |
| AppA `PublishChanged` | `FirstChangedHandler.Handle` and inherited `ChangedHandlerBase.Handle` |
| AppB `SendPing` | AppB `PingHandler.Handle` only |
| AppB `PublishChanged` | AppB `ChangedHandler.Handle` only |
| `CustomSend` | Native call to `FakeSender.Send`; no framework relationship |
| `ErasedObject`, `ErasedContract` | Defer unknown concrete payload type |
| `UnknownRequest` | Defer missing handler |
| `DerivedDoesNotUseBaseHandler` | No base-request handler substitution |
| `OpenGenericDeferred` | Defer open generic handler construction |
| `AnonymousDeferred` | Defer anonymous callable without a native callable identity |

`SendPing` explicitly supplies the response type argument `int`; the request
type must come from the actual argument. `NamedArguments` reverses source
argument order, requiring compiler parameter mapping. The conditional site
retains its branch and `await` facts; returning `Task` does not prove runtime
scheduling. The abstract inherited handler
base is not an additional concrete candidate. No C# flow inventory is fabricated.

The single `.codecallnavigator/graph.json` is recorded before inference and then updated
with reviewed changes. Git retains the baseline history.

## Attributed ASP.NET controllers

`AppA` and `AppB` reference the installed `Microsoft.AspNetCore.App` shared
framework. Both expose `/api/items` from different compiler assemblies; a
solution selection alone must not merge their application origins.

The controller fixture includes literal and single-parameter GET routes, POST,
class/method prefix composition, `/` and `~/` rooted templates, and multiple
templates on one action. AppA's `CaseOnly`/`caseonly` actions deliberately
collide under ASP.NET's case-insensitive routing policy; AppB has a single
case variant in its separate application.

The constrained route and its overlapping literal, nonzero method/class route
orders, and catch-all template must remain visible as candidates or blockers.
Unsupported declarations must not disappear and make a competing route appear
unique. `[NonAction]`, nonpublic/static/generic actions, `[NonController]`, and
open generic controllers are exclusions. The source-owned `PretendRouting`
attribute must not be confused with an ASP.NET route contract.

The fixture intentionally produces four ASP0023 warnings for its case/order
collision controls; these are expected and are not compilation errors.

These classes only compile and supply analyzer input; no server is started and
no route or dependency-injection configuration is executed.
