package codecallnavigator.maven;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.ArrayList;
import java.util.Collection;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;

/** Native compiler-task boundaries; this does not certify individual javac reads. */
public final class CompilerJournal {
    public static final int VERSION = 1;
    public static final String CAPABILITY = "compiler_observation_v1";
    private final Path output;
    private final String attempt;
    private final Map<String, Operation> operations = new LinkedHashMap<>();
    private Map<String, String> failure;

    public CompilerJournal(Path output, String attempt) throws IOException {
        if (output == null) {
            throw new IOException("Compiler observation requires an output");
        }
        if (attempt == null || attempt.isBlank()) {
            throw new IOException("Compiler observation requires an attempt token");
        }
        this.output = output.toAbsolutePath().normalize();
        this.attempt = attempt;
    }

    public enum Outcome {
        SUCCESS, UP_TO_DATE, FROM_CACHE, NO_SOURCE, NOT_APPLICABLE, UNSUPPORTED;
        String wire() { return name().toLowerCase(java.util.Locale.ROOT); }
    }

    public record Owner(String project, String sourceSet, String operation) {}
    public record Inputs(List<String> sources, List<String> classpath, List<String> classes,
            List<String> generatedSources) {}

    public synchronized void before(Owner owner, Inputs inputs) throws IOException {
        if (operations.containsKey(owner.operation())) {
            throw new IOException("Repeated compiler operation identity: " + owner.operation());
        }
        Operation entry = new Operation(owner, inputs);
        entry.before = CompileSnapshot.capture(inputs.sources(), inputs.classpath(),
                inputs.generatedSources(), inputs.classes());
        operations.put(owner.operation(), entry);
        write();
    }

    public synchronized void after(Owner owner, Inputs inputs, Outcome outcome) throws IOException {
        Operation entry = required(owner);
        if (entry.detail != null) {
            return;
        }
        entry.sources = List.copyOf(inputs.sources());
        entry.classpath = List.copyOf(inputs.classpath());
        // Capture the consumed projection at this boundary, never by resolving
        // historical paths against a later filesystem/link topology.
        entry.consumedAfter = CompileSnapshot.capture(entry.sources, entry.classpath,
                entry.generatedSources, entry.classes);
        entry.after = entry.captureAfter();
        CompileSnapshot.Difference changed = entry.before.differenceConsumed(entry.consumedAfter);
        if (changed != null) {
            fail(entry, changed);
        }
        entry.outcome = outcome == Outcome.SUCCESS && entry.before.javaMembers().isEmpty()
                ? Outcome.NO_SOURCE : outcome;
        if (outcome == Outcome.UNSUPPORTED) {
            entry.detail = "Native compiler task skipped without a reusable compilation outcome";
        }
        write();
    }

    public synchronized void unsupported(Owner owner, Inputs inputs, String detail) throws IOException {
        Operation entry = new Operation(owner, inputs);
        entry.before = new CompileSnapshot();
        entry.after = entry.before;
        entry.detail = detail;
        operations.put(owner.operation(), entry);
        write();
    }

    public synchronized void requireSourceSelection(Owner owner, List<String> nativeFiles) throws IOException {
        Operation entry = required(owner);
        java.util.Set<String> selected = new java.util.TreeSet<>();
        for (String source : nativeFiles) {
            if (!CompileSnapshot.excluded(Path.of(source), entry.generatedSources)) {
                selected.add(Path.of(source).toAbsolutePath().toString());
            }
        }
        if (!entry.before.javaMembers().equals(selected)) {
            entry.detail = "Native compiler source selection cannot be represented by complete Java source scopes";
            entry.after = entry.before;
            write();
        }
    }

    public synchronized void failed(Owner owner) throws IOException {
        Operation entry = operations.get(owner.operation());
        if (entry != null) {
            entry.detail = "Native compiler operation failed";
            write();
        }
    }

    /** Recheck every completed invocation after all later build tasks have finished. */
    public synchronized void finish() throws IOException {
        for (Operation entry : operations.values()) {
            if (entry.after != null && entry.detail == null) {
                CompileSnapshot.Difference changed = entry.after.difference(entry.captureAfter());
                if (changed != null) {
                    fail(entry, changed);
                }
            }
        }
        write();
    }

    private Operation required(Owner owner) throws IOException {
        Operation entry = operations.get(owner.operation());
        if (entry == null) {
            throw new IOException("Compiler completion has no matching start: " + owner.operation());
        }
        return entry;
    }

    private void fail(Operation entry, CompileSnapshot.Difference difference) throws IOException {
        failure = Map.of("path", difference.path(), "change", difference.change(), "operation", entry.operation);
        write();
        throw new IOException("Compiler input changed: " + difference.path() + " (" + difference.change() + ")");
    }

    private void write() throws IOException {
        Map<String, Object> document = new LinkedHashMap<>();
        document.put("version", VERSION);
        document.put("attempt", attempt);
        document.put("operations", operations.values().stream().map(Operation::document).toList());
        if (failure != null) {
            document.put("failure", failure);
        }
        Path temporary = Files.createTempFile(output.getParent(), "compile-observation-", ".json");
        try {
            Files.writeString(temporary, Json.write(document), StandardCharsets.UTF_8);
            Files.move(temporary, output, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } finally {
            Files.deleteIfExists(temporary);
        }
    }

    private static List<String> combined(Collection<String> first, Collection<String> second) {
        LinkedHashSet<String> result = new LinkedHashSet<>(first);
        result.addAll(second);
        return new ArrayList<>(result);
    }

    private static final class Operation {
        final String project;
        final String sourceSet;
        final String operation;
        final List<String> classes;
        final List<String> generatedSources;
        List<String> sources;
        List<String> classpath;
        CompileSnapshot before;
        CompileSnapshot consumedAfter;
        CompileSnapshot after;
        Outcome outcome = Outcome.UNSUPPORTED;
        String detail;

        Operation(Owner owner, Inputs inputs) {
            this.project = Path.of(owner.project()).toAbsolutePath().toString();
            this.sourceSet = owner.sourceSet();
            this.operation = owner.operation();
            this.sources = List.copyOf(inputs.sources());
            this.classpath = List.copyOf(inputs.classpath());
            this.classes = List.copyOf(inputs.classes());
            this.generatedSources = List.copyOf(inputs.generatedSources());
        }

        CompileSnapshot captureAfter() throws IOException {
            return CompileSnapshot.capture(combined(sources, generatedSources), classpath, List.of(), classes);
        }

        Map<String, Object> document() {
            Map<String, Object> result = new LinkedHashMap<>();
            result.put("project", project);
            result.put("sourceSet", sourceSet);
            result.put("operation", operation);
            result.put("outcome", outcome.wire());
            result.put("before", before.document());
            result.put("consumedAfter", (consumedAfter == null ? before : consumedAfter).document());
            result.put("after", (after == null ? before : after).document());
            result.put("outputs", Map.of("classes", classes, "generatedSources", generatedSources));
            if (detail != null) {
                result.put("detail", detail);
            }
            return result;
        }
    }
}
