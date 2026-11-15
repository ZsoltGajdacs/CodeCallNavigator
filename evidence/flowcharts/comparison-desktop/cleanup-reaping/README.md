# Cleanup termination and reaping checkpoint

The actual packaged engine reproduced a parent-owned zombie after verified SIGTERM.
The helper's `waitpid` returned `ECHILD`; the group disappeared only after its parent
exited. This establishes the ownership defect in the original cleanup ordering.
The first native attempt did not record process state, so this repro does not prove
that attempt's remaining member was a zombie.

Failure cleanup now records absent or all-zombie groups as terminated, with zombies
explicitly awaiting parent reaping. Stopped or unknown states do not qualify. After
stopping and waiting for the owned app, every leased group (including zombies) and
private query session must disappear before cleanup is certified. Stable signal
identity excludes transient scheduler state only; successful UI Cancel still requires
full reaping while the GUI is alive.

Independent testing passed all 17 Python cases outside the sandbox, with no skips;
all other 92 previously frozen paths were unchanged at that check. Both changed
files score 10 in both CodeScene versions and their safeguards pass. The actual
packaged-engine green audit confirms awaiting-parent-reap becomes cleared only after
parent cleanup and true absence. Independent review approved the change.

The original and green repros, tests, audits and raw quality results are preserved in
`raw-evidence.tar.gz`; `archive-members.json` gives SHA256 values after extraction.
JSON copies are unchanged. The native-testing documentation was then updated to
explain this two-stage contract and the unlocked/active-display prerequisite.
Native comparison/ordinary C# capture gates remain pending desktop availability.
