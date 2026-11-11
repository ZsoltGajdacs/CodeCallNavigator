# Third-party software

Dependencies fetched by `tools/bootstrap.py` retain their original licenses in
their distributions. This repository does not replace those notices.

| Dependency | License | Upstream |
| --- | --- | --- |
| CallAtlas, with the checked-in assembly identity serialization patch | MIT | https://github.com/castorhrio/callatlas |
| Rune | MIT | https://github.com/anomalyco/rune |
| Eclipse JDT Language Server | EPL-2.0, with bundled dependency notices | https://github.com/eclipse-jdtls/eclipse.jdt.ls |
| Lombok agent | MIT | https://projectlombok.org/ |
| Gradle wrapper used by the fixture | Apache-2.0 | https://gradle.org/ |
| Graphviz layout dependency | EPL-2.0 and bundled dependency notices | https://graphviz.org/ |
| java-callgraph2 | Apache-2.0 | https://github.com/Adrninistrator/java-callgraph2 |
| Apache Commons BCEL | Apache-2.0 | https://commons.apache.org/proper/commons-bcel/ |
| AviatorScript 5.4.3 (Java analyzer dependency) | LGPL-3.0 | https://github.com/killme2008/aviatorscript/tree/aviator-5.4.3 |
| SLF4J 1.7.36 | MIT | https://github.com/qos-ch/slf4j/tree/v_1.7.36 |
| zip4j 2.11.5 | Apache-2.0 | https://github.com/srikanth-lingala/zip4j/tree/v2.11.5 |
| libltdl, bundled with Graphviz | LGPL-2.1-or-later | https://www.gnu.org/software/libtool/ |

The package script copies upstream license files, Java dependency notices,
NuGet license metadata and the applied CallAtlas patch into the application's
`Contents/Resources/Licenses` directory. JDT and its Lombok agent are retained
for investigation and are not bundled in the production application.

The LGPL libraries are unmodified, separate runtime libraries. Their applicable
licenses permit modification/replacement and reverse engineering for debugging
such modifications; CodeCallNavigator imposes no additional restriction on those rights.
Java JARs remain separate in `Resources/analyzers/java-bytecode/lib`; libltdl is
in the Graphviz lib directory. Re-sign a locally modified app ad hoc if needed.
Supplemental pinned upstream notices and the GPL text accompanying Aviator
are included under `Licenses/supplemental`; upstream source links are above.

## Project-installed TypeScript / React / Next.js

TypeScript analysis loads the selected project's installed TypeScript compiler
(Microsoft, Apache-2.0). React and Next.js packages supply project type information
and framework identity. These project dependencies and Node.js are not redistributed
in CodeCallNavigator; their installed license notices remain with the project/toolchain.
The bundled TypeScript bridge is CodeCallNavigator source and has no npm dependencies.
