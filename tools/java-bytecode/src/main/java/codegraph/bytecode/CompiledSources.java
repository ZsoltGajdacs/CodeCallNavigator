package codegraph.bytecode;

import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Set;
import java.util.Map;
import java.util.TreeMap;
import java.util.TreeSet;
import org.apache.bcel.classfile.ClassParser;

/** Selects source files represented in successful compiler outputs, excluding build-excluded files. */
final class CompiledSources {
    private CompiledSources() {}

    static TreeSet<Path> collect(String classDirectories, String[] roots) throws IOException {
        Set<String> unlocated = new TreeSet<>();
        var relatives = relativeFiles(classDirectories, unlocated);
        var sources = new TreeSet<Path>();
        for (var entry : relatives.entrySet()) {
            Set<Path> located = locate(entry.getKey(), roots);
            if (located.isEmpty()) unlocated.addAll(entry.getValue());
            sources.addAll(located);
        }
        sources.addAll(UnlocatedSources.resolve(unlocated, roots));
        return sources;
    }

    private static Set<Path> locate(Path relative, String[] roots) throws IOException {
        Set<Path> located = new TreeSet<>();
        for (String root : roots) {
            Path source = Path.of(root).resolve(relative);
            if (Files.isRegularFile(source)) located.add(source.toRealPath());
        }
        return located;
    }

    private static Map<Path, Set<String>> relativeFiles(String classDirectories, Set<String> unlocated) throws IOException {
        Map<Path, Set<String>> relatives = new TreeMap<>();
        for (String directory : classDirectories.split(java.util.regex.Pattern.quote(File.pathSeparator))) {
            Path root = Path.of(directory);
            if (Files.isDirectory(root)) ClassInventory.visit(root, (entry, name, bytes) -> {
                var type = new ClassParser(new ByteArrayInputStream(bytes), entry).parse();
                addSources(relatives, unlocated, name, type.getSourceFileName());
            });
        }
        return relatives;
    }

    private static void addSources(Map<Path, Set<String>> relatives, Set<String> unlocated, String binaryName, String sourceFile) {
        if (binaryName.equals("package-info") || binaryName.endsWith(".package-info")) return;
        if (sourceFile.equals("<Unknown>")) {
            unlocated.add(binaryName);
            return;
        }
        int separator = binaryName.lastIndexOf('.');
        Path namespace = Path.of(binaryName.substring(0, separator + 1).replace('.', '/'));
        if (sourceFile.endsWith(".java")) relatives.computeIfAbsent(namespace.resolve(sourceFile), ignored -> new TreeSet<>()).add(binaryName);
    }
}
