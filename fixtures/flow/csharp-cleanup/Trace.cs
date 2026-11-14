using System.Text.Json;
using System.Text.Json.Serialization;

namespace CleanupFixture;

public enum TraceOperation { Acquire, Mark, ReturnValue, Dispose, GetEnumerator, MoveNext, Current, Gate, LockInside, ContinueDecision, BadNullOperator }
public enum TraceOutcome { Normal, Throw }
public enum ResourceFault { None, Acquire, Dispose }
public readonly record struct ProbeName(string Value);
public sealed record TraceEvent(TraceOperation Operation, string Name, TraceOutcome Outcome, string? Value = null);
public sealed record RunResult(string Scenario, int? Result, string? Error, object? LockProbe = null);
public sealed class ProbeException(string message) : Exception(message);

public static class TraceRuntime
{
    private static readonly List<TraceEvent> Events = [];
    public static readonly ProbeName FilterFinallyMarker = new("filter-outer-finally");
    public static readonly ProbeName InnerFinallyMarker = new("inner-finally");
    public static readonly ProbeName OuterFinallyMarker = new("outer-finally");
    public static readonly ProbeName FinallyMarker = new("finally");
    public static int SharedValue { get; set; } = 7;

    public static void Record(TraceEvent observation) => Events.Add(observation);

    public static ValueResource Acquire(ProbeName name, ResourceFault fault = ResourceFault.None)
    {
        Record(new(TraceOperation.Acquire, name.Value, fault == ResourceFault.Acquire ? TraceOutcome.Throw : TraceOutcome.Normal));
        if (fault == ResourceFault.Acquire) throw new ProbeException("acquire:" + name.Value);
        return new(name, fault);
    }

    public static void Mark(ProbeName name) => Record(new(TraceOperation.Mark, name.Value, TraceOutcome.Normal));

    public static bool ShouldContinue(int item)
    {
        var skip = item == 0;
        Record(new(TraceOperation.ContinueDecision, "iteration", TraceOutcome.Normal, skip ? "true" : "false"));
        return skip;
    }

    public static int ReturnValue()
    {
        Record(new(TraceOperation.ReturnValue, "result", TraceOutcome.Normal, SharedValue.ToString()));
        return SharedValue;
    }

    public static ReferenceResource AcquireReference(ProbeName name)
    {
        Record(new(TraceOperation.Acquire, name.Value, TraceOutcome.Normal));
        return new(name);
    }

    public static ReferenceResource? AcquireNull(ProbeName name)
    {
        Record(new(TraceOperation.Acquire, name.Value, TraceOutcome.Normal, "null"));
        return null;
    }

    public static ValueResource? AcquireNullable(ProbeName name)
    {
        Record(new(TraceOperation.Acquire, name.Value, TraceOutcome.Normal, "null"));
        return null;
    }

    public static void Report(RunResult run)
    {
        var options = new JsonSerializerOptions { PropertyNamingPolicy = JsonNamingPolicy.CamelCase };
        options.Converters.Add(new JsonStringEnumConverter(JsonNamingPolicy.KebabCaseLower));
        Console.WriteLine(JsonSerializer.Serialize(new
        {
            run.Scenario, events = Events, run.Result, outcome = run.Error is null ? TraceOutcome.Normal : TraceOutcome.Throw,
            run.Error, sharedValue = SharedValue, run.LockProbe,
        }, options));
    }
}

public readonly struct ValueResource(ProbeName name, ResourceFault fault) : IDisposable
{
    public void Dispose()
    {
        TraceRuntime.Record(new(TraceOperation.Dispose, name.Value, fault == ResourceFault.Dispose ? TraceOutcome.Throw : TraceOutcome.Normal));
        TraceRuntime.SharedValue = 99;
        if (fault == ResourceFault.Dispose) throw new ProbeException("dispose:" + name.Value);
    }
}
