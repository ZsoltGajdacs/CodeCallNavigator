package codegraph.bytecode;

import com.adrninistrator.javacg2.common.enums.JavaCG2CalleeRawActualEnum;
import com.adrninistrator.javacg2.common.enums.JavaCG2OutPutFileTypeEnum;
import com.adrninistrator.javacg2.conf.JavaCG2ConfigureWrapper;
import com.adrninistrator.javacg2.conf.enums.JavaCG2ConfigKeyEnum;
import com.adrninistrator.javacg2.conf.enums.JavaCG2OtherConfigFileUseListEnum;
import com.adrninistrator.javacg2.dto.output.JavaCG2OutputInfo;
import com.adrninistrator.javacg2.entry.JavaCG2Entry;
import com.adrninistrator.javacg2.el.enums.JavaCG2ElConfigEnum;
import java.io.IOException;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.List;
import java.util.Properties;
import java.util.Set;

/** Only configures the existing analyzer and reports its result; it resolves no calls. */
public final class Analyzer {
    private Analyzer() {}

    public static void main(String[] arguments) {
        try {
            System.exit(analyze(Options.parse(arguments)));
        } catch (Exception error) {
            System.err.println("CodeGraph bytecode analysis failed: " + error.getMessage());
            System.exit(2);
        }
    }

    private static int analyze(Options options) throws IOException {
        Files.createDirectories(options.outputRoot());
        Set<Integer> inputsWithMethods = BytecodeMetadata.write(options.outputRoot(), options.inputs());
        boolean expectsMethods = ProjectMethodFilter.expectsMethods(options.projectInputs(), inputsWithMethods);
        JavaCG2Entry engine = new JavaCG2Entry(configuration(options));
        boolean succeeded = engine.run();
        JavaCG2OutputInfo output = engine.getJavaCG2InputAndOutput().getJavaCG2OutputInfo();
        boolean declarationsComplete = hasMethodDeclarations(output) || !expectsMethods;
        writeStatus(options.outputRoot(), output, succeeded, declarationsComplete);
        if (!succeeded || output == null) {
            return 1;
        }
        if (!output.getFrameSnapshotNumExceedMethodSetReadOnly().isEmpty()) {
            System.err.println("Incomplete analysis: engine frame snapshot limit exceeded for "
                    + output.getFrameSnapshotNumExceedMethodSetReadOnly().size() + " methods");
            return 3;
        }
        if (!declarationsComplete) {
            System.err.println("Incomplete analysis: no project method declarations were emitted");
            return 1;
        }
        System.out.println("ENGINE_OUTPUT_DIRECTORY=" + output.getOutputDirPath());
        return 0;
    }

    private static boolean hasMethodDeclarations(JavaCG2OutputInfo output) throws IOException {
        if (output == null) {
            return false;
        }
        Path methodInfo = Path.of(output.getMainFilePath(JavaCG2OutPutFileTypeEnum.OPFTE_METHOD_INFO));
        return Files.isRegularFile(methodInfo) && Files.size(methodInfo) > 0;
    }

    private static JavaCG2ConfigureWrapper configuration(Options options) throws IOException {
        JavaCG2ConfigureWrapper configuration = new JavaCG2ConfigureWrapper(true);
        configuration.setOtherConfigList(JavaCG2OtherConfigFileUseListEnum.OCFULE_JAR_DIR, options.inputs());
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_OUTPUT_ROOT_PATH, options.outputRoot().toString());
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_JDK_RUNTIME_MAJOR_VERSION, options.runtimeMajor());
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_CONTINUE_WHEN_ERROR, "false");
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_PARSE_METHOD_CALL_TYPE_VALUE, "true");
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_EL_IGNORE_DATA_MAX_LINE_NUM, "0");
        // The engine otherwise excludes project equals/hashCode overrides by default.
        configuration.setElConfigText(JavaCG2ElConfigEnum.ECE_PARSE_IGNORE_METHOD,
                ProjectMethodFilter.expression(options.inputs(), options.projectInputs()));
        String bothTargets = JavaCG2CalleeRawActualEnum.CRAE_RAW_ACTUAL.getType();
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_HANDLE_CALLEE_NEW_RAW_ACTUAL, bothTargets);
        configuration.setMainConfig(JavaCG2ConfigKeyEnum.CKE_HANDLE_CALLEE_SPRING_BEAN_RAW_ACTUAL, bothTargets);
        return configuration;
    }

    private static void writeStatus(Path outputRoot, JavaCG2OutputInfo output, boolean succeeded, boolean declarationsComplete) throws IOException {
        Properties status = new Properties();
        status.setProperty("engine", "java-callgraph2");
        status.setProperty("engineVersion", "4.1.0");
        status.setProperty("bcelVersion", "6.13.0");
        status.setProperty("engineSucceeded", Boolean.toString(succeeded));
        status.setProperty("analysisComplete", Boolean.toString(succeeded && declarationsComplete));
        if (output != null) {
            status.setProperty("outputDirectory", output.getOutputDirPath());
            status.setProperty("frameSnapshotLimitMethods",
                    Integer.toString(output.getFrameSnapshotNumExceedMethodSetReadOnly().size()));
            if (!output.getFrameSnapshotNumExceedMethodSetReadOnly().isEmpty()) {
                status.setProperty("analysisComplete", "false");
            }
        }
        try (Writer writer = Files.newBufferedWriter(outputRoot.resolve("analysis-status.properties"))) {
            status.store(writer, "CodeGraph existing-engine execution status");
        }
    }

    private record Options(Path outputRoot, String runtimeMajor, List<String> inputs, Set<Integer> projectInputs) {
        static Options parse(String[] arguments) {
            if (arguments.length < 3) {
                throw new IllegalArgumentException("Usage: codegraph-bytecode OUTPUT_ROOT JAVA_MAJOR CLASS_DIR_OR_JAR...");
            }
            String runtimeMajor = arguments[1];
            if (!List.of("21", "25").contains(runtimeMajor)) {
                throw new IllegalArgumentException("JAVA_MAJOR must be 21 or 25");
            }
            boolean filtered = arguments.length > 3 && arguments[2].equals("--project-inputs");
            int firstInput = filtered ? 4 : 2;
            List<String> inputs = Arrays.stream(arguments).skip(firstInput).map(Options::validatedInput).toList();
            Set<Integer> projectInputs = filtered ? InputSelection.parse(arguments[3], inputs.size()) : null;
            return new Options(Path.of(arguments[0]).toAbsolutePath().normalize(), runtimeMajor, inputs, projectInputs);
        }

        private static String validatedInput(String value) {
            Path path = Path.of(value).toAbsolutePath().normalize();
            if (!Files.exists(path)) {
                throw new IllegalArgumentException("Analyzer input does not exist: " + path);
            }
            return path.toString();
        }
    }
}
