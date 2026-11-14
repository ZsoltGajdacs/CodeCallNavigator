namespace CleanupFixture;

public static class CleanupCases
{
    public static int UseScope()
    {
        using ValueResource resource = TraceRuntime.Acquire(new("one"));
        TraceRuntime.Mark(new("body"));
        return 0;
    }

    public static int AcquisitionFailure()
    {
        using (ValueResource first = TraceRuntime.Acquire(new("first")),
               second = TraceRuntime.Acquire(new("second"), ResourceFault.Acquire))
        {
            TraceRuntime.Mark(new("impossible-body"));
        }
        return 0;
    }

    public static int DisposeThrowsOuter()
    {
        using ValueResource outer = TraceRuntime.Acquire(new("outer"));
        using ValueResource inner = TraceRuntime.Acquire(new("inner"), ResourceFault.Dispose);
        TraceRuntime.Mark(new("body"));
        return 0;
    }

    public static int AcquisitionFailureCaught()
    {
        try
        {
            using (ValueResource first = TraceRuntime.Acquire(new("first")),
                   second = TraceRuntime.Acquire(new("second"), ResourceFault.Acquire))
            {
                TraceRuntime.Mark(new("impossible-body"));
            }
        }
        catch (ProbeException) { TraceRuntime.Mark(new("caught")); }
        TraceRuntime.Mark(new("after-catch"));
        return 0;
    }

    public static int NestedUsing()
    {
        using ValueResource outer = TraceRuntime.Acquire(new("outer"));
        {
            using (ValueResource inner = TraceRuntime.Acquire(new("inner"))) TraceRuntime.Mark(new("body"));
            TraceRuntime.Mark(new("after-inner"));
        }
        TraceRuntime.Mark(new("after-body"));
        return 0;
    }

    public static int ReturnBeforeDispose()
    {
        using ValueResource resource = TraceRuntime.Acquire(new("one"));
        return TraceRuntime.ReturnValue();
    }

    public static int CaptureMutation()
    {
        ReferenceResource current = TraceRuntime.AcquireReference(new("original"));
        using (current)
        {
            current = TraceRuntime.AcquireReference(new("replacement"));
            TraceRuntime.Mark(new("mutated"));
        }
        return 0;
    }

    public static int NullReference()
    {
        ReferenceResource? missing = TraceRuntime.AcquireNull(new("null"));
        using (missing) TraceRuntime.Mark(new("null-body"));
        return 0;
    }

    public static int NullNullable()
    {
        ValueResource? missing = TraceRuntime.AcquireNullable(new("nullable"));
        using (missing) TraceRuntime.Mark(new("nullable-body"));
        return 0;
    }

    public static int LoopTransfers()
    {
        for (int index = 0; index < 2; index++)
        {
            using ValueResource iteration = TraceRuntime.Acquire(new("iteration"));
            TraceRuntime.Mark(new("body"));
            if (index == 0) continue;
            break;
        }
        TraceRuntime.Mark(new("after-loop"));
        return 0;
    }
}
