package codegraph.bytecode;

import java.io.BufferedWriter;
import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.TreeSet;
import org.apache.bcel.classfile.ClassParser;
import org.apache.bcel.classfile.JavaClass;

/** Emits bindings only after replayed executable classes equal the real build outputs. */
final class LambdaCorrelation {
    private LambdaCorrelation() {}

    static void write(LambdaCompilation.Context context, String directories,
            List<LambdaSourceSite> sites, BufferedWriter writer) throws IOException {
        if (sites.isEmpty()) return;
        var actual = classes(directories);
        var owners = new TreeMap<String, List<LambdaSourceSite>>();
        for (var site : sites) owners.computeIfAbsent(site.owner(), ignored -> new ArrayList<>()).add(site);
        var replays = new HashMap<Integer, List<LambdaCompilation.Result>>();
        for (var entry : owners.entrySet()) {
            String owner = entry.getKey();
            var type = actual.get(owner);
            if (type == null) {
                outcome(writer, owner, "unavailable", "Compiled owner is missing or ambiguous");
                continue;
            }
            int release = type.getMajor() - 44;
            var candidates = replays.computeIfAbsent(release, ignored -> List.of(
                    LambdaCompilation.run(context, release, "-g"),
                    LambdaCompilation.run(context, release, "-g:none")));
            verify(writer, type, entry.getValue(), candidates);
        }
    }

    private static Map<String, JavaClass> classes(String directories) throws IOException {
        var classes = new HashMap<String, JavaClass>();
        var duplicates = new TreeSet<String>();
        for (String directory : directories.split(java.util.regex.Pattern.quote(File.pathSeparator))) {
            Path root = Path.of(directory);
            if (Files.isDirectory(root)) ClassInventory.visit(root, (entry, name, bytes) -> {
                var type = new ClassParser(new ByteArrayInputStream(bytes), entry).parse();
                if (classes.putIfAbsent(name, type) != null) duplicates.add(name);
            });
        }
        duplicates.forEach(classes::remove);
        return classes;
    }

    private static void verify(BufferedWriter writer, JavaClass actual, List<LambdaSourceSite> sites,
            List<LambdaCompilation.Result> candidates) throws IOException {
        String owner = actual.getClassName();
        List<Proof> proofs;
        try {
            Object expected = CompiledClassModel.of(actual);
            proofs = candidates.stream().map(candidate -> prove(owner, expected, sites, candidate)).toList();
        } catch (RuntimeException error) {
            outcome(writer, owner, "unavailable", "Cannot compare executable class: " + error.getClass().getSimpleName());
            return;
        }
        var matches = proofs.stream().filter(proof -> proof.bindings() != null).map(Proof::bindings).toList();
        boolean generated = proofs.stream().anyMatch(Proof::generated);
        String reason = proofs.getLast().reason();
        if (matches.isEmpty()) {
            outcome(writer, owner, generated ? "rejected" : "unavailable", reason);
            return;
        }
        var binding = matches.getFirst();
        if (matches.stream().anyMatch(candidate -> !candidate.equals(binding))) {
            outcome(writer, owner, "rejected", "Matching compiler replays disagree on source attribution");
            return;
        }
        outcome(writer, owner, "verified", "");
        for (var entry : binding.entrySet()) row(writer, "lambda-binding", FlowJson.object(
                "implementation", entry.getKey(), "sources", entry.getValue()));
    }

    private record Proof(Map<String, List<String>> bindings, boolean generated, String reason) {}

    private static Proof prove(String owner, Object expected, List<LambdaSourceSite> sites,
            LambdaCompilation.Result candidate) {
        var replay = candidate.classes().get(owner);
        if (replay == null) return new Proof(null, false, candidate.failure());
        if (!expected.equals(CompiledClassModel.of(replay))) {
            return new Proof(null, true, "Replayed executable class differs from actual build output");
        }
        var bindings = bindings(sites, candidate.implementations());
        return new Proof(bindings, true, bindings == null ? "Not every source lambda has a lowered implementation handle" : "");
    }

    private static Map<String, List<String>> bindings(List<LambdaSourceSite> sites,
            Map<LambdaSourceSite, String> implementations) {
        var groups = new TreeMap<String, TreeSet<String>>();
        for (var site : sites) {
            String implementation = implementations.get(site);
            if (implementation == null || !implementation.startsWith(site.owner() + ":")) return null;
            groups.computeIfAbsent(implementation, ignored -> new TreeSet<>()).add(site.symbol());
        }
        var result = new TreeMap<String, List<String>>();
        groups.forEach((implementation, members) -> result.put(implementation, List.copyOf(members)));
        return result;
    }

    private static void outcome(BufferedWriter writer, String owner, String status, String reason) throws IOException {
        row(writer, "lambda-correlation", FlowJson.object("owner", owner, "status", status, "reason", reason));
    }
    private static void row(BufferedWriter writer, String kind, Object value) throws IOException {
        ContextTsv.write(writer, kind, "", "", FlowJson.encode(value), "", "", "", "", "");
    }
}
