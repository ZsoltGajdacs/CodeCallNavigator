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
        result.put("modules", modules);
        result.put("sourceSets", sourceSets);
        result.put("buildInputs", inputs);
        result.put("outputRoots", outputs);
        return result;
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
        add(inputs, request.getUserSettingsFile());
        add(inputs, request.getGlobalSettingsFile());
        add(inputs, request.getUserToolchainsFile());
        add(inputs, request.getGlobalToolchainsFile());
        File root = request.getMultiModuleProjectDirectory();
        if (root != null) {
            add(inputs, new File(root, ".mvn"));
            add(inputs, new File(root, "mvnw"));
        }
        return inputs;
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
