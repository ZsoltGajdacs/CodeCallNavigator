package fixture;

import lombok.Builder;
import lombok.experimental.SuperBuilder;

public class Flow {
    public static void main(String[] args) { recordBuilder(); classBuilder(); childBuilder(); }
    public static Value.ValueBuilder recordBuilder() { leaf(); return Value.builder(); }
    public static Item.ItemBuilder classBuilder() { return Item.builder(); }
    public static Child.ChildBuilder<String, ?, ?> childBuilder() { return Child.<String>builder(); }
    public static Value.ValueBuilder[] arrays(Value.ValueBuilder[] builders) { return builders; }
    public static <B extends Item.ItemBuilder> B bounded(B builder) { return builder; }
    private static void leaf() {}
}

@Builder
record Value(String text) {}

@Builder
class Item { private String text; }

@SuperBuilder
class Parent<T> { private T value; }

@SuperBuilder
class Child<T> extends Parent<T> { private int count; }
