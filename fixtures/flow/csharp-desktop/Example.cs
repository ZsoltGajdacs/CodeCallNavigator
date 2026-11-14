namespace DesktopFlow;

// Cached source navigation row 000: café 🧭
// Cached source navigation row 001: café 🧭
// Cached source navigation row 002: café 🧭
// Cached source navigation row 003: café 🧭
// Cached source navigation row 004: café 🧭
// Cached source navigation row 005: café 🧭
// Cached source navigation row 006: café 🧭
// Cached source navigation row 007: café 🧭
// Cached source navigation row 008: café 🧭
// Cached source navigation row 009: café 🧭
// Cached source navigation row 010: café 🧭
// Cached source navigation row 011: café 🧭
// Cached source navigation row 012: café 🧭
// Cached source navigation row 013: café 🧭
// Cached source navigation row 014: café 🧭
// Cached source navigation row 015: café 🧭
// Cached source navigation row 016: café 🧭
// Cached source navigation row 017: café 🧭
// Cached source navigation row 018: café 🧭
// Cached source navigation row 019: café 🧭
// Cached source navigation row 020: café 🧭
// Cached source navigation row 021: café 🧭
// Cached source navigation row 022: café 🧭
// Cached source navigation row 023: café 🧭
// Cached source navigation row 024: café 🧭
// Cached source navigation row 025: café 🧭
// Cached source navigation row 026: café 🧭
// Cached source navigation row 027: café 🧭
// Cached source navigation row 028: café 🧭
// Cached source navigation row 029: café 🧭
// Cached source navigation row 030: café 🧭
// Cached source navigation row 031: café 🧭
// Cached source navigation row 032: café 🧭
// Cached source navigation row 033: café 🧭
// Cached source navigation row 034: café 🧭
// Cached source navigation row 035: café 🧭
// Cached source navigation row 036: café 🧭
// Cached source navigation row 037: café 🧭
// Cached source navigation row 038: café 🧭
// Cached source navigation row 039: café 🧭
// Cached source navigation row 040: café 🧭
// Cached source navigation row 041: café 🧭
// Cached source navigation row 042: café 🧭
// Cached source navigation row 043: café 🧭
// Cached source navigation row 044: café 🧭
// Cached source navigation row 045: café 🧭
// Cached source navigation row 046: café 🧭
// Cached source navigation row 047: café 🧭
// Cached source navigation row 048: café 🧭
// Cached source navigation row 049: café 🧭
// Cached source navigation row 050: café 🧭
// Cached source navigation row 051: café 🧭
// Cached source navigation row 052: café 🧭
// Cached source navigation row 053: café 🧭
// Cached source navigation row 054: café 🧭
// Cached source navigation row 055: café 🧭
// Cached source navigation row 056: café 🧭
// Cached source navigation row 057: café 🧭
// Cached source navigation row 058: café 🧭
// Cached source navigation row 059: café 🧭
// Cached source navigation row 060: café 🧭
// Cached source navigation row 061: café 🧭
// Cached source navigation row 062: café 🧭
// Cached source navigation row 063: café 🧭
// Cached source navigation row 064: café 🧭
// Cached source navigation row 065: café 🧭
// Cached source navigation row 066: café 🧭
// Cached source navigation row 067: café 🧭
// Cached source navigation row 068: café 🧭
// Cached source navigation row 069: café 🧭
// Cached source navigation row 070: café 🧭
// Cached source navigation row 071: café 🧭
// Cached source navigation row 072: café 🧭
// Cached source navigation row 073: café 🧭
// Cached source navigation row 074: café 🧭
// Cached source navigation row 075: café 🧭
// Cached source navigation row 076: café 🧭
// Cached source navigation row 077: café 🧭
// Cached source navigation row 078: café 🧭
// Cached source navigation row 079: café 🧭
// Cached source navigation row 080: café 🧭
// Cached source navigation row 081: café 🧭
// Cached source navigation row 082: café 🧭
// Cached source navigation row 083: café 🧭
// Cached source navigation row 084: café 🧭
// Cached source navigation row 085: café 🧭
// Cached source navigation row 086: café 🧭
// Cached source navigation row 087: café 🧭
// Cached source navigation row 088: café 🧭
// Cached source navigation row 089: café 🧭
// Cached source navigation row 090: café 🧭
// Cached source navigation row 091: café 🧭
// Cached source navigation row 092: café 🧭
// Cached source navigation row 093: café 🧭
// Cached source navigation row 094: café 🧭
// Cached source navigation row 095: café 🧭
// Cached source navigation row 096: café 🧭
// Cached source navigation row 097: café 🧭
// Cached source navigation row 098: café 🧭
// Cached source navigation row 099: café 🧭
public interface IWorker { int Work(); }

public readonly struct ScopeResource : IDisposable
{
    public void Dispose() { }
}

public static class Example
{
    public static int Run(bool flag, IWorker worker)
    {
        using ScopeResource resource = new ScopeResource();
        string note = "héllo 🧭";
        if (flag) return Leaf(7);
        worker.Work();
        return Leaf(note);
    }

    public static int Leaf(int value) => value + 1;
    public static int Leaf(string value) => value.Length;

    public static async Task<int> Suspended()
    {
        await Task.Yield();
        return Leaf(9);
    }
}

public abstract class MissingBody
{
    public abstract int Absent();
}

public sealed class Worker : IWorker
{
    public int Work() => 3;
}
