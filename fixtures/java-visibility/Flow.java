package fixture;

public class Flow {
    public void publicMethod() {}
    private void privateMethod() {}
    protected void protectedMethod() {}
    void packageMethod() {}

    public static class PublicNested { private void member() {} }
    private static class PrivateNested { public void member() {} }
    protected static class ProtectedNested { public void member() {} }
    static class PackageNested { public void member() {} }
    interface Contract { void implicitPublic(); }
    public void localDeclarations() {
        class Local { public void localMember() {} }
        new Local().localMember();
        Runnable action = () -> privateMethod();
        action.run();
        new Runnable() { public void run() { privateMethod(); } }.run();
    }
}
class PackageType { public void member() {} }
