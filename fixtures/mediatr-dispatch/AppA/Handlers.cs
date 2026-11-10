using DispatchContracts;
using MediatR;

namespace DispatchAppA;

public sealed class PingHandler : IRequestHandler<Ping, int>
{
    public Task<int> Handle(Ping request, CancellationToken cancellationToken) =>
        Task.FromResult(request.Value + 1);
}

public sealed class ResetHandler : IRequestHandler<Reset>
{
    Task IRequestHandler<Reset>.Handle(Reset request, CancellationToken cancellationToken) =>
        Task.CompletedTask;
}

public sealed class FirstChangedHandler : INotificationHandler<Changed>
{
    public Task Handle(Changed notification, CancellationToken cancellationToken) => Task.CompletedTask;
}

public abstract class ChangedHandlerBase : INotificationHandler<Changed>
{
    public Task Handle(Changed notification, CancellationToken cancellationToken) => Task.CompletedTask;
}

public sealed class InheritedChangedHandler : ChangedHandlerBase;

public sealed class BaseRequestHandler : IRequestHandler<BaseRequest, int>
{
    public Task<int> Handle(BaseRequest request, CancellationToken cancellationToken) => Task.FromResult(1);
}

public sealed class OpenGenericHandler<T> : IRequestHandler<GenericRequest<T>, T>
{
    public Task<T> Handle(GenericRequest<T> request, CancellationToken cancellationToken) =>
        Task.FromResult(request.Value);
}
