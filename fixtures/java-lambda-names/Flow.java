package fixture;
import java.util.function.Function;
import java.util.function.Supplier;

public class Flow {
    public static final Function<String, String> COST_TYPE_VALIDATION =
        row -> {
            return leaf(row);
        };
    public final Runnable instanceAction = () -> {
        leaf("instance");
    };
    public void validate() {
        Runnable localAction = () -> {
            leaf("local");
        };
        consume(() -> {
            leaf("inline");
        });
        Supplier<Runnable> nested = () -> {
            return () -> {
                leaf("nested");
            };
        };
        Runnable first = () -> leaf("first"); Runnable second = () -> leaf("second");
        localAction.run();
        nested.get().run();
        first.run(); second.run();
    }
    public static void main(String[] args) {
        COST_TYPE_VALIDATION.apply("row");
        new Flow().validate();
    }
    public void lambda$manual$99() {}
    private static String leaf(String value) { return value; }
    private static void consume(Runnable action) { action.run(); }
}
