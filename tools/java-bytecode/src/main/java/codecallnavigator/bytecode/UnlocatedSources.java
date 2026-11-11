package codecallnavigator.bytecode;

import com.sun.source.tree.ClassTree;
import com.sun.source.tree.CompilationUnitTree;
import com.sun.source.util.JavacTask;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.Map;
import java.util.Set;
import java.util.TreeSet;
import javax.tools.ToolProvider;

/** Resolves missing SourceFile attributes by parsed top-level declarations, never filename guesses. */
final class UnlocatedSources {
    private UnlocatedSources() {}

    static Set<Path> resolve(Set<String> names, String[] roots) throws IOException {
        if (names.isEmpty()) return Set.of();
        Set<Path> sources = candidates(roots);
        if (sources.isEmpty()) return Set.of();
        var compiler = ToolProvider.getSystemJavaCompiler();
        try (var manager = compiler.getStandardFileManager(null, null, null)) {
            var diagnostics = new javax.tools.DiagnosticCollector<javax.tools.JavaFileObject>();
            var task = (JavacTask) compiler.getTask(null, manager, diagnostics, java.util.List.of("-proc:none", "-implicit:none"), null,
                    manager.getJavaFileObjectsFromPaths(sources));
            Map<String, SourceOwner> owners = new HashMap<>();
            for (CompilationUnitTree unit : task.parse()) collectOwners(unit, names, owners);
            return owners.values().stream().map(SourceOwner::source).collect(java.util.stream.Collectors.toCollection(TreeSet::new));
        }
    }

    private static Set<Path> candidates(String[] roots) throws IOException {
        Set<Path> candidates = new TreeSet<>();
        for (String root : roots) {
            if (!Files.isDirectory(Path.of(root))) continue;
            try (var files = Files.walk(Path.of(root))) {
                for (Path path : files.filter(path -> path.toString().endsWith(".java")).toList()) candidates.add(path.toRealPath());
            }
        }
        return candidates;
    }

    private static void collectOwners(CompilationUnitTree unit, Set<String> names, Map<String, SourceOwner> owners) throws IOException {
        String namespace = unit.getPackageName() == null ? "" : unit.getPackageName() + ".";
        Path source = Path.of(unit.getSourceFile().toUri()).toRealPath();
        for (var declaration : unit.getTypeDecls()) {
            if (declaration instanceof ClassTree type) matchOwner(namespace + type.getSimpleName(), source, names, owners);
        }
    }

    private static void matchOwner(String owner, Path source, Set<String> names, Map<String, SourceOwner> owners) throws IOException {
        for (String name : names) {
            if (!name.equals(owner) && !name.startsWith(owner + "$")) continue;
            registerOwner(name, new SourceOwner(owner, source), owners);
        }
    }

    private record SourceOwner(String name, Path source) {}

    private static void registerOwner(String binaryName, SourceOwner candidate, Map<String, SourceOwner> owners) throws IOException {
        SourceOwner previous = owners.get(binaryName);
        if (previous == null) {
            owners.put(binaryName, candidate);
            return;
        }
        // Longest top-level owner also handles nested classes within legal dollar-containing names.
        int specificity = Integer.compare(candidate.name().length(), previous.name().length());
        if (specificity > 0) owners.put(binaryName, candidate);
        if (specificity != 0) return;
        if (!previous.source().equals(candidate.source())) throw new IOException("Ambiguous source declaration for " + binaryName);
    }

}
