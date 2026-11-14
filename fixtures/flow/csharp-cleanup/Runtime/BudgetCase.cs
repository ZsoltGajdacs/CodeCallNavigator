namespace CleanupFixture;

public static class BudgetCase
{
    public static int ShallowResources()
    {
        using (ValueResource r0 = TraceRuntime.Acquire(new("budget-0")),
            r1 = TraceRuntime.Acquire(new("budget-1")),
            r2 = TraceRuntime.Acquire(new("budget-2")),
            r3 = TraceRuntime.Acquire(new("budget-3")),
            r4 = TraceRuntime.Acquire(new("budget-4")),
            r5 = TraceRuntime.Acquire(new("budget-5")),
            r6 = TraceRuntime.Acquire(new("budget-6")),
            r7 = TraceRuntime.Acquire(new("budget-7")),
            r8 = TraceRuntime.Acquire(new("budget-8")),
            r9 = TraceRuntime.Acquire(new("budget-9")),
            r10 = TraceRuntime.Acquire(new("budget-10")),
            r11 = TraceRuntime.Acquire(new("budget-11")),
            r12 = TraceRuntime.Acquire(new("budget-12")),
            r13 = TraceRuntime.Acquire(new("budget-13")),
            r14 = TraceRuntime.Acquire(new("budget-14")),
            r15 = TraceRuntime.Acquire(new("budget-15")),
            r16 = TraceRuntime.Acquire(new("budget-16")),
            r17 = TraceRuntime.Acquire(new("budget-17")),
            r18 = TraceRuntime.Acquire(new("budget-18")),
            r19 = TraceRuntime.Acquire(new("budget-19")),
            r20 = TraceRuntime.Acquire(new("budget-20")),
            r21 = TraceRuntime.Acquire(new("budget-21")),
            r22 = TraceRuntime.Acquire(new("budget-22")),
            r23 = TraceRuntime.Acquire(new("budget-23")),
            r24 = TraceRuntime.Acquire(new("budget-24")),
            r25 = TraceRuntime.Acquire(new("budget-25")),
            r26 = TraceRuntime.Acquire(new("budget-26")),
            r27 = TraceRuntime.Acquire(new("budget-27")),
            r28 = TraceRuntime.Acquire(new("budget-28")),
            r29 = TraceRuntime.Acquire(new("budget-29")),
            r30 = TraceRuntime.Acquire(new("budget-30")),
            r31 = TraceRuntime.Acquire(new("budget-31")),
            r32 = TraceRuntime.Acquire(new("budget-32")),
            r33 = TraceRuntime.Acquire(new("budget-33")),
            r34 = TraceRuntime.Acquire(new("budget-34")),
            r35 = TraceRuntime.Acquire(new("budget-35")),
            r36 = TraceRuntime.Acquire(new("budget-36")),
            r37 = TraceRuntime.Acquire(new("budget-37")),
            r38 = TraceRuntime.Acquire(new("budget-38")),
            r39 = TraceRuntime.Acquire(new("budget-39")))
            TraceRuntime.Mark(new("budget-body"));
        return 0;
    }
}
