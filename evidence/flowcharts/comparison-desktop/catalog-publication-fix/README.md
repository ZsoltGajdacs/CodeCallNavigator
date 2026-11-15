# Native catalog publication regression

The first final native attempt passed dark but failed light after the Changed
case: its observer read the previous catalog between worker settlement and UI
publication. Both runs certified query cleanup. This failed attempt is retained.

A runtime regression reproduced stale-query failure. The opt-in probe now awaits
the requested echoed query and offset0 before retaining its exact-ID assertion.
Matching-response missing-ID still fails. No workspace/client/default behavior
changed. Independent review and22 release cases passed; both CodeScene reports
passed two scores10 and safeguards. Environment failures are preserved.

The complete full115-stage run precedes this probe-only change. Final light99,
official packaging/signature and both native workflows on the final package
passed afterward, recorded in ../final-verification and ../native-final.
