# Bounded call-target labels

Two new runtime regressions observed clipping in the actual text child: a1456-point
label inside a324-point control. The shared target row now constrains width and
uses middle ellipsis while retaining the complete accessible label, help, disabled
policy and exact follow target. No vendor change was needed.

Independent review and27 distinct release UI tests passed:13 ordinary,14 comparison,
including both new cases in both themes at1050x680 with native titlebar inset.
Tests inspect presented label/control/inspector bounds and execute navigation.
Both CodeScene reports passed three scores of10; safeguard checked90 eligible
files of261 total. Clippy passed with11 existing Rune warnings.

The three source hashes stayed frozen. Final full/package/native verification is
recorded separately; this checkpoint alone does not establish it. Raw reports/logs
are archived byte for byte, with sizes and hashes in the index.
