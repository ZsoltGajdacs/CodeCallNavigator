package fixture;
interface Work { void run(); }
class First implements Work { public void run() { Flow.helper(); } }
class Second implements Work { public void run() { Flow.helper(); } }
public class Flow {
    public static void main(String[] args) { Work w = new First(); w.run(); overloaded(1); overloaded("s"); identity(1); recurse(2); Runnable r = Flow::helper; r.run(); }
    static void helper() {}
    static void overloaded(int i) {}
    static void overloaded(String s) {}
    static <T> T identity(T value) { return value; }
    static void recurse(int n) { if (n > 0) recurse(n - 1); }
    static void disconnected() { helper(); }
}
