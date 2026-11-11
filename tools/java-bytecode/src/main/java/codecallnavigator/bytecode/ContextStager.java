package codecallnavigator.bytecode;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

/** Selects the first class definition in an already evaluated, ordered compile context. */
public final class ContextStager {
    private final Path output;
    private final Map<String, Selection> selected = new HashMap<>();
    private final Set<Integer> projectInputs;
    private int inputIndex;
    private long stagedBytes;
    private final ArtifactInventory inventory;

    private ContextStager(Path output, Set<Integer> projectInputs, ArtifactInventory inventory) {
        this.output = output;
        this.projectInputs = projectInputs;
        this.inventory = inventory;
    }

    public static void main(String[] arguments) throws IOException {
        if (arguments.length < 2) {
            throw new IllegalArgumentException("Usage: ContextStager OUTPUT_ROOT CLASSPATH_ENTRY...");
        }
        Path output = Path.of(arguments[0]).toAbsolutePath().normalize();
        boolean filtered = arguments.length > 3 && arguments[1].equals("--project-inputs");
        int firstInput = filtered ? 3 : 1;
        Path cache = output.resolve("artifact-inventories");
        if (arguments.length > firstInput + 1 && arguments[firstInput].equals("--artifact-cache")) {
            cache = Path.of(arguments[firstInput + 1]);
            firstInput += 2;
        }
        Set<Integer> projectInputs = filtered ? InputSelection.parse(arguments[2], arguments.length - firstInput) : null;
        Files.createDirectories(output);
        ArtifactInventory inventory = new ArtifactInventory(cache);
        ContextStager stager = new ContextStager(output, projectInputs, inventory);
        stager.stage(arguments, firstInput);
        inventory.writeStatistics(output.resolve("artifact-stats.properties"));
        Files.writeString(output.resolve("artifact-stats.properties"), "staged_bytes=" + stager.stagedBytes + "\n", java.nio.file.StandardOpenOption.APPEND);
    }

    private void stage(String[] arguments, int firstInput) throws IOException {
        try (var inputs = Files.newBufferedWriter(output.resolve("inputs.tsv"));
                var provenance = Files.newBufferedWriter(output.resolve("provenance.tsv"));
                var shadows = Files.newBufferedWriter(output.resolve("shadows.tsv"))) {
            ContextTsv.write(inputs, "original_path", "staged_path");
            ContextTsv.write(provenance, "input_index", "original_path", "class_name", "class_entry", "sha256");
            ContextTsv.write(shadows, "class_name", "selected_path", "shadowed_path", "identical");
            Manifests manifests = new Manifests(inputs, provenance, shadows);
            try (var prefetch = new ArtifactPrefetch(inventory, requestedInputs(arguments, firstInput))) {
                prefetch.visit(prepared -> stageInput(prepared, manifests));
            }
        }
    }

    private List<ArtifactPrefetch.Input> requestedInputs(String[] arguments, int firstInput) {
        List<ArtifactPrefetch.Input> inputs = new ArrayList<>();
        for (int index = firstInput; index < arguments.length; index++) {
            boolean project = projectInputs == null || projectInputs.contains(index - firstInput + 1);
            inputs.add(new ArtifactPrefetch.Input(Path.of(arguments[index]).toAbsolutePath().normalize(), project));
        }
        return inputs;
    }

    private void stageInput(ArtifactPrefetch.Prepared prepared, Manifests manifests) throws IOException {
        Path original = prepared.input().original();
        boolean projectInput = prepared.input().project();
        int index = inputIndex + 1;
        Path staged = output.resolve("input-" + index);
        for (ArtifactInventory.Definition definition : prepared.definitions()) {
            select(new Candidate(original, staged, index, definition, projectInput), manifests);
        }
        if (Files.isDirectory(staged)) {
            ContextTsv.write(manifests.inputs(), original, staged);
            inputIndex = index;
        }
    }

    private void select(Candidate candidate, Manifests manifests) throws IOException {
        String hash = candidate.definition().hash();
        Selection previous = selected.putIfAbsent(candidate.definition().name(), new Selection(candidate.original(), hash));
        if (previous != null) {
            ContextTsv.write(manifests.shadows(), candidate.definition().name(), previous.original(), candidate.original(), previous.hash().equals(hash));
            return;
        }
        if (!candidate.projectInput()) {
            return;
        }
        Path target = candidate.staged().resolve(candidate.definition().name().replace('.', '/') + ".class").normalize();
        if (!target.startsWith(candidate.staged())) {
            throw new IOException("Invalid class name: " + candidate.definition().name());
        }
        Files.createDirectories(target.getParent());
        Files.copy(candidate.definition().bytes(), target);
        stagedBytes += Files.size(target);
        ContextTsv.write(manifests.provenance(), candidate.index(), candidate.original(), candidate.definition().name(), candidate.definition().entry(), hash);
    }

    private record Selection(Path original, String hash) {}
    private record Manifests(BufferedWriter inputs, BufferedWriter provenance, BufferedWriter shadows) {}
    private record Candidate(Path original, Path staged, int index, ArtifactInventory.Definition definition, boolean projectInput) {}
}
