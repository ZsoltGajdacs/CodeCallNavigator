package codecallnavigator.maven;

import java.io.File;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;
import org.apache.maven.execution.MojoExecutionEvent;
import org.apache.maven.plugin.PluginParameterExpressionEvaluator;
import org.apache.maven.project.MavenProject;
import org.codehaus.plexus.util.xml.Xpp3Dom;

/** Public Maven model/parameter evaluation; no plugin implementation reflection. */
final class MavenCompilerInputs {
    private MavenCompilerInputs() {}

    static CompilerJournal.Inputs collect(MojoExecutionEvent event) throws IOException {
        MavenProject project = event.getProject();
        boolean test = test(event);
        List<String> sources = paths(project, test ? project.getTestCompileSourceRoots() : project.getCompileSourceRoots());
        List<String> classpath;
        try {
            classpath = paths(project, test ? project.getTestClasspathElements() : project.getCompileClasspathElements());
        } catch (org.apache.maven.artifact.DependencyResolutionRequiredException failure) {
            throw new IOException("Unresolved compiler classpath for " + project.getId(), failure);
        }
        return outputs(event, sources, classpath);
    }

    private static CompilerJournal.Inputs outputs(MojoExecutionEvent event, List<String> sources,
            List<String> classpath) throws IOException {
        MavenProject project = event.getProject();
        boolean test = test(event);
        String generatedName = test ? "generatedTestSourcesDirectory" : "generatedSourcesDirectory";
        String generatedDefault = project.getBuild().getDirectory()
                + (test ? "/generated-test-sources/test-annotations" : "/generated-sources/annotations");
        String generated = configuration(event, generatedName, generatedDefault);
        String output = configuration(event, "outputDirectory",
                test ? project.getBuild().getTestOutputDirectory() : project.getBuild().getOutputDirectory());
        return new CompilerJournal.Inputs(sources, classpath, paths(project, List.of(output)),
                paths(project, List.of(generated)));
    }

    static String unsupported(MojoExecutionEvent event, CompilerJournal.Inputs inputs) throws IOException {
        String skip = test(event) ? "skip" : "skipMain";
        if (skipped(event, skip)) {
            return "Native Maven compiler invocation is skipped: " + skip + "=true";
        }
        String compiler = configuration(event, "compilerId", "javac");
        if (!"javac".equals(compiler)) {
            return "Unsupported native Maven compiler identity: " + compiler;
        }
        String filter = sourceFilter(event);
        if (filter != null) {
            return filter;
        }
        MavenProject project = event.getProject();
        String expected = test(event) ? project.getBuild().getTestOutputDirectory() : project.getBuild().getOutputDirectory();
        String actual = new File(inputs.classes().get(0)).getCanonicalPath();
        if (!actual.equals(new File(paths(project, List.of(expected)).get(0)).getCanonicalPath())) {
            return "Compiler output differs from the evaluated Maven project output: " + actual;
        }
        return null;
    }

    private static String sourceFilter(MojoExecutionEvent event) {
        Xpp3Dom configuration = event.getExecution().getConfiguration();
        if (configuration == null) {
            return null;
        }
        List<String> filters = test(event) ? List.of("testIncludes", "testExcludes", "compileSourceRoots", "testPath")
                : List.of("includes", "excludes", "compileSourceRoots", "compilePath");
        for (String name : filters) {
            Xpp3Dom filter = configuration.getChild(name);
            if (customSelection(filter)) {
                return "Custom Maven compiler source selection is unsupported: " + name;
            }
        }
        return null;
    }

    private static boolean customSelection(Xpp3Dom filter) {
        if (filter == null) {
            return false;
        }
        return filter.getChildCount() > 0 || filter.getValue() != null;
    }

    private static boolean test(MojoExecutionEvent event) {
        return "testCompile".equals(event.getExecution().getGoal());
    }

    private record ParameterValue(String expression, String defaultExpression) {}

    private static String configuration(MojoExecutionEvent event, String name, String modelDefault) throws IOException {
        return resolvedPath(evaluatedConfiguration(event, name, modelDefault), name);
    }

    private static boolean skipped(MojoExecutionEvent event, String name) throws IOException {
        // Maven's primitive boolean parameters default to false when the property
        // expression has no value and its descriptor declares no default-value.
        Object value = evaluatedConfiguration(event, name, "false");
        return value != null && Boolean.parseBoolean(value.toString().trim());
    }

    private static Object evaluatedConfiguration(MojoExecutionEvent event, String name, String modelDefault)
            throws IOException {
        ParameterValue parameter = parameter(event, name, modelDefault);
        try {
            return evaluatedParameter(evaluator(event), parameter);
        } catch (org.codehaus.plexus.component.configurator.expression.ExpressionEvaluationException failure) {
            throw new IOException("Cannot evaluate compiler configuration: " + name, failure);
        }
    }

    private static ParameterValue parameter(MojoExecutionEvent event, String name, String modelDefault) {
        Xpp3Dom configuration = event.getExecution().getConfiguration();
        Xpp3Dom value = configuration == null ? null : configuration.getChild(name);
        ParameterValue descriptor = descriptorParameter(event, name);
        if (value == null) {
            return descriptor == null ? new ParameterValue(null, modelDefault) : descriptor;
        }
        String defaultExpression = value.getAttribute("default-value");
        if (defaultExpression == null && descriptor != null) {
            defaultExpression = descriptor.defaultExpression();
        }
        return new ParameterValue(value.getValue(), defaultExpression);
    }

    private static ParameterValue descriptorParameter(MojoExecutionEvent event, String name) {
        var configuration = event.getExecution().getMojoDescriptor().getMojoConfiguration();
        var descriptor = configuration == null ? null : configuration.getChild(name, false);
        if (descriptor == null) {
            return null;
        }
        return new ParameterValue(descriptor.getValue(null), descriptor.getAttribute("default-value", null));
    }

    private static Object evaluatedParameter(PluginParameterExpressionEvaluator evaluator, ParameterValue parameter)
            throws org.codehaus.plexus.component.configurator.expression.ExpressionEvaluationException {
        Object value = parameter.expression() == null ? null : evaluator.evaluate(parameter.expression());
        if (value == null && parameter.defaultExpression() != null) {
            value = evaluator.evaluate(parameter.defaultExpression());
        }
        return value;
    }

    private static PluginParameterExpressionEvaluator evaluator(MojoExecutionEvent event) {
        // afterMojoExecutionSuccess may run after Maven cleared its current project.
        // clone() creates an independent current-project ThreadLocal; never mutate
        // the native session's thread-local evaluation context.
        var evaluationSession = event.getSession().clone();
        evaluationSession.setCurrentProject(event.getProject());
        return new PluginParameterExpressionEvaluator(evaluationSession, event.getExecution());
    }

    private static String resolvedPath(Object evaluated, String name) throws IOException {
        if (evaluated == null) {
            throw new IOException("Compiler configuration evaluated to null: " + name);
        }
        String result = evaluated.toString();
        if (result.contains("${")) {
            throw new IOException("Compiler configuration remains unevaluated: " + name);
        }
        return result;
    }

    private static List<String> paths(MavenProject project, List<String> values) {
        List<String> result = new ArrayList<>();
        for (String value : values) {
            File path = new File(value);
            if (!path.isAbsolute()) {
                path = new File(project.getBasedir(), value);
            }
            result.add(path.toPath().toAbsolutePath().toString());
        }
        return result;
    }
}
