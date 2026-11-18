package codecallnavigator.maven;

import java.io.IOException;
import java.nio.file.DirectoryStream;
import java.nio.file.Files;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.nio.file.attribute.BasicFileAttributes;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Collection;
import java.util.HexFormat;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

/** JDK-only compiler input observation shared by Maven and Gradle. */
final class CompileSnapshot {
    private final Map<String, Map<String, Object>> inputs = new TreeMap<>();
    private final Map<String, Map<String, Object>> scopes = new TreeMap<>();
    private final RequiredLinks links = new RequiredLinks();

    static CompileSnapshot capture(Collection<String> sources, Collection<String> classpath,
            Collection<String> javaExclusions, Collection<String> classExclusions) throws IOException {
        CompileSnapshot snapshot = new CompileSnapshot();
        for (String source : sources) {
            snapshot.scope(Path.of(source), "java", javaExclusions);
        }
        for (String entry : classpath) {
            Path path = absolute(Path.of(entry));
            if (excluded(path, classExclusions)) {
                continue;
            }
            if (leafClasspath(path)) {
                snapshot.input(path);
            } else {
                snapshot.scope(path, "class", classExclusions);
            }
        }
        return snapshot;
    }

    Set<String> javaMembers() {
        Set<String> result = new TreeSet<>();
        for (Map<String, Object> scope : scopes.values()) {
            if ("java".equals(scope.get("selection"))) {
                for (Object member : (Collection<?>) scope.get("members")) {
                    result.add((String) member);
                }
            }
        }
        return result;
    }

    Map<String, Object> document() {
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("scopes", new ArrayList<>(scopes.values()));
        result.put("inputs", new ArrayList<>(inputs.values()));
        result.put("links", links.document());
        return result;
    }

    Difference difference(CompileSnapshot current) {
        Difference changed = links.difference(current.links);
        return changed == null ? compareContents(current) : changed;
    }

    Difference differenceConsumed(CompileSnapshot current) {
        // Empty directories traversed only because AP output ownership changed
        // are not consumed inputs. Select link relevance from frozen paths/maps;
        // never resolve a historical path against the current filesystem.
        Difference changed = links.differenceConsumed(current.links,
                inputs.keySet().stream().map(Path::of).toList(),
                current.inputs.keySet().stream().map(Path::of).toList());
        return changed == null ? compareContents(current) : changed;
    }

    private Difference compareContents(CompileSnapshot current) {
        Difference changed = compareInputs(current);
        if (changed != null) {
            return changed;
        }
        for (String key : union(scopes.keySet(), current.scopes.keySet())) {
            Map<String, Object> before = scopes.get(key);
            Map<String, Object> after = current.scopes.get(key);
            String path = (String) (before == null ? after : before).get("path");
            if (!java.util.Objects.equals(before, after)) {
                return new Difference(path, "membership_changed");
            }
        }
        return null;
    }

    private Difference compareInputs(CompileSnapshot current) {
        for (String path : union(inputs.keySet(), current.inputs.keySet())) {
            if (!java.util.Objects.equals(inputs.get(path), current.inputs.get(path))) {
                Map<String, Object> before = inputs.get(path);
                Map<String, Object> after = current.inputs.get(path);
                String change = before == null || after == null
                        || !java.util.Objects.equals(before.get("kind"), after.get("kind"))
                        ? "kind_changed" : "content_changed";
                return new Difference(path, change);
            }
        }
        return null;
    }

    private void scope(Path raw, String selection, Collection<String> excluded) throws IOException {
        Path root = absolute(raw);
        if (excluded(root, excluded)) {
            return;
        }
        input(root);
        Set<String> members = new TreeSet<>();
        new ScopeWalk(selection, excluded, members).walk(root);
        Map<String, Object> record = new LinkedHashMap<>();
        record.put("path", root.toString());
        record.put("selection", selection);
        record.put("members", new ArrayList<>(members));
        scopes.put(root + "\0" + selection, record);
    }

