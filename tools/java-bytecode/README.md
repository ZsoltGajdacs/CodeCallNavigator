# Existing Java bytecode analyzer adapter

This driver configures `java-callgraph2` 4.1.0 and respects its boolean result.
It adds no call-resolution, DI, framework or source-parsing logic. BCEL is pinned
to 6.13.0 to support Java 25 class files. Both raw and engine-reported actual
targets are retained in the engine output.
The engine's default equals/hashCode omission is disabled using its existing
`ECE_PARSE_IGNORE_METHOD` configuration so project overrides remain visible.

Build with an existing Gradle 9.4 installation and JDK 25:

```sh
gradle --project-dir tools/java-bytecode installDist --write-locks
```

Run after the target project's own build succeeds:

```sh
tools/java-bytecode/build/install/codegraph-bytecode/bin/codegraph-bytecode \
  /absolute/path/to/analysis-output 25 /absolute/path/to/project/build/classes/java/main
```

Additional class directories or jars may be passed as additional arguments.
Only pass selected project outputs for the project-only acceptance run. Include
test outputs explicitly when testing those, and track the input-to-project
mapping outside the analyzer.

The analyzer emits its documented tab-separated text files in a child directory
of the output root. `analysis-status.properties` records that directory, engine
success, adapter completeness and any frame snapshot limit. Exit 0 means the engine returned success
with method declarations and without reporting that limit. Exit 1 means engine failure or empty output, 2 means invalid inputs
or an exception, and 3 means the engine reported limited analysis.

Retain all rows from `method_info.txt` to preserve disconnected declarations;
use `method_line_number.txt` for available line ranges and `method_call.txt` for
reported relationships. Preserve call type and enabled flag. Bytecode does not
provide precise source columns; abstract or generated declarations may lack
line information. Class ownership follows analyzer input provenance, not package
prefixes. No runtime target is inferred by the adapter.

Two adapter metadata files are written directly under the output root using
BCEL's existing `SourceFile`, method flags, annotations and line-table attributes:

- `class_metadata.tsv`: input index (one-based), input path, class entry, class
  name, SourceFile attribute, class flags, annotation descriptors.
- `method_metadata.tsv`: input index, input path, class entry, class name,
  SourceFile attribute, engine-style method signature, return type, method flags,
  minimum line, maximum line, annotation descriptors.

Both have a header row. Backslash, tab, newline and carriage return in fields
are escaped as `\\`, `\t`, `\n`, `\r`. No line table is represented by zero line
numbers. Missing SourceFile attributes retain BCEL's unknown value; the adapter
does not invent file locations. Method identity includes return type when joining
to engine output, preserving bridge methods that share a name and argument list.
Annotation descriptors include class-retained generator markers where present.

Upstream output documentation:

- https://github.com/Adrninistrator/java-callgraph2/blob/main/docs/file_format.md
- https://github.com/Adrninistrator/java-callgraph2/blob/main/docs/call_type.md
- https://commons.apache.org/proper/commons-bcel/changes.html
