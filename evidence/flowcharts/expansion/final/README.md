# Final expansion verification

The final mandatory full run, `task22-final-full-r4`, passed all **133 stages**. The independent audit checked its actual result records, four category selections and all 11 corresponding gate stages. The selections contain 1,447 unique inventory registrations, not an asserted actual passing-test count. Raw subprocess/self-test output can repeat test names; it is not counted as extra independent coverage.

All 108 recorded source hashes match before, after and the current files. Compilation, setup, execution and cleanup timings remain separate in `verification.json` and the raw summary. Every checks/category cleanup passed; every removed or absent owned build/tmp/profile path is currently absent. Only the documented newest previous packages remain retained by policy. The separate real private Maven copy was reclaimed (270,438,581 bytes), and its original checkout fingerprint remained unchanged.

The archive retains the first two full failures at Clippy, their two narrow fix/quality checkpoints and the all-target Clippy diagnostic; the third full failure at cache-clear lock contention; and the successful fourth run. Earlier failed runs are not relabeled, and their unexecuted later stages remain not run. The deterministic lock regression red/green evidence is separately indexed in `../lock-lifetime/`.

Both native desktop acceptance gates passed using public compiler fixtures in light and dark themes. Root inspected the captures; this report audits their terminal records and does not claim another native run or independent screenshot inspection.

`reports.tar.gz` contains only raw JSON/log/stderr evidence with per-member SHA256 and byte counts in `archive-index.json`, verified by reading the archive back. Private graph/source/artifact/profile data, binaries and packages are excluded. Final pre-commit checks, task closure and commit remain Root-owned.
