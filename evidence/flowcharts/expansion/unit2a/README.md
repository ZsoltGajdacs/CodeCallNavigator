# C# execution facts checkpoint

Unit2A passed independent source review and 176 distinct tests: 86 native Flow,
25 engine, 4 explicitly executed ignored engine, 5 release C# integration,
16 shared facts/compatibility, and 40 Python dependency/bootstrap/package contracts.
Two existing Java-resource checks were skipped; full/package/native GUI gates
were not run for this checkpoint. See [independent.json](independent.json).

Fresh old/current analyzer output preserved 12 native nodes, 5 edges and all
11 flow bodies apart from additive execution/invocation metadata. The protected
reference DLL, all 22 Unit2A files, 17 Unit1 files and staged baseline stayed unchanged.
The combined patch exactly matches the upstream checkout, with an exact prior
checkpoint and pinned checksums. Historical canonical flow inventory stays version 1;
fresh C# resource capability is version 2.

Both raw CodeScene review verdicts remain failed for existing findings:
FlowExpressions 9.38 and FlowExport 9.68, identical before and after. Both safeguards
passed; source review found no correctness or maintainability blocker.

The archive preserves original reports/logs/TRX byte for byte. The index records
their sizes and SHA-256 hashes. Test-first failures and intermediate assertion
corrections are disclosed in [verification.json](verification.json).
