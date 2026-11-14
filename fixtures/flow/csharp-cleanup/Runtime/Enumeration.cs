using System.Collections;

namespace CleanupFixture;

public enum EnumerationFault { None, Acquire, MoveNext, Current, Dispose }

public sealed class ProbeEnumerable(EnumerationFault fault)
{
    public ProbeEnumerator GetEnumerator()
    {
        TraceRuntime.Record(new(TraceOperation.GetEnumerator, "enumerator", fault == EnumerationFault.Acquire ? TraceOutcome.Throw : TraceOutcome.Normal));
        if (fault == EnumerationFault.Acquire) throw new ProbeException("enumerator-acquire");
        return new(fault);
    }
}

public struct ProbeEnumerator(EnumerationFault fault) : IEnumerator<int>
{
    private int index = -1;

    public bool MoveNext()
    {
        index++;
        if (fault == EnumerationFault.MoveNext)
        {
            TraceRuntime.Record(new(TraceOperation.MoveNext, "enumerator", TraceOutcome.Throw));
            throw new ProbeException("enumerator-move");
        }
        var available = index < 2;
        TraceRuntime.Record(new(TraceOperation.MoveNext, "enumerator", TraceOutcome.Normal, available ? "true" : "false"));
        return available;
    }

    public int Current
    {
        get
        {
            TraceRuntime.Record(new(TraceOperation.Current, "enumerator", fault == EnumerationFault.Current ? TraceOutcome.Throw : TraceOutcome.Normal));
            if (fault == EnumerationFault.Current) throw new ProbeException("enumerator-current");
            return index;
        }
    }

    object IEnumerator.Current => Current;
    public void Reset() => throw new NotSupportedException();

    public void Dispose()
    {
        TraceRuntime.Record(new(TraceOperation.Dispose, "enumerator", fault == EnumerationFault.Dispose ? TraceOutcome.Throw : TraceOutcome.Normal));
        if (fault == EnumerationFault.Dispose) throw new ProbeException("enumerator-dispose");
    }
}
