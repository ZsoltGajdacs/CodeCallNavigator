using Fixture;
public class Program {
    public static void Main() { IWorker w = new Worker(); w.Work(); Worker.Overload(1); Worker.Overload("s"); Worker.Identity(1); Worker.Recurse(2); System.Action callback = Worker.Helper; callback(); }
}
