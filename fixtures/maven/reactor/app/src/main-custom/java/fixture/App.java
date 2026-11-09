package fixture;
public final class App {
    public static void main(String[] args) { Service.work(); GeneratedThing.generated(); }
    public void disconnected() {}
    public App() { constructorOnly(); }
    private void constructorOnly() {}
    void packageMethod() {}
    protected void protectedMethod() {}
    public String toString() { return "app"; }
    public String toString(int value) { return "overload"; }
    public int hashCode() { return 1; }
    public boolean equals(Object other) { return this == other; }
}
