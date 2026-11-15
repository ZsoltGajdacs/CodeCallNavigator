# Native-harness functional checkpoint

Independent review approved the same 94 source/fixture/document hashes recorded in
[freeze.json](freeze.json). Independent tests passed 20 Rust native-probe cases and
14 Python gate/protocol/process cases outside the sandbox, with no ignored tests or
skips. The [independent report](independent-unit4.json) includes a fresh real .NET
fixture preparation, exact source and mutation checks, ref rebinding and process cleanup.

The fixture covers changed/unchanged/added/removed functions, historical missing
source, explicit manual pairing and a recorded project target. Original compiler
artifacts prove Before differs only by the declared Legacy cached-source row, while
After remains compiler-produced. Cached source matches the actual original BOM/CRLF
input bytes. This host normalizes Git source blobs through `core.autocrlf=input`;
that diagnostic does not change the saved-source contract or cause reconstruction
from Git. A later empty commit retains artifact bytes while changing the comparison
binding. Live C# files and the working graph are removed before native boot.

Both raw CodeScene reports pass: 34 scored reviews at 10, one declaration-only module
without a score, and a passing safeguard over 90 eligible source files (94 changed
files total). Scoped Clippy and static checks pass. Existing vendor Rune warnings
remain in the logs. Version 1 native configuration/report behavior is preserved;
ordinary app launches return before probe setup.

Failure regressions cover terminal report writes, canceled acknowledgments, owned
Git/query cleanup, reused/inconclusive identities and macOS signal-zero permission
behavior. Process status now uses checked numeric process-group listings; failed
inspection cannot certify absence or authorize an unverified signal.

`raw-evidence.tar.gz` preserves the original bytes of every archived output.
`archive-members.json` lists their SHA256 values; the producer's original
`evidence-sha256.json` validates its ten bound members after extraction. JSON copies
are unchanged. Readable logs under `logs/` have only end-of-file whitespace normalized
and should not be checked against the original-byte hashes.

This checkpoint does not establish actual native rendering, capture inspection,
packaging or full verification. Those gates are recorded separately after integration.
