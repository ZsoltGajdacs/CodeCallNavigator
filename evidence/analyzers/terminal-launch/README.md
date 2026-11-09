# Terminal-launched Gradle suspension — TASK-18

The live development app was waiting in `run_process` for a Gradle wrapper client
that macOS had suspended. Gradle had already exported complete ownership metadata
and printed `BUILD SUCCESSFUL in 5s`.

The app was the foreground group on its controlling terminal. The Gradle client
ran in its own background process group, as required for cancellation, but
inherited fd 0 from that terminal. Gradle's input forwarding attempted a terminal
read, causing `SIGTTIN`. The parent kept waiting for a process that could not exit.

A controlled reproduction using StoryEngine's wrapper and the user's SDKMAN
25.0.2-jbr confirmed:

| Standard input | Result | Time |
|---|---|---:|
| Inherited controlling terminal | Stopped with `SIGTTIN`, metadata already exported | 5.459 s |
| Closed (`/dev/null`) | Exited successfully | 4.232 s |

The fix gives one-shot analyzer processes `Stdio::null()` for stdin. Captured
stdout/stderr, separate process groups, cancellation, deadlines and exit validation
are unchanged. TypeScript's two-phase worker retains its explicit protocol pipe.

The regression launches the actual private Rust process runner in a separate
process under a Python-created controlling PTY. Its analyzer child reads fd 0;
before the fix the controller observed stopped state `T` and failed the test.
After the fix it verifies EOF and non-terminal stdin. Cleanup only targets the
helper session's process groups and reaps the helper.

Verification:

- Terminal regression failed before the fix and passed after it.
- Five process/worker/cancellation tests passed.
- All-target Clippy passed with warnings denied for project code.
- Both modified source files scored CodeScene 10.0; the two-file safeguard passed.
- The fixed **debug** CLI completed full StoryEngine analysis under a controlling
  terminal with SDKMAN 25.0.2-jbr in **42.188 seconds**: 8,136 symbols and 11,112
  relationships. Its graph cache was isolated from the user's workspace.
- Independent review found no remaining issues.

The previous performance measurements used non-terminal stdin and therefore did
not exercise this failure. The original user analysis process was not signalled,
cancelled or replaced during investigation. Restart the app to load the rebuilt
executable; the already-running process cannot pick up this change.

Evidence: [diagnosis](diagnosis.json), [full terminal run](storyengine-terminal.json).
