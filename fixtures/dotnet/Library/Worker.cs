namespace Fixture;
public interface IWorker { void Work(); }
public class Worker : IWorker {
    public void Work() { Helper(); }
    public static void Helper() { }
    public static void Overload(int n) { }
    public static void Overload(string s) { }
    public static T Identity<T>(T value) => value;
    public static void Recurse(int n) { if(n > 0) Recurse(n - 1); }
    public static void Disconnected() { Helper(); }
}
public class OtherWorker : IWorker { public void Work() { Worker.Helper(); } }
