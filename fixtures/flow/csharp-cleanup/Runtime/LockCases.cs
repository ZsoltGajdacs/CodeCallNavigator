namespace CleanupFixture;

public static class LockCases
{
    private static readonly object GateObject = new();

    public static object Gate()
    {
        TraceRuntime.Record(new(TraceOperation.Gate, "lock", TraceOutcome.Normal));
        return GateObject;
    }

    public static void Inside()
    {
        var entered = Monitor.IsEntered(GateObject);
        TraceRuntime.Record(new(TraceOperation.LockInside, "lock", TraceOutcome.Normal, entered.ToString().ToLowerInvariant()));
        if (!entered) throw new InvalidOperationException("Genuine lock body does not own the monitor");
    }

    public static int Normal()
    {
        lock (Gate()) { Inside(); TraceRuntime.Mark(new("locked-body")); }
        TraceRuntime.Mark(new("after-lock"));
        return 0;
    }

    public static int Throwing()
    {
        lock (Gate()) { Inside(); TraceRuntime.Mark(new("locked-body")); throw new ProbeException("locked-body"); }
    }

    // This observation runs outside the selected callable and on a genuinely different thread.
    public static object ReleaseProbe()
    {
        var caller = Environment.CurrentManagedThreadId;
        var workerId = caller;
        var released = false;
        var worker = new Thread(() =>
        {
            workerId = Environment.CurrentManagedThreadId;
            released = Monitor.TryEnter(GateObject, 1000);
            if (released) Monitor.Exit(GateObject);
        });
        worker.Start();
        if (!worker.Join(3000)) throw new InvalidOperationException("Lock release probe timed out");
        return new { callerThread = caller, workerThread = workerId, released };
    }
}
