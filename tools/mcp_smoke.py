"""Exercise the installed MCP over real stdio using only its public protocol."""
import json
from pathlib import Path
import queue
import subprocess
import threading


def output_lines(stream, output):
    for line in stream:
        output.put(line)
    output.put(None)


def response(output, identity):
    while True:
        value = read_message(output)
        if value.get('id') == identity:
            break
    if 'error' in value:
        raise RuntimeError(f'Installed MCP rejected request: {value["error"]}')
    return value['result']


def read_message(output):
    line = output.get(timeout=20)
    if line is None:
        raise RuntimeError('Installed MCP closed stdout before its response')
    return json.loads(line)


def send(process, method, identity=None, params=None):
    value = dict(jsonrpc='2.0', method=method)
    if identity is not None:
        value['id'] = identity
    if params is not None:
        value['params'] = params
    process.stdin.write(json.dumps(value) + '\n')
    process.stdin.flush()


def smoke(binary, engine, project):
    Path(project).mkdir()
    output = queue.Queue()
    with subprocess.Popen([str(binary), '--engine', str(engine), '--project', str(project)],
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True) as process:
        threading.Thread(target=output_lines, args=(process.stdout, output), daemon=True).start()
        try:
            exchange(process, output)
            process.stdin.close()
            if process.wait(timeout=20):
                raise RuntimeError('Installed MCP exited unsuccessfully')
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


def exchange(process, output):
    send(process, 'initialize', 1, dict(protocolVersion='2025-11-25', capabilities={},
                                      clientInfo=dict(name='package-acceptance', version='1')))
    response(output, 1)
    send(process, 'notifications/initialized')
    send(process, 'tools/list', 2, {})
    tools = {tool['name'] for tool in response(output, 2)['tools']}
    if not {'graph_status', 'refresh_graph', 'function_flow'} <= tools:
        raise RuntimeError('Installed MCP omitted required tools')
    send(process, 'tools/call', 3, dict(name='graph_status', arguments={}))
    result = response(output, 3)
    envelope = result.get('structuredContent', {})
    if not result.get('isError') or envelope.get('error', {}).get('code') != 'graph_missing':
        raise RuntimeError(f'Installed MCP did not preserve the real CLI error: {result}')
