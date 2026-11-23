namespace RevisionFixture;
public static class Stable {
    public static int Unchanged() => 42;
}

public static class OldNeighbor {
    public static int Rate() => 17;
}

public static class NewNeighbor {
    public static int Rate() => 23;
}
