package codegraph.bytecode;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.Map;
import java.util.Set;

/** Selects the first class definition in an already evaluated, ordered compile context. */
public final class ContextStager {
    private final Path output;
    private final Map<String, Selection> selected = new HashMap<>();
    private final Set<Integer> projectInputs;
    private int inputIndex;

    private ContextStager(Path output, Set<Integer> projectInputs) {
        this.output = output;
        this.projectInputs = projectInputs;
    }

    public static void main(String[] arguments) throws IOException {
        if (arguments.length < 2) {
            throw new IllegalArgumentException("Usage: ContextStager OUTPUT_ROOT CLASSPATH_ENTRY...");
        }
        Path output = Path.of(arguments[0]).toAbsolutePath().normalize();
        boolean filtered = arguments.length > 3 && arguments[1].equals("--project-inputs");
        int firstInput = filtered ? 3 : 1;
        Set<Integer> projectInputs = filtered ? InputSelection.parse(arguments[2], arguments.length - firstInput) : null;
        Files.createDirectories(output);
        new ContextStager(output, projectInputs).stage(arguments, firstInput);
    }

    private void stage(String[] arguments, int firstInput) throws IOException {
        try (var inputs = Files.newBufferedWriter(output.resolve("inputs.tsv"));
                var provenance = Files.newBufferedWriter(output.resolve("provenance.tsv"));
                var shadows = Files.newBufferedWriter(output.resolve("shadows.tsv"))) {
            ContextTsv.write(inputs, "original_path", "staged_path");
            ContextTsv.write(provenance, "input_index", "original_path", "class_name", "class_entry", "sha256");
            ContextTsv.write(shadows, "class_name", "selected_path", "shadowed_path", "identical");
            Manifests manifests = new Manifests(inputs, provenance, shadows);
            for (int index = firstInput; index < arguments.length; index++) {
                boolean projectInput = projectInputs == null || projectInputs.contains(index - firstInput + 1);
                stageInput(Path.of(arguments[index]).toAbsolutePath().normalize(), projectInput, manifests);
            }
        }
    }

    private void stageInput(Path original, boolean projectInput, Manifests manifests) throws IOException {
        int index = inputIndex + 1;
        Path staged = output.resolve("input-" + index);
        ClassInventory.visit(original, (entry, name, bytes) -> select(
                new Candidate(original, staged, index, entry, name, bytes, projectInput), manifests));
        if (Files.isDirectory(staged)) {
            ContextTsv.write(manifests.inputs(), original, staged);
            inputIndex = index;
        }
    }

    private void select(Candidate candidate, Manifests manifests) throws IOException {
        String hash = sha256(candidate.bytes());
        Selection previous = selected.putIfAbsent(candidate.name(), new Selection(candidate.original(), hash));
        if (previous != null) {
            ContextTsv.write(manifests.shadows(), candidate.name(), previous.original(), candidate.original(), previous.hash().equals(hash));
            return;
        }
        if (!candidate.projectInput()) {
            return;
        }
        Path target = candidate.staged().resolve(candidate.name().replace('.', '/') + ".class").normalize();
        if (!target.startsWith(candidate.staged())) {
            throw new IOException("Invalid class name: " + candidate.name());
        }
        Files.createDirectories(target.getParent());
        Files.write(target, candidate.bytes());
        ContextTsv.write(manifests.provenance(), candidate.index(), candidate.original(), candidate.name(), candidate.entry(), hash);
    }

    private static String sha256(byte[] bytes) {
        try {
            return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
        } catch (NoSuchAlgorithmException exception) {
            throw new IllegalStateException("JDK SHA-256 is unavailable", exception);
        }
    }

    private record Selection(Path original, String hash) {}
    private record Manifests(BufferedWriter inputs, BufferedWriter provenance, BufferedWriter shadows) {}
    private record Candidate(Path original, Path staged, int index, String entry, String name, byte[] bytes, boolean projectInput) {}
}
