namespace RevisionFixture;
public static class Stable {
    public static int Unchanged() => 42;
}

public static class OldNeighbor {
    public static int Rate() => 17 + DepthRates.Offset();
}

public static class NewNeighbor {
    public static int Rate() => 23 + DepthRates.Offset();
}

public static class DepthRates {
    public static int Offset() => 61;
}
