package fixture;
import java.util.concurrent.Future;
import java.util.concurrent.CompletableFuture;
public class Flow {
  public Flow() {}
  static Flow receiver() { return new Flow(); }
  static int arg() { return 1; }
  private int hidden(int x) { return x; }
  final int fixed() { return 2; }
  int run(Contract contract) { return receiver().hidden(arg()) + fixed() + contract.hit() + "x".length(); }
  Future<Integer> future() { return CompletableFuture.completedFuture(1); }
  void refs() { Runnable ref = this::fixed; Runnable lambda = () -> fixed(); new Flow(); ref.run(); }
}
interface Contract { int hit(); }
final class FinalOwner { int leaf() { return 1; } int run() { return leaf(); } }
record Data(int value) {}
interface Box<T> { T get(); }
class Concrete implements Box<String> { public String get() { return "value"; } }
