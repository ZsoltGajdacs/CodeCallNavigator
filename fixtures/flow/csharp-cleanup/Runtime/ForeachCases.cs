namespace CleanupFixture;

public static class ForeachCases
{
    public static int Run(EnumerationFault fault)
    {
        foreach (int item in new ProbeEnumerable(fault)) TraceRuntime.Mark(new("item"));
        TraceRuntime.Mark(new("after-loop"));
        return 0;
    }

    public static int Break(EnumerationFault fault)
    {
        foreach (int item in new ProbeEnumerable(fault))
        {
            TraceRuntime.Mark(new("item"));
            break;
        }
        TraceRuntime.Mark(new("after-loop"));
        return 0;
    }

    public static int Continue()
    {
        foreach (int item in new ProbeEnumerable(EnumerationFault.None))
        {
            TraceRuntime.Mark(new("item"));
            if (TraceRuntime.ShouldContinue(item)) continue;
            TraceRuntime.Mark(new("tail-item"));
        }
        TraceRuntime.Mark(new("after-loop"));
        return 0;
    }

    public static int BodyFailure()
    {
        foreach (int item in new ProbeEnumerable(EnumerationFault.None))
        {
            TraceRuntime.Mark(new("body"));
            throw new ProbeException("foreach-body");
        }
        return 0;
    }
}
