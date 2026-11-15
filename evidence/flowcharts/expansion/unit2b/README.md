# Java execution facts checkpoint

Unit2B passed independent review and 84 distinct tests: 40 Python, 27 Java engine,
1 release real-compiler normalization/projection and 16 shared compatibility.
Python had no skips. Twelve broader Rust Java cases remain ignored in the normal
unit group; the selected real compiler case ran explicitly. No full/native GUI
or package command was run. See [independent.json](independent.json).

The v9 source helper adds compiler-owned execution/invocation facts. Typed compiler
errors suppress only new facts in affected files; unowned errors suppress all new
facts. Ambiguous javac recovery and valid-neighbor regressions exercise this.
Historical v8 source data and canonical inventory version 1 remain readable; fresh
analysis requires v9 and packaging checks both new helper classes.

Native declarations, call IDs, line/ownership tables and mapped graph/source
projection match the immutable v8 helper. Raw source records differ only in the
header and optional facts. Earlier manifests, staged baseline and helper JAR
hashes stayed unchanged. Both CodeScene versions passed: 18 files scored 10 and
safeguards checked 126 eligible files of 140 total without regressions.

The archive retains original reports/logs, raw source rows and the five compared
native tables from each helper byte for byte. Its index records sizes and hashes.
Compiler binaries and unrelated generated analysis files are not duplicated.
