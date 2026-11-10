# Compiler-bound MediatR dispatch fixture

Analyze `MediatRDispatch.sln` as one target. `Contracts` contains shared message
types; `AppA` and `AppB` each reference `Contracts`, without referencing each
other. Their handlers must remain separate even when payload symbols are shared.

The fixture uses .NET SDK 10.0.403, targets `net10.0`, and pins the public MediatR
package to exactly 12.4.1. Each project commits its NuGet lock file. The local
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

The single `.codegraph/graph.json` is recorded before inference and then updated
with reviewed changes. Git retains the baseline history.