    private final class ScopeWalk {
        private final String selection;
        private final Collection<String> excluded;
        private final Set<String> members;
        private final Set<String> ancestors = new TreeSet<>();

        ScopeWalk(String selection, Collection<String> excluded, Set<String> members) {
            this.selection = selection;
            this.excluded = excluded;
            this.members = members;
        }

        void walk(Path path) throws IOException {
            if (excluded(path, excluded)) {
                return;
            }
            BasicFileAttributes attributes = attributes(path);
            if (attributes == null) {
                missingSelection(path);
                return;
            }
            if (attributes.isDirectory()) {
                directory(path);
            } else {
                file(path, attributes);
            }
        }

        private void directory(Path path) throws IOException {
            links.observe(path);
            String physical = path.toRealPath().toString();
            if (!ancestors.add(physical)) {
                throw new IOException("Cyclic compiler input directory: " + path);
            }
            try (DirectoryStream<Path> children = Files.newDirectoryStream(path)) {
                for (Path child : children) {
                    walk(child);
                }
            }
            ancestors.remove(physical);
        }

        private boolean selected(Path path) {
            return path.toString().endsWith("." + selection);
        }

        private void missingSelection(Path path) throws IOException {
            if (selected(path)) {
                // Selected broken links still fail required-link observation;
                // unrelated missing/non-Java leaves are not compiler inputs.
                input(path);
            }
        }

        private void file(Path path, BasicFileAttributes attributes) throws IOException {
            if (!selected(path)) {
                return;
            }
            if (!attributes.isRegularFile()) {
                throw new IOException("Unsupported nonregular compiler input: " + path);
            }
            members.add(path.toString());
            input(path);
        }
    }

    private void input(Path raw) throws IOException {
        Path path = absolute(raw);
        links.observe(path);
        BasicFileAttributes attributes = attributes(path);
        Map<String, Object> record = new LinkedHashMap<>();
        record.put("path", path.toString());
        if (attributes == null) {
            record.put("kind", "missing");
        } else if (attributes.isDirectory()) {
            record.put("kind", "directory");
        } else if (attributes.isRegularFile()) {
            record.put("kind", "file");
            record.put("sha256", digest(path));
        } else {
            throw new IOException("Unsupported nonregular compiler input: " + path);
        }
        inputs.put(path.toString(), record);
    }

    private static boolean leafClasspath(Path path) {
        if (Files.isRegularFile(path)) {
            return true;
        }
        return path.toString().endsWith(".jar") || path.toString().endsWith(".zip");
    }

    private static BasicFileAttributes attributes(Path path) throws IOException {
        try {
            return Files.readAttributes(path, BasicFileAttributes.class);
        } catch (NoSuchFileException missing) {
            return null;
        }
    }

    private static String digest(Path path) throws IOException {
        try {
            MessageDigest hash = MessageDigest.getInstance("SHA-256");
            try (var input = Files.newInputStream(path)) {
                byte[] buffer = new byte[65536];
                int count;
                while ((count = input.read(buffer)) != -1) {
                    hash.update(buffer, 0, count);
                }
            }
            return HexFormat.of().formatHex(hash.digest());
        } catch (NoSuchAlgorithmException impossible) {
            throw new IllegalStateException("JDK SHA-256 is unavailable", impossible);
        }
    }

    static boolean excluded(Path path, Collection<String> exclusions) throws IOException {
        Path lexical = absolute(path);
        Path physical = lexical.toFile().getCanonicalFile().toPath();
        for (String exclusion : exclusions) {
            Path output = absolute(Path.of(exclusion));
            if (lexical.startsWith(output) || physical.startsWith(output.toFile().getCanonicalFile().toPath())) {
                return true;
            }
        }
        return false;
    }

    private static Set<String> union(Set<String> first, Set<String> second) {
        Set<String> result = new TreeSet<>(first);
        result.addAll(second);
        return result;
    }

    private static Path absolute(Path path) {
        return path.toAbsolutePath();
    }

    record Difference(String path, String change) {}
}
