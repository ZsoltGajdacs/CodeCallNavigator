package codecallnavigator.maven;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.apache.maven.execution.MavenExecutionRequest;
import org.apache.maven.execution.MavenSession;
import org.apache.maven.project.MavenProject;

final class BuildMetadata {
    private BuildMetadata() {}

    static Map<String, Object> collect(MavenSession session, boolean evaluated) throws IOException {
        Set<String> modules = new LinkedHashSet<>();
        Set<String> inputs = sessionInputs(session.getRequest());
        Set<String> outputs = new LinkedHashSet<>();
        List<Map<String, Object>> sourceSets = new ArrayList<>();
        for (MavenProject project : session.getProjects()) {
            modules.add(path(project.getBasedir()));
            ProjectMetadata metadata = new ProjectMetadata(project);
            inputs.addAll(metadata.inputs());
            outputs.addAll(metadata.outputs());
            // Aggregators retain ownership but have no compilation output of their own.
            if (!"pom".equals(project.getPackaging())) {
                sourceSets.add(sourceSet(project, metadata, false, evaluated));
                sourceSets.add(sourceSet(project, metadata, true, evaluated));
            }
        }
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("compileObservationVersion", 1);
        result.put("configurationInputs", configurationInputs(session));
        result.put("modules", modules);
        result.put("sourceSets", sourceSets);
        result.put("buildInputs", inputs);
        result.put("outputRoots", outputs);
        return result;
    }

    private static Set<String> configurationInputs(MavenSession session) throws IOException {
        MavenExecutionRequest request = session.getRequest();
        Set<String> inputs = new LinkedHashSet<>();
        addInput(inputs, request.getUserSettingsFile());
        addInput(inputs, request.getGlobalSettingsFile());
        addInput(inputs, request.getUserToolchainsFile());
        addInput(inputs, request.getGlobalToolchainsFile());
        addConfigurationFiles(inputs, request.getMultiModuleProjectDirectory());
        for (MavenProject project : session.getProjects()) {
            addParentConfigurations(inputs, project);
        }
        return inputs;
    }

    private static void addConfigurationFiles(Set<String> inputs, File root) throws IOException {
        if (root == null) {
            return;
        }
        for (String name : List.of(".mvn/maven.config", ".mvn/jvm.config", ".mvn/extensions.xml",
                ".mvn/wrapper/maven-wrapper.properties", ".mvn/wrapper/maven-wrapper.jar", "mvnw", "mvnw.cmd")) {
            addInput(inputs, new File(root, name));
        }
    }

    private static void addParentConfigurations(Set<String> inputs, MavenProject project) throws IOException {
        for (MavenProject current = project; current != null; current = current.getParent()) {
            addInput(inputs, current.getFile());
            addInput(inputs, current.getParentFile());
        }
    }

    private static Map<String, Object> sourceSet(MavenProject project, ProjectMetadata metadata,
            boolean test, boolean evaluated) throws IOException {
        Map<String, Object> result = metadata.sourceSet(test, evaluated);
        String output = test ? project.getBuild().getTestOutputDirectory()
                : project.getBuild().getOutputDirectory();
        File directory = new File(output);
        if (!directory.isAbsolute()) {
            directory = new File(project.getBasedir(), output);
        }
        // Canonical classDirs retain ownership semantics; this observation must not
        // erase a symlink in the evaluated output spelling used by the build tool.
        result.put("rawClassDirs", List.of(directory.getAbsolutePath()));
        return result;
    }

    private static Set<String> sessionInputs(MavenExecutionRequest request) throws IOException {
        Set<String> inputs = new LinkedHashSet<>();
        addInput(inputs, request.getUserSettingsFile());
        addInput(inputs, request.getGlobalSettingsFile());
        addInput(inputs, request.getUserToolchainsFile());
        addInput(inputs, request.getGlobalToolchainsFile());
        File root = request.getMultiModuleProjectDirectory();
        if (root != null) {
            File configuration = new File(root, ".mvn");
            addInput(inputs, configuration);
            for (String name : List.of("maven.config", "jvm.config", "extensions.xml",
                    "wrapper/maven-wrapper.properties", "wrapper/maven-wrapper.jar")) {
                addInput(inputs, new File(configuration, name));
            }
            addInput(inputs, new File(root, "mvnw"));
        }
        return inputs;
    }

    private static void addInput(Set<String> paths, File file) throws IOException {
        if (file != null) {
            String lexical = file.toPath().toAbsolutePath().normalize().toString();
            if (!lexical.equals(path(file))) {
                paths.add(lexical);
            }
            add(paths, file);
        }
    }

    static void add(Set<String> paths, File file) throws IOException {
        if (file != null) {
            paths.add(path(file));
        }
    }

    static String path(File file) throws IOException {
        return file.getCanonicalPath();
    }
}
