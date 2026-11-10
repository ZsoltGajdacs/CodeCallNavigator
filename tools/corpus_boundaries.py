"""Materialize explicit corpus deployment scopes in the current project config."""
import json
import os


def configure(workload, workloads, project, component_roots):
    if 'http' not in workload:
        return

    def target(name):
        root = component_roots[name]
        return os.path.relpath(root / workloads[name]['targets'][0], project)

    scopes = []
    for scope in workload['http']:
        servers = []
        for server in scope['servers']:
            selected = target(server['component'])
            servers.append({'target': selected, 'application': server['application']}
                           if 'application' in server else selected)
        scopes.append({'client': target(scope['client']), 'servers': servers})
    directory = project / '.codegraph'
    directory.mkdir(exist_ok=True)
    # JSON is also valid YAML; no optional YAML dependency is needed by the harness.
    (directory / 'boundaries.yml').write_text(json.dumps({'version': 1, 'http': scopes}, indent=2) + '\n')
