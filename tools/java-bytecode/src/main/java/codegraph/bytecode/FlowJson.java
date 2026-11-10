package codegraph.bytecode;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;

/** Small detached JSON values; no analyzer dependency is needed to encode source IR. */
final class FlowJson {
    private FlowJson() {}
    static Map<String, Object> object(Object... pairs) {
        Map<String, Object> result = new LinkedHashMap<>();
        for (int index = 0; index < pairs.length; index += 2) result.put((String) pairs[index], pairs[index + 1]);
        return result;
    }
    static String encode(Object value) {
        if (value == null) return "null";
        if (value instanceof String text) return quote(text);
        if (value instanceof Map<?, ?> map) return map.entrySet().stream()
                .map(entry -> quote(entry.getKey().toString()) + ":" + encode(entry.getValue())).collect(Collectors.joining(",", "{", "}"));
        if (value instanceof List<?> list) return list.stream().map(FlowJson::encode).collect(Collectors.joining(",", "[", "]"));
        return value.toString();
    }
    private static String quote(String text) {
        var result = new StringBuilder("\"");
        for (int index = 0; index < text.length(); index++) {
            char character = text.charAt(index);
            switch (character) {
                case '"' -> result.append("\\\"");
                case '\\' -> result.append("\\\\");
                case '\n' -> result.append("\\n");
                case '\r' -> result.append("\\r");
                case '\t' -> result.append("\\t");
                default -> {
                    if (character < 0x20) result.append(String.format("\\u%04x", (int) character));
                    else result.append(character);
                }
            }
        }
        return result.append('"').toString();
    }
}
