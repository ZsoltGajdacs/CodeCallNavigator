public delegate void Callback();
public class Worker {
    public static int Counter = Seed();
    public int Value { get; } = 2;
    public static int Seed() => 1;
    public static void Work() { Counter++; }
    public static Worker operator +(Worker a, Worker b) => a;
}
public class Program {
    public static void Main() {
        Worker.Work();
        var first = new Worker();
        var second = first + new Worker();
        var value = second.Value;
        Callback callback = Worker.Work;
        callback();
        void Local() { Worker.Work(); }
        Local();
    }
}
