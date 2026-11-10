package codegraph.bytecode;

import java.util.List;

/** Path syntax is inspected only after compiler annotation binding, never for API resolution. */
final class SpringHttpPaths {
    private SpringHttpPaths() {}
    static String combine(String prefix, String path, List<String> restrictions) {
        if (prefix == null || path == null) return null;
        var combined = join(prefix, path);
        if (combined.contains("${") || combined.contains("#{")) {
            restrictions.add("spring.unresolved_path:" + combined);
            return null;
        }
        if (!supported(combined)) restrictions.add("spring.path_pattern:" + combined);
        return combined;
    }
    private static String join(String prefix, String path) {
        if (prefix.isEmpty()) return leading(path);
        if (path.isEmpty()) return leading(prefix);
        var left = leading(prefix);
        if (left.endsWith("/")) left = left.substring(0, left.length() - 1);
        return left + leading(path);
    }
    private static String leading(String path) {
        return path.startsWith("/") ? path : "/" + path;
    }
    private static boolean supported(String path) {
        return java.util.Arrays.stream(path.split("/", -1)).allMatch(SpringHttpPaths::segment);
    }
    private static boolean segment(String value) {
        if (value.matches("\\{[A-Za-z_][A-Za-z0-9_]*}")) return true;
        return value.chars().noneMatch(character -> "{}*?:[]".indexOf(character) >= 0);
    }
}
