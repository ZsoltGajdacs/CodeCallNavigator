# Explicit project-lock lifetime verification

Mandatory full-r3 exposed a second consecutive cache-clear call returning `LockBusy`. The original full results and runtime log are preserved in the archive. This is a bounded prerequisite repair, not a full-verification pass.

Two real-filesystem regression tests retain a cloned locked descriptor across guard drop. Before the fix, both fail immediate reacquisition; the original cache-clear failure also reproduces (57 passed, 3 failed). Later test artifacts are not run after that failing library artifact.

The acquired-only `LockedFile` guard explicitly unlocks on drop. Failed contenders cannot unlock the owner; double-Busy controls, old-clone closure preserving a new owner, immediate reacquisition, and independent lock types remain covered. No retry or broad serialization masks contention.

The green core integration category passes all 111 distinct selected tests with zero failures or ignores. All five stages and owned cleanup pass; source SHA is identical before and after. Timings and cleanup bytes are in `verification.json`. Both raw CodeScene versions score the modified file 10 and pass safeguards.

`category-reports.tar.gz` contains only raw JSON/log/stderr reports for the red/green categories and the bounded original full failure. Its per-member byte counts and SHA256 values are in `category-report-index.json`, verified by reading the archive back. No binaries, packages, or retained build workspaces are archived. The real descriptor proof does not identify which historical fork retained a descriptor; final full/package/native verification remains separately owned by Root.
