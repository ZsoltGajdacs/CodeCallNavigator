using CleanupFixture;

ReportOnExit();
using ValueResource resource = TraceRuntime.Acquire(new("top"));
TraceRuntime.Mark(new("top-after"));

// The report belongs to this executable and runs after compiler-generated disposal.
static void ReportOnExit() => AppDomain.CurrentDomain.ProcessExit +=
    (_, _) => TraceRuntime.Report(new("top-level-using", 0, null));
