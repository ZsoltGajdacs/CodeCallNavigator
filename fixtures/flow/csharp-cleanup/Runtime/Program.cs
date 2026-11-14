using CleanupFixture;

var cases = new Dictionary<string, Func<int>>
{
    ["using-scope"] = CleanupCases.UseScope,
    ["acquisition-failure"] = CleanupCases.AcquisitionFailure,
    ["dispose-throws-outer"] = CleanupCases.DisposeThrowsOuter,
    ["acquisition-failure-caught"] = CleanupCases.AcquisitionFailureCaught,
    ["nested-using"] = CleanupCases.NestedUsing,
    ["return-before-dispose"] = CleanupCases.ReturnBeforeDispose,
    ["capture-mutation"] = CleanupCases.CaptureMutation,
    ["null-reference"] = CleanupCases.NullReference,
    ["null-nullable"] = CleanupCases.NullNullable,
    ["loop-transfers"] = CleanupCases.LoopTransfers,
    ["rethrow"] = ExceptionCases.Rethrow,
    ["finally-throws"] = ExceptionCases.FinallyThrows,
    ["foreach-normal"] = () => ForeachCases.Run(EnumerationFault.None),
    ["foreach-break"] = () => ForeachCases.Break(EnumerationFault.None),
    ["foreach-continue"] = ForeachCases.Continue,
    ["foreach-acquire-failure"] = () => ForeachCases.Run(EnumerationFault.Acquire),
    ["foreach-move-failure"] = () => ForeachCases.Run(EnumerationFault.MoveNext),
    ["foreach-current-failure"] = () => ForeachCases.Run(EnumerationFault.Current),
    ["foreach-dispose-failure"] = () => ForeachCases.Break(EnumerationFault.Dispose),
    ["foreach-body-failure"] = ForeachCases.BodyFailure,
    ["lock-normal"] = LockCases.Normal,
    ["lock-throwing"] = LockCases.Throwing,
    ["filter-accepted"] = () => BoundaryCases.Filtered(true),
    ["filter-rejected"] = () => BoundaryCases.Filtered(false),
    ["async-body"] = () => BoundaryCases.AsyncBody().GetAwaiter().GetResult(),
    ["await-using"] = () => BoundaryCases.AwaitUsing().GetAwaiter().GetResult(),
    ["await-foreach"] = () => BoundaryCases.AwaitForeach().GetAwaiter().GetResult(),
    ["iterator"] = () => BoundaryCases.Iterator().Count(),
    ["guarded-switch"] = BoundaryCases.GuardedSwitch,
    ["goto"] = BoundaryCases.Goto,
    ["resource-budget"] = BudgetCase.ShallowResources,
};
var scenario = args.Length == 0 ? "using-scope" : args[0];
int? result = null;
string? error = null;
try { result = cases[scenario](); }
catch (Exception caught) { error = caught.GetType().Name + ":" + caught.Message; }
var lockProbe = scenario.StartsWith("lock-", StringComparison.Ordinal) ? LockCases.ReleaseProbe() : null;
TraceRuntime.Report(new(scenario, result, error, lockProbe));
