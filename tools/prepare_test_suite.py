#!/usr/bin/env python3
"""Record exact suite selection and ownership before creating build artifacts."""
import argparse
import json
from pathlib import Path

from verification.category_cleanup import initialize_category, reclaim_abandoned
from verification.suites import CATEGORIES, COMPONENTS, SuiteContext, SuiteSelection, execution_steps


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('suite', choices=CATEGORIES)
    parser.add_argument('--component', choices=COMPONENTS, action='append')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--ordinary', action='store_true')
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    context = SuiteContext(root, args.output, SuiteSelection.from_args(args.suite, args))
    _, report = execution_steps(context)
    initialize_category(root, args.output, args.suite)
    reclamation = reclaim_abandoned(root, args.output)
    (args.output / 'reclamation.json').write_text(json.dumps(reclamation, indent=2) + '\n')
    if reclamation['status'] != 'passed':
        raise RuntimeError('Abandoned artifact cleanup could not establish safe reclamation')
    (args.output / 'selection.json').write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
