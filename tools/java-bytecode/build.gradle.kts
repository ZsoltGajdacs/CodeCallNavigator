plugins {
    application
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("com.github.adrninistrator:java-callgraph2:4.1.0")
    // The engine pins BCEL 6.10; BCEL 6.13 recognizes Java 25 class files.
    implementation("org.apache.bcel:bcel:6.13.0")
    runtimeOnly("org.slf4j:slf4j-simple:1.7.36")
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(25)
    }
}

application {
    mainClass = "codegraph.bytecode.Analyzer"
    applicationDefaultJvmArgs = listOf(
        "-Xmx8g",
        "--add-exports=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED"
    )
}

dependencyLocking {
    lockAllConfigurations()
}

// Correlation reads javac lowering symbols, confined to the pinned JDK 25 helper.
tasks.withType<JavaCompile>().configureEach {
    options.compilerArgs.addAll(listOf(
        "--add-exports=jdk.compiler/com.sun.tools.javac.code=ALL-UNNAMED",
        "--add-exports=jdk.compiler/com.sun.tools.javac.tree=ALL-UNNAMED"
    ))
}
