package codegraph.maven;

import java.util.Collection;
import java.util.Map;
import java.util.stream.Collectors;

/** Encodes the small metadata contract without introducing runtime dependencies into Maven. */
final class Json {
    private Json() {}

    static String write(Object value) {
        if (value instanceof String string) {
            return quote(string);
        }
        if (value instanceof Boolean flag) {
            return flag.toString();
        }
        return container(value);
    }

    private static String container(Object value) {
        if (value instanceof Map<?, ?> map) {
            return object(map);
        }
        if (value instanceof Collection<?> collection) {
            return array(collection);
        }
        throw new IllegalArgumentException("Unsupported metadata value: " + value);
    }

    private static String object(Map<?, ?> map) {
        return map.entrySet().stream()
                .map(entry -> quote((String) entry.getKey()) + ":" + write(entry.getValue()))
                .collect(Collectors.joining(",", "{", "}"));
    }

    private static String array(Collection<?> collection) {
        return collection.stream().map(Json::write).collect(Collectors.joining(",", "[", "]"));
    }

    private static String quote(String value) {
        StringBuilder result = new StringBuilder("\"");
        for (int index = 0; index < value.length(); index++) {
            char character = value.charAt(index);
            switch (character) {
                case '"' -> result.append("\\\"");
                case '\\' -> result.append("\\\\");
                case '\n' -> result.append("\\n");
                case '\r' -> result.append("\\r");
                case '\t' -> result.append("\\t");
                default -> {
                    if (character < 0x20) {
                        result.append(String.format("\\u%04x", (int) character));
                    } else {
                        result.append(character);
                    }
                }
            }
        }
        return result.append('"').toString();
    }
}
