# Maven build metadata extension

This first-party core extension observes evaluated Maven projects and emits the
module/source/output ownership consumed by CodeCallNavigator's existing bytecode engine.
It performs no call resolution and does not change the Maven build lifecycle.

The JAR contains only CodeCallNavigator classes and its Sisu component index. Maven
3.9.9 and javax.inject 1 are pinned compile-only APIs supplied by the user's
Maven runtime; no Maven implementation or additional third-party runtime library
is bundled. Transitive build dependencies are pinned in `gradle.lockfile`.

Build with the repository's Gradle wrapper and JDK 25:

```sh
fixtures/gradle/gradlew --project-dir tools/maven-metadata jar
```

Load `build/libs/codecallnavigator-maven-metadata.jar` using Maven's
`-Dmaven.ext.class.path` extension mechanism. Set the absolute output paths
`-Dcodecallnavigator.metadata.initial=...` and `-Dcodecallnavigator.metadata.final=...`.
Initial metadata is exported after projects are evaluated; final metadata is
exported after a successful build, including source roots added by generators.
Missing final metadata must be treated as incomplete analysis.

The JSON object contains canonical `modules`, `sourceSets`, `buildInputs`, and
`outputRoots`. Each source set provides module display/ownership, a test flag,
authored/generated source roots, and class directories. Parent-only aggregators
are included in `modules` and input tracking but contribute no source sets.
Optional missing input paths are retained so callers can detect later creation.
Maven's generated-source defaults and explicitly configured compiler generator
directories are classified as generated. Resource roots, filter files, local
parent POMs, settings/toolchains, wrapper/config files, and existing paths in
plugin configuration contribute build inputs. Credentials are never serialized.

Compatibility smoke builds passed with Maven 3.9.9, installed Maven 3.10.0,
and Maven 4.0.0-rc-7 on JDK 25. The pinned distribution SHA-512 values are:

| Distribution | SHA-512 |
| --- | --- |
| `apache-maven-3.9.9-bin.tar.gz` | `a555254d6b53d267965a3404ecb14e53c3827c09c3b94b5678835887ab404556bfaf78dcfe03ba76fa2508649dca8531c74bca4d5846513522404d48e8c4ac8b` |
| `apache-maven-4.0.0-rc-7-bin.tar.gz` | `17b886c0ffb7ec225e514afa8179b3143d855d196cfbb4607ece6272a9b4f51501a389438ab6841b393a4b1391a1b714f07f450daba1951f53f732b400338184` |
