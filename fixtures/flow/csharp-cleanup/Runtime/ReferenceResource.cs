using System.Runtime.CompilerServices;

namespace CleanupFixture;

public sealed class ReferenceResource(ProbeName name) : IDisposable
{
    public void Dispose() => TraceRuntime.Record(new(TraceOperation.Dispose, name.Value, TraceOutcome.Normal));

    public static bool operator ==(ReferenceResource? left, ReferenceResource? right)
    {
        TraceRuntime.Record(new(TraceOperation.BadNullOperator, "equals", TraceOutcome.Normal));
        return ReferenceEquals(left, right);
    }

    public static bool operator !=(ReferenceResource? left, ReferenceResource? right)
    {
        TraceRuntime.Record(new(TraceOperation.BadNullOperator, "not-equals", TraceOutcome.Normal));
        return !ReferenceEquals(left, right);
    }

    public override bool Equals(object? other) => ReferenceEquals(this, other);
    public override int GetHashCode() => RuntimeHelpers.GetHashCode(this);
}
