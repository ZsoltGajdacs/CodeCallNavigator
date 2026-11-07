package codegraph.bytecode;

import java.io.BufferedWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.IntSummaryStatistics;
import java.util.List;
import java.util.jar.JarFile;
import java.util.stream.Collectors;
import org.apache.bcel.classfile.AnnotationEntry;
import org.apache.bcel.classfile.ClassParser;
import org.apache.bcel.classfile.JavaClass;
import org.apache.bcel.classfile.LineNumber;
import org.apache.bcel.classfile.LineNumberTable;
import org.apache.bcel.classfile.Method;
import org.apache.bcel.generic.Type;

/** Exports existing BCEL attributes and input provenance, without resolving relationships. */
final class BytecodeMetadata implements AutoCloseable {
    private final BufferedWriter classes;
    private final BufferedWriter methods;

    private BytecodeMetadata(Path outputRoot) throws IOException {
        classes = Files.newBufferedWriter(outputRoot.resolve("class_metadata.tsv"));
        methods = Files.newBufferedWriter(outputRoot.resolve("method_metadata.tsv"));
        classes.write("input_index\tinput_path\tclass_entry\tclass_name\tsource_file\tflags\tannotations\n");
        methods.write("input_index\tinput_path\tclass_entry\tclass_name\tsource_file\tmethod\treturn_type\tflags\tstart_line\tend_line\tannotations\n");
    }

    static void write(Path outputRoot, List<String> inputs) throws IOException {
        try (BytecodeMetadata metadata = new BytecodeMetadata(outputRoot)) {
            for (int index = 0; index < inputs.size(); index++) {
                metadata.writeInput(new Input(index + 1, Path.of(inputs.get(index))));
            }
        }
    }

    private void writeInput(Input input) throws IOException {
        if (Files.isDirectory(input.path())) {
            writeDirectory(input);
            return;
        }
        writeArchive(input);
    }

    private void writeDirectory(Input input) throws IOException {
        try (var files = Files.walk(input.path())) {
            for (Path path : files.filter(Files::isRegularFile).filter(BytecodeMetadata::isClassFile).sorted().toList()) {
                String entry = input.path().relativize(path).toString();
                writeClass(input, entry, new ClassParser(path.toString()).parse());
            }
        }
    }

    private static boolean isClassFile(Path path) {
        return path.toString().endsWith(".class");
    }

    private void writeArchive(Input input) throws IOException {
        try (JarFile archive = new JarFile(input.path().toFile())) {
            var entries = archive.stream().filter(entry -> entry.getName().endsWith(".class")).toList();
            for (var entry : entries) {
                try (var stream = archive.getInputStream(entry)) {
                    writeClass(input, entry.getName(), new ClassParser(stream, entry.getName()).parse());
                }
            }
        }
    }

    private void writeClass(Input input, String entry, JavaClass javaClass) throws IOException {
        String sourceFile = javaClass.getSourceFileName();
        writeRow(classes, List.of(input.index(), input.path(), entry, javaClass.getClassName(),
                sourceFile, javaClass.getAccessFlags(), annotations(javaClass.getAnnotationEntries())));
        ClassOrigin origin = new ClassOrigin(input, entry, javaClass.getClassName(), sourceFile);
        for (Method method : javaClass.getMethods()) {
            writeMethod(origin, method);
        }
    }

    private void writeMethod(ClassOrigin origin, Method method) throws IOException {
        String arguments = Arrays.stream(method.getArgumentTypes()).map(Type::toString).collect(Collectors.joining(","));
        String signature = origin.name() + ":" + method.getName() + "(" + arguments + ")";
        IntSummaryStatistics lines = lineNumbers(method.getLineNumberTable());
        int start = lines.getCount() == 0 ? 0 : lines.getMin();
        int end = lines.getCount() == 0 ? 0 : lines.getMax();
        writeRow(methods, List.of(origin.input().index(), origin.input().path(), origin.entry(), origin.name(),
                origin.sourceFile(), signature, method.getReturnType(), method.getAccessFlags(), start, end,
                annotations(method.getAnnotationEntries())));
    }

    private static IntSummaryStatistics lineNumbers(LineNumberTable table) {
        if (table == null) {
            return new IntSummaryStatistics();
        }
        return Arrays.stream(table.getLineNumberTable()).mapToInt(LineNumber::getLineNumber).summaryStatistics();
    }

    private static String annotations(AnnotationEntry[] entries) {
        return Arrays.stream(entries).map(AnnotationEntry::getAnnotationType).collect(Collectors.joining(","));
    }

    private static void writeRow(BufferedWriter writer, List<?> fields) throws IOException {
        writer.write(fields.stream().map(Object::toString).map(BytecodeMetadata::escape).collect(Collectors.joining("\t")));
        writer.newLine();
    }

    private static String escape(String field) {
        return field.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r");
    }

    @Override
    public void close() throws IOException {
        try {
            classes.close();
        } finally {
            methods.close();
        }
    }

    private record Input(int index, Path path) {}
    private record ClassOrigin(Input input, String entry, String name, String sourceFile) {}
}
