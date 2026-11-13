# TASK-62 full verification

The full run passed all 92 functional, performance, packaging, integration and
CodeScene stages. Its final cleanup stage returned an inconclusive native lsof
result and safely retained the release directory. The unchanged standard cleanup
command passed on retry after a native lsof inspection returned exit 1 with empty
stdout/stderr (idle). No protection was overridden. The original full command
therefore exited 1; these records document the completed checks and separate
successful operational cleanup, without rewriting the original result.

- [Original stage results](initial-results.json)
- [Original cleanup log](initial-93-cleanup.log)
- [Original cleanup report](initial-cleanup.json)
- [Successful cleanup retry](cleanup-retry.json)
- [CodeScene stage output](codescene.log)

Full command, run outside the macOS sandbox:

```sh
CODECALLNAVIGATOR_RECURRING_BILLING=/Users/zsolt.gajdacs/Projects/Recurring-billing \
CODECALLNAVIGATOR_TS_NODE_MODULES=/Users/zsolt.gajdacs/Zs/Programming/PacificFateMapEditor/node_modules \
CODECALLNAVIGATOR_FRESHNESS_EXECUTABLE=/Users/zsolt.gajdacs/Zs/Programming/CodeGraph/target/freshness/task62-final-frozen/ccnavigator \
./verify-full.sh \
  --profile-source '/Users/zsolt.gajdacs/Library/Application Support/CodeCallNavigator' \
  --timeout 3600 --output target/verification/task62-full
```

Cleanup retry (exit 0):

```sh
python3.13 tools/verification/cleanup.py \
  --root /Users/zsolt.gajdacs/Zs/Programming/CodeGraph \
  --output target/verification/task62-full --mode full
```

The narrow declaration-only CodeScene parser correction was additionally checked
with all 17 tests in `tests/test_verification_quality.py`; both modified Python
files received numeric CodeScene 10 reviews. This verification-only change did
not modify the frozen executables used for publication/idle/acceptance evidence.
