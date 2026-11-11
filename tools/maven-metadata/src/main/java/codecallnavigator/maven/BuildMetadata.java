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
                sourceSets.add(metadata.sourceSet(false, evaluated));
                sourceSets.add(metadata.sourceSet(true, evaluated));
            }
        }
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("modules", modules);
        result.put("sourceSets", sourceSets);
        result.put("buildInputs", inputs);
        result.put("outputRoots", outputs);
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
