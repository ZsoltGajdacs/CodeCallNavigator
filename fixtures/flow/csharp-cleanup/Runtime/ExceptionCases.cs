namespace CleanupFixture;

public static class ExceptionCases
{
    public static int Rethrow()
    {
        try
        {
            try { TraceRuntime.Mark(new("body")); throw new ProbeException("body"); }
            catch (ProbeException) { TraceRuntime.Mark(new("caught")); throw; }
            finally { TraceRuntime.Mark(TraceRuntime.InnerFinallyMarker); }
        }
        finally { TraceRuntime.Mark(TraceRuntime.OuterFinallyMarker); }
    }

    public static int FinallyThrows()
    {
        try { TraceRuntime.Mark(new("body")); return TraceRuntime.ReturnValue(); }
        finally { TraceRuntime.Mark(TraceRuntime.FinallyMarker); throw new ProbeException("finally"); }
    }
}
