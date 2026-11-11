package codecallnavigator.bytecode;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import java.util.Set;
import java.util.TreeSet;
import java.util.stream.Collectors;

/** Restricts body analysis to the owning source set while retaining project dependency declarations. */
final class ProjectMethodFilter {
    private ProjectMethodFilter() {}

    static boolean expectsMethods(Set<Integer> projectInputs, Set<Integer> inputsWithMethods) {
        if (projectInputs == null) {
            return true;
        }
        return !java.util.Collections.disjoint(projectInputs, inputsWithMethods);
    }

    static String expression(List<String> inputs, Set<Integer> projectInputs) throws IOException {
        if (projectInputs == null) {
            return "false";
        }
        Set<String> names = projectClassNames(inputs, projectInputs);
        if (names.isEmpty()) {
            throw new IOException("Project inputs contain no class definitions");
        }
        return "!(" + matches(names) + ")";
    }

    private static String matches(Collection<String> names) {
        List<String> ordered = new ArrayList<>(new TreeSet<>(names));
        List<String> groups = new ArrayList<>();
        for (int start = 0; start < ordered.size(); start += 100) {
            String arguments = ordered.subList(start, Math.min(start + 100, ordered.size())).stream()
                    .map(ProjectMethodFilter::quote).collect(Collectors.joining(", "));
            groups.add("string.equalsAny(class_name, " + arguments + ")");
        }
        return String.join(" || ", groups);
    }

    private static Set<String> projectClassNames(List<String> inputs, Set<Integer> projectInputs) throws IOException {
        Set<String> names = new TreeSet<>();
        for (int index : projectInputs) {
            Path root = Path.of(inputs.get(index - 1));
            if (!Files.isDirectory(root)) {
                ClassInventory.visit(root, (entry, name, bytes) -> names.add(name));
                continue;
            }
            try (var files = Files.walk(root)) {
                files.filter(Files::isRegularFile).map(root::relativize).map(Path::toString)
                        .filter(name -> name.endsWith(".class"))
                        .map(name -> name.substring(0, name.length() - 6).replace('/', '.'))
                        .forEach(names::add);
            }
        }
        return names;
    }

    private static String quote(String name) {
        return "'" + name.replace("\\", "\\\\").replace("'", "\\'") + "'";
    }
}
