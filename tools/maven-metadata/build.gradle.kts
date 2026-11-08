plugins { java }

repositories { mavenCentral() }

dependencies {
    // Maven supplies these APIs at runtime. Never package Maven's implementation.
    compileOnly("org.apache.maven:maven-core:3.9.9")
    compileOnly("javax.inject:javax.inject:1")
}

java {
    toolchain { languageVersion = JavaLanguageVersion.of(25) }
}

// Maven 3.9's component scanner predates Java 25 class files.
tasks.compileJava { options.release = 17 }

tasks.jar {
    archiveFileName = "codegraph-maven-metadata.jar"
}

dependencyLocking { lockAllConfigurations() }
