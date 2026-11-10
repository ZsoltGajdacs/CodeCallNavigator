package fixture;
public final class Standalone {
    static final StringBuilder events = new StringBuilder();
    static void mark(String event) { events.append(event).append(","); }
    static int exercise(int mode) {
        outer: for (int index = 0; index < 2; index++) {
            try {
                mark("body");
                if (mode == 1) return 1;
                if (mode == 2) break outer;
                if (mode == 3) continue outer;
                if (mode == 4) throw new IllegalStateException();
            } catch (IllegalStateException error) {
                mark("catch");
                return 4;
            } finally {
                mark("finally");
            }
        }
        mark("after");
        return 0;
    }
    static final class Resource implements AutoCloseable {
        public void close() { mark("close"); }
    }
    static Resource acquire(int value) {
        mark("acquire");
        if (value < 0) throw new IllegalStateException();
        return new Resource();
    }
    static int resources(boolean fail) {
        try (Resource first = acquire(0); Resource second = acquire(fail ? -1 : 0)) {
            mark("work");
            return 1;
        } catch (IllegalStateException error) {
            mark("catch");
            return 2;
        } finally {
            mark("finally");
        }
    }
    static int overriding() {
        try {
            mark("try");
            return 1;
        } finally {
            mark("override");
            return 2;
        }
    }
    public static void main(String[] arguments) {
        int mode = Integer.parseInt(arguments[0]);
        if (mode < 5) exercise(mode);
        else if (mode < 7) resources(mode == 6);
        else overriding();
        System.out.println(events);
    }
}
