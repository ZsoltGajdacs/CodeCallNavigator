# Native visual regression before label correction

The preserved reviewed candidate passed comparison version2 and ordinary C#
version1 native harnesses in both themes after the desktop became active.
The comparison used1050x680 content viewports, actual Drawn frames and owned
window captures. Query cleanup was certified cleared in both themes.

Visual inspection of all six comparison and both ordinary captures nevertheless
found the long target label clipped horizontally in the narrower comparison
inspector. The restored-pair captures show this in both themes. The ordinary
wide target row remained readable. Task22.6 visual acceptance was kept open;
a shared-row regression/fix and final native verification are required.

The archive preserves original reports/configurations/traces/window leases,
cleanup audits and PNGs byte for byte. Runtime harness status is deliberately
unchanged: passing automated checks did not establish visual acceptance.
Its index records sizes and SHA-256 hashes.

The comparison observed readiness-to-Drawn maxima of628.30ms dark and589.67ms
light; these are pre-correction measurements and exclude OS input delivery.
