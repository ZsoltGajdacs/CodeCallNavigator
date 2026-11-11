package codecallnavigator.bytecode;

import java.util.Arrays;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

/** Shared validation for one-based project input selections in both helper entrypoints. */
final class InputSelection {
    private InputSelection() {}

    static Set<Integer> parse(String value, int inputCount) {
        Set<Integer> indices = Arrays.stream(value.split(",", -1)).map(Integer::valueOf).collect(Collectors.toSet());
        Set<Integer> valid = IntStream.rangeClosed(1, inputCount).boxed().collect(Collectors.toSet());
        if (!valid.containsAll(indices)) {
            throw new IllegalArgumentException("Project input indices must select existing one-based inputs");
        }
        return indices;
    }
}
