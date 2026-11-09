package codegraph.bytecode;

import java.io.BufferedInputStream;
import java.io.BufferedOutputStream;
import java.io.DataInputStream;
import java.io.DataOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.LongAdder;

/** Job-local immutable artifact facts; context ordering remains the stager's responsibility. */
final class ArtifactInventory {
    private static final int VERSION = 1;
    private final Path root;
    private final LongAdder reads = new LongAdder();
    private final LongAdder hits = new LongAdder();
    private final LongAdder classes = new LongAdder();
    private final LongAdder bytesRead = new LongAdder();
    private final LongAdder retainedBytes = new LongAdder();
    private final LongAdder indexBytes = new LongAdder();
    private final Map<Path, Object> locks = new ConcurrentHashMap<>();

    ArtifactInventory(Path root) throws IOException {
        this.root = root;
        Files.createDirectories(root);
    }

    record Definition(String entry, String name, String hash, Path bytes) {}

    List<Definition> definitions(Path input, boolean project) throws IOException {
        Path cached = root.resolve(hash(input.toString().getBytes(StandardCharsets.UTF_8)));
        synchronized (locks.computeIfAbsent(cached, ignored -> new Object())) {
            return load(input, project, cached);
        }
    }

    private List<Definition> load(Path input, boolean project, Path cached) throws IOException {
        if (Files.isDirectory(cached)) {
            hits.increment();
        } else {
            create(input, project, cached);
            reads.increment();
        }
        List<Definition> definitions = read(cached);
        if (project) retainProjectBytes(input, definitions);
        return definitions;
    }

    private void create(Path input, boolean project, Path cached) throws IOException {
        Path temporary = Files.createTempDirectory(root, "building-");
        try {
            List<Definition> definitions = new ArrayList<>();
            ClassInventory.visit(input, (entry, name, bytes) -> {
                Path stored = temporary.resolve(definitions.size() + ".class");
                bytesRead.add(bytes.length);
                if (project) {
                    Files.write(stored, bytes);
                    retainedBytes.add(bytes.length);
                }
                definitions.add(new Definition(entry, name, hash(bytes), stored));
                classes.increment();
            });
            write(temporary, definitions);
            indexBytes.add(Files.size(temporary.resolve("entries.bin")));
            Files.move(temporary, cached, StandardCopyOption.ATOMIC_MOVE);
        } finally {
            removeTemporary(temporary);
        }
    }

    private static void write(Path directory, List<Definition> definitions) throws IOException {
        try (var output = new DataOutputStream(new BufferedOutputStream(Files.newOutputStream(directory.resolve("entries.bin"))))) {
            output.writeInt(VERSION);
            output.writeInt(definitions.size());
            for (Definition definition : definitions) {
                output.writeUTF(definition.entry());
                output.writeUTF(definition.name());
                output.writeUTF(definition.hash());
            }
        }
    }

    private static List<Definition> read(Path directory) throws IOException {
        try (var input = new DataInputStream(new BufferedInputStream(Files.newInputStream(directory.resolve("entries.bin"))))) {
            int count = readHeader(input);
            List<Definition> definitions = new ArrayList<>();
            for (int index = 0; index < count; index++) {
                Path bytes = directory.resolve(index + ".class");
                definitions.add(new Definition(input.readUTF(), input.readUTF(), input.readUTF(), bytes));
            }
            if (input.read() != -1) throw new IOException("Trailing artifact inventory data");
            return definitions;
        }
    }

    private void retainProjectBytes(Path input, List<Definition> definitions) throws IOException {
        List<Definition> missing = definitions.stream().filter(definition -> !Files.isRegularFile(definition.bytes())).toList();
        if (missing.isEmpty()) return;
        ClassInventory.withReader(input, reader -> {
            for (Definition definition : missing) {
                byte[] bytes = reader.read(definition.entry());
                if (!hash(bytes).equals(definition.hash())) {
                    throw new IOException("Artifact changed before project staging: " + input + "!/" + definition.entry());
                }
                retain(definition.bytes(), bytes);
                bytesRead.add(bytes.length);
                retainedBytes.add(bytes.length);
            }
        });
    }

    private static void retain(Path destination, byte[] bytes) throws IOException {
        Path temporary = Files.createTempFile(destination.getParent(), "retaining-", ".class");
        try {
            Files.write(temporary, bytes);
            Files.move(temporary, destination, StandardCopyOption.ATOMIC_MOVE);
        } finally {
            Files.deleteIfExists(temporary);
        }
    }

    private static int readHeader(DataInputStream input) throws IOException {
        if (input.readInt() != VERSION) throw new IOException("Unsupported artifact inventory");
        int count = input.readInt();
        if (count < 0) throw new IOException("Invalid artifact inventory length");
        return count;
    }

    void writeStatistics(Path output) throws IOException {
        Files.writeString(output, "artifact_reads=" + reads.sum() + "\nartifact_hits=" + hits.sum() + "\nclasses_indexed=" + classes.sum()
                + "\nartifact_bytes_read=" + bytesRead.sum() + "\nartifact_retained_bytes=" + retainedBytes.sum()
                + "\nartifact_index_bytes=" + indexBytes.sum() + "\n");
    }

    static String hash(byte[] bytes) {
        try {
            return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
        } catch (NoSuchAlgorithmException exception) {
            throw new IllegalStateException("JDK SHA-256 is unavailable", exception);
        }
    }

    private static void removeTemporary(Path directory) throws IOException {
        if (!Files.exists(directory)) return;
        try (var paths = Files.walk(directory)) {
            for (Path path : paths.sorted(Comparator.reverseOrder()).toList()) Files.deleteIfExists(path);
        }
    }
}
