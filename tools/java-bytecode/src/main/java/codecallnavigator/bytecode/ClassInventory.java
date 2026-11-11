package codecallnavigator.bytecode;

import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.jar.JarFile;
import java.util.zip.ZipFile;
import org.apache.bcel.classfile.ClassParser;

/** Reads class definitions; visibility and duplicate policy belong to ContextStager. */
final class ClassInventory {
    private ClassInventory() {}

    @FunctionalInterface
    interface Visitor {
        void accept(String entry, String name, byte[] bytes) throws IOException;
    }

    static void visit(Path input, Visitor visitor) throws IOException {
        if (Files.isDirectory(input)) {
            directory(input, visitor);
        } else {
            archive(input, visitor);
        }
    }

    @FunctionalInterface
    interface EntryReader {
        byte[] read(String entry) throws IOException;
    }

    @FunctionalInterface
    interface ReadAction {
        void accept(EntryReader reader) throws IOException;
    }

    static void withReader(Path input, ReadAction action) throws IOException {
        if (Files.isDirectory(input)) {
            action.accept(entry -> directoryBytes(input, entry));
        } else {
            try (var jar = new JarFile(input.toFile(), false, ZipFile.OPEN_READ, Runtime.Version.parse("25"))) {
                action.accept(entry -> archiveBytes(jar, entry));
            }
        }
    }

    private static byte[] directoryBytes(Path input, String entry) throws IOException {
        Path file = input.resolve(entry).normalize();
        if (!file.startsWith(input)) throw new IOException("Invalid class entry: " + entry);
        return Files.readAllBytes(file);
    }

    private static byte[] archiveBytes(JarFile jar, String entry) throws IOException {
        var selected = jar.getJarEntry(entry);
        if (selected == null) throw new IOException("Class entry disappeared: " + entry);
        try (var stream = jar.getInputStream(selected)) {
            return stream.readAllBytes();
        }
    }

    private static void directory(Path input, Visitor visitor) throws IOException {
        try (var files = Files.walk(input)) {
            for (Path file : files.filter(Files::isRegularFile)
                    .filter(path -> classFile(input.relativize(path).toString())).sorted().toList()) {
                visitClass(input.relativize(file).toString(), Files.readAllBytes(file), visitor);
            }
        }
    }

    private static void archive(Path input, Visitor visitor) throws IOException {
        // versionedStream follows the jar manifest and Java 25 multi-release selection.
        try (var jar = new JarFile(input.toFile(), false, ZipFile.OPEN_READ, Runtime.Version.parse("25"))) {
            for (var entry : jar.versionedStream().filter(item -> classFile(item.getName())).sorted(
                    java.util.Comparator.comparing(java.util.jar.JarEntry::getName)).toList()) {
                try (var stream = jar.getInputStream(entry)) {
                    visitClass(entry.getRealName(), stream.readAllBytes(), visitor);
                }
            }
        }
    }

    private static boolean classFile(String path) {
        return path.endsWith(".class") && !path.endsWith("module-info.class")
                && !path.startsWith("META-INF/versions/");
    }

    private static void visitClass(String entry, byte[] bytes, Visitor visitor) throws IOException {
        String name = new ClassParser(new ByteArrayInputStream(bytes), entry).parse().getClassName();
        visitor.accept(entry, name, bytes);
    }
}
