namespace CleanupFixture;

public static class BoundaryCases
{
    private static bool Filter(bool accepted)
    {
        TraceRuntime.Mark(new("filter"));
        return accepted;
    }

    public static int Filtered(bool accepted)
    {
        try
        {
            try { TraceRuntime.Mark(new("try-body")); throw new ProbeException("filtered"); }
            catch (ProbeException) when (Filter(accepted)) { TraceRuntime.Mark(new("filtered-catch")); }
        }
        finally { TraceRuntime.Mark(TraceRuntime.FilterFinallyMarker); }
        return 0;
    }

    public static async Task<int> AsyncBody()
    {
        TraceRuntime.Mark(new("async-before"));
        await Task.Yield();
        TraceRuntime.Mark(new("async-after"));
        return 0;
    }

    public static async Task<int> AwaitUsing()
    {
        await using var resource = new AsyncResource();
        TraceRuntime.Mark(new("await-using-body"));
        return 0;
    }

    public static async IAsyncEnumerable<int> AsyncItems()
    {
        await Task.Yield();
        yield return 1;
    }

    public static async Task<int> AwaitForeach()
    {
        await foreach (var item in AsyncItems()) TraceRuntime.Mark(new("await-foreach-body"));
        return 0;
    }

    public static IEnumerable<int> Iterator()
    {
        TraceRuntime.Mark(new("iterator-body"));
        yield return 1;
    }

    private static bool Guard()
    {
        TraceRuntime.Mark(new("guard"));
        return true;
    }

    public static int GuardedSwitch()
    {
        return 1 switch { _ when Guard() => Arm(), _ => 0 };
    }

    private static int Arm() { TraceRuntime.Mark(new("guarded-arm")); return 1; }

    public static int Goto()
    {
        goto Done;
        TraceRuntime.Mark(new("unreachable"));
        Done: TraceRuntime.Mark(new("after-goto"));
        return 0;
    }
}

public sealed class AsyncResource : IAsyncDisposable
{
    public ValueTask DisposeAsync()
    {
        TraceRuntime.Mark(new("dispose-async"));
        return ValueTask.CompletedTask;
    }
}
