using MediatR;

namespace DispatchContracts;

public sealed record Ping(int Value) : IRequest<int>;
public sealed record Reset : IRequest;
public sealed record Changed(string Value) : INotification;
public record BaseRequest : IRequest<int>;
public sealed record DerivedRequest : BaseRequest;
public sealed record Unhandled : IRequest<int>;
public sealed record GenericRequest<T>(T Value) : IRequest<T>;
