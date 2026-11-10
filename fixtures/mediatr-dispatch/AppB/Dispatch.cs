using DispatchContracts;
using MediatR;

namespace DispatchAppB;

public static class Dispatch
{
    public static Task<int> SendPing(ISender sender, CancellationToken token) =>
        sender.Send(new Ping(20), token);

    public static Task PublishChanged(IPublisher publisher, CancellationToken token) =>
        publisher.Publish(new Changed("b"), token);
}

public sealed class PingHandler : IRequestHandler<Ping, int>
{
    public Task<int> Handle(Ping request, CancellationToken cancellationToken) =>
        Task.FromResult(request.Value + 100);
}

public sealed class ChangedHandler : INotificationHandler<Changed>
{
    public Task Handle(Changed notification, CancellationToken cancellationToken) => Task.CompletedTask;
}
