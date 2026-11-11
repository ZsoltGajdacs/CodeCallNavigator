package codecallnavigator.maven;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import org.apache.maven.model.Plugin;
import org.apache.maven.model.PluginExecution;
import org.apache.maven.model.Resource;
import org.apache.maven.project.MavenProject;
import org.codehaus.plexus.util.xml.Xpp3Dom;

final class ProjectMetadata {
    private final MavenProject project;
    private final Set<String> inputs = new LinkedHashSet<>();
    private final Set<String> outputs = new LinkedHashSet<>();

    ProjectMetadata(MavenProject project) throws IOException {
        this.project = project;
        collectBuildPaths();
    }

    Set<String> inputs() { return inputs; }
    Set<String> outputs() { return outputs; }

    Map<String, Object> sourceSet(boolean test, boolean evaluated) throws IOException {
        Set<String> roots = paths(test ? project.getTestCompileSourceRoots() : project.getCompileSourceRoots());
        Set<String> generated = generatedRoots(test);
        for (String root : roots) {
            if (new File(root).toPath().startsWith(resolve(project.getBuild().getDirectory()).toPath())) {
                generated.add(root);
            }
        }
        roots.addAll(generated);
        Map<String, Object> result = new LinkedHashMap<>();
        result.put("project", project.getGroupId() + ":" + project.getArtifactId());
        result.put("projectDir", BuildMetadata.path(project.getBasedir()));
        result.put("isTest", test);
        result.put("sourceSet", test ? "test" : "main");
        result.put("compileClasspath", evaluated ? compileClasspath(test) : List.of());
        result.put("classpathAliases", classpathAliases(test));
        result.put("sourceRoots", roots);
        result.put("generatedRoots", generated);
        result.put("classDirs", List.of(BuildMetadata.path(resolve(test
                ? project.getBuild().getTestOutputDirectory() : project.getBuild().getOutputDirectory()))));
        result.put("buildInputs", inputs);
        return result;
    }

    private Set<String> compileClasspath(boolean test) throws IOException {
        try {
            return paths(test ? project.getTestClasspathElements() : project.getCompileClasspathElements());
        } catch (org.apache.maven.artifact.DependencyResolutionRequiredException exception) {
            throw new IOException("Compile classpath is unresolved for " + project.getId(), exception);
        }
    }

    private Set<String> classpathAliases(boolean test) throws IOException {
        if (test) {
            return testArtifactAliases();
        }
        Set<String> result = new LinkedHashSet<>();
        if (project.getArtifact() != null) {
            BuildMetadata.add(result, project.getArtifact().getFile());
        }
        if ("jar".equals(project.getPackaging())) {
            BuildMetadata.add(result, resolve(project.getBuild().getDirectory() + "/"
                    + project.getBuild().getFinalName() + ".jar"));
        }
        return result;
    }

    private Set<String> testArtifactAliases() throws IOException {
        Set<String> result = new LinkedHashSet<>();
        for (var artifact : project.getAttachedArtifacts()) {
            if ("tests".equals(artifact.getClassifier())) {
                BuildMetadata.add(result, artifact.getFile());
            }
        }
        return result;
    }

    private void collectBuildPaths() throws IOException {
        for (MavenProject parent = project; parent != null; parent = parent.getParent()) {
            BuildMetadata.add(inputs, parent.getFile());
            BuildMetadata.add(inputs, parent.getParentFile());
        }
        for (Resource resource : combinedResources()) {
            BuildMetadata.add(inputs, resolve(resource.getDirectory()));
        }
        inputs.addAll(paths(project.getFilters()));
        for (String value : List.of(project.getBuild().getDirectory(), project.getBuild().getOutputDirectory(),
                project.getBuild().getTestOutputDirectory())) {
            BuildMetadata.add(outputs, resolve(value));
        }
        outputs.addAll(generatedRoots(false));
        outputs.addAll(generatedRoots(true));
        for (Plugin plugin : project.getBuildPlugins()) {
            collectConfigurationInputs(plugin.getConfiguration());
            for (PluginExecution execution : plugin.getExecutions()) {
                collectConfigurationInputs(execution.getConfiguration());
            }
        }
    }

    private List<Resource> combinedResources() {
        List<Resource> resources = new ArrayList<>(project.getResources());
        resources.addAll(project.getTestResources());
        return resources;
    }

    private Set<String> generatedRoots(boolean test) throws IOException {
        Set<String> result = new LinkedHashSet<>();
        String option = test ? "generatedTestSourcesDirectory" : "generatedSourcesDirectory";
        String fallback = project.getBuild().getDirectory() + (test
                ? "/generated-test-sources/test-annotations" : "/generated-sources/annotations");
        BuildMetadata.add(result, resolve(fallback));
        Plugin compiler = project.getPlugin("org.apache.maven.plugins:maven-compiler-plugin");
        if (compiler != null) {
            collectGeneratedOption(result, compiler.getConfiguration(), option);
            for (PluginExecution execution : compiler.getExecutions()) {
                collectGeneratedOption(result, execution.getConfiguration(), option);
            }
        }
        return result;
    }

    private void collectGeneratedOption(Set<String> paths, Object configuration, String option) throws IOException {
        if (configuration instanceof Xpp3Dom tree && tree.getChild(option) != null) {
            BuildMetadata.add(paths, resolve(tree.getChild(option).getValue()));
        }
    }

    private void collectConfigurationInputs(Object configuration) throws IOException {
        if (!(configuration instanceof Xpp3Dom tree)) {
            return;
        }
        String value = tree.getValue();
        if (value != null && !value.isBlank()) {
            File candidate = resolve(value);
            if (candidate.exists() && outputs.stream().noneMatch(output -> candidate.toPath().startsWith(output))) {
                BuildMetadata.add(inputs, candidate);
            }
        }
        for (Xpp3Dom child : tree.getChildren()) {
            collectConfigurationInputs(child);
        }
    }

    private Set<String> paths(List<String> values) throws IOException {
        Set<String> result = new LinkedHashSet<>();
        for (String value : values) {
            BuildMetadata.add(result, resolve(value));
        }
        return result;
    }

    private File resolve(String value) {
        String expanded = value.replace("${project.basedir}", project.getBasedir().getPath())
                .replace("${basedir}", project.getBasedir().getPath())
                .replace("${project.build.directory}", project.getBuild().getDirectory());
        File file = new File(expanded);
        return file.isAbsolute() ? file : new File(project.getBasedir(), expanded);
    }
}
