"""Render changed-file verification plans for terminals and automation."""
import json
import shlex


def display_plan(report, form):
    if form == 'json':
        print(json.dumps(report, indent=2, sort_keys=True))
        return
    scope = report['scope']
    if report.get('phase') == 'development':
        scope = f"development (unit tests only, final scope: {scope})"
    print(f"Verification: {scope} (base {report['base']} = {report['resolved_base']})")
    for match in report['matches']:
        print(f"  {match['path']!r}: {', '.join(match['rules'])}")
    for selection in report['selections']:
        print(f"  Required: {selection['category']} / {selection['component']}")
    print('Requirements: ' + (', '.join(report['requirements']) or 'none'))
    for stage in report['stages']:
        desktop = " (requires accessible desktop)" if stage['requires_desktop'] else ""
        print(f"  {stage['name']}{desktop}: {shlex.join(stage['command'])}")
    print('Execute: ' + report['execution_command'])
    if report.get('phase') == 'development':
        display_development(report)
    display_result(report)


def display_result(report):
    if 'result' in report:
        evidence = f"; evidence: {report['evidence']}" if 'evidence' in report else ''
        print(f"Result: {report['result']}{evidence}")


def display_development(report):
    if not report['stages']:
        print('No unit tests selected.')
    deferred = report['deferred_until_final']
    if any(deferred.values()):
        print(f"Deferred until final: {len(deferred['tests'])} tests, {len(deferred['gates'])} gates")
        if deferred['checks']:
            print('  Checks: ' + ', '.join(deferred['checks']))
    print('Final verification: ' + report['final_execution_command'])
