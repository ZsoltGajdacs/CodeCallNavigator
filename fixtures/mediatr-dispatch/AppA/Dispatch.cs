using DispatchContracts;
using MediatR;

namespace DispatchAppA;

public static class Dispatch
{
    public static Task<int> SendPing(ISender sender, CancellationToken token) =>
        sender.Send<int>(new Ping(7), token);

    public static Task SendReset(ISender sender, CancellationToken token) =>
        sender.Send(new Reset(), token);

    public static Task PublishChanged(IPublisher publisher, CancellationToken token) =>
        publisher.Publish(new Changed("a"), token);

    public static Task<int> NamedArguments(ISender sender, CancellationToken token) =>
        sender.Send(cancellationToken: token, request: new Ping(8));

    public static async Task<int> Conditional(ISender sender, bool enabled, CancellationToken token)
    {
        if (enabled)
            return await sender.Send(new Ping(9), token);
        return 0;
    }

    public static Task<int> LocalFunction(ISender sender, CancellationToken token)
    {
        Task<int> Invoke() => sender.Send(new Ping(10), token);
        return Invoke();
    }

    public static Task<object?> ErasedObject(ISender sender, object request, CancellationToken token) =>
        sender.Send(request, token);

    public static Task<int> ErasedContract(ISender sender, IRequest<int> request, CancellationToken token) =>
        sender.Send(request, token);

    public static Task<int> UnknownRequest(ISender sender, CancellationToken token) =>
        sender.Send(new Unhandled(), token);

    public static Task<int> DerivedDoesNotUseBaseHandler(ISender sender, CancellationToken token) =>
        sender.Send<int>(new DerivedRequest(), token);

    public static Task<int> OpenGenericDeferred(ISender sender, CancellationToken token) =>
        sender.Send(new GenericRequest<int>(12), token);

    public static Func<Task<int>> AnonymousDeferred(ISender sender, CancellationToken token) =>
        () => sender.Send(new Ping(13), token);

    public static Task<int> CustomSend(FakeSender sender) => sender.Send(new Ping(14));
}

public sealed class FakeSender
{
    public Task<int> Send(Ping request) => Task.FromResult(request.Value);
}
