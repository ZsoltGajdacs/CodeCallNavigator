package codecallnavigator.maven;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.Collection;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

/** Captures lexical and resolved target-ancestor bindings without normalizing link/.. first. */
final class RequiredLinks {
    private final Map<String, String> bindings = new TreeMap<>();

    void observe(Path path) throws IOException {
        ArrayDeque<Path> pending = new ArrayDeque<>();
        Set<String> visited = new TreeSet<>();
        pending.add(path);
        while (!pending.isEmpty()) {
            Path required = pending.removeFirst();
            if (visited.add(required.toString())) {
                observeAncestors(required, pending);
            }
        }
    }

    private void observeAncestors(Path path, ArrayDeque<Path> pending) throws IOException {
        Path ancestor = path.getRoot();
        for (Path component : path) {
            ancestor = ancestor.resolve(component);
            if (Files.isSymbolicLink(ancestor)) {
                Path target = Files.readSymbolicLink(ancestor);
                bindings.put(ancestor.toString(), target.toString());
                ancestor.toRealPath();
                pending.add(destination(ancestor, target));
            }
        }
    }

    List<Map<String, String>> document() {
        return bindings.entrySet().stream()
                .map(entry -> Map.of("path", entry.getKey(), "target", entry.getValue())).toList();
    }

    CompileSnapshot.Difference difference(RequiredLinks current) {
        return compare(bindings, current.bindings);
    }

    CompileSnapshot.Difference differenceConsumed(RequiredLinks current, Collection<Path> beforeInputs,
            Collection<Path> afterInputs) {
        return compare(project(beforeInputs), current.project(afterInputs));
    }

    private static CompileSnapshot.Difference compare(Map<String, String> before, Map<String, String> after) {
        Set<String> paths = new TreeSet<>(before.keySet());
        paths.addAll(after.keySet());
        for (String path : paths) {
            if (!java.util.Objects.equals(before.get(path), after.get(path))) {
                return new CompileSnapshot.Difference(path, "link_changed");
            }
        }
        return null;
    }

    private Map<String, String> project(Collection<Path> inputs) {
        Map<String, String> result = new TreeMap<>();
        ArrayDeque<Path> pending = new ArrayDeque<>(inputs);
        Set<String> visited = new TreeSet<>();
        while (!pending.isEmpty()) {
            Path path = pending.removeFirst();
            if (visited.add(path.toString())) {
                selectAncestors(path, result, pending);
            }
        }
        return result;
    }

    private void selectAncestors(Path path, Map<String, String> result, ArrayDeque<Path> pending) {
        Path ancestor = path.getRoot();
        for (Path component : path) {
            ancestor = ancestor.resolve(component);
            String target = bindings.get(ancestor.toString());
            if (target != null) {
                result.put(ancestor.toString(), target);
                pending.add(destination(ancestor, Path.of(target)));
            }
        }
    }

    private static Path destination(Path link, Path target) {
        return target.isAbsolute() ? target : link.getParent().resolve(target);
    }
}
