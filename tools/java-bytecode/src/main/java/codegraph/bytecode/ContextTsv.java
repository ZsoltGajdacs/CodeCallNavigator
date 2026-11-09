package codegraph.bytecode;

import java.io.BufferedWriter;
import java.io.IOException;
import java.util.Arrays;
import java.util.stream.Collectors;

/** One escaping contract for staging manifests and original bytecode metadata. */
final class ContextTsv {
    private ContextTsv() {}

    static void write(BufferedWriter writer, Object... values) throws IOException {
        writer.write(Arrays.stream(values).map(Object::toString).map(ContextTsv::escape)
                .collect(Collectors.joining("\t")));
        writer.newLine();
    }

    private static String escape(String field) {
        return field.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r");
    }
}
