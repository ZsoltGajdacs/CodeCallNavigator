#!/usr/bin/env python3
"""Collect project-owned JDT call hierarchy results; never infer call targets.

This is an acceptance harness, not a Java parser. It uses the server's document
symbols and call hierarchy exclusively. An evidence file is emitted on failures
as well as success, and a nonzero exit means the result must not be published as
a complete graph.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
from urllib.parse import unquote, urlparse


SKIP_DIRECTORIES = {".git", ".gradle", ".idea", ".settings", "node_modules", "obj", "bin"}
CALLABLE_KINDS = {6, 9, 12}  # LSP method, constructor, function
EMPTY_CLIENT_RESPONSES = {"client/registerCapability", "client/unregisterCapability",
                          "window/workDoneProgress/create", "workspace/semanticTokens/refresh",
                          "workspace/codeLens/refresh", "workspace/inlayHint/refresh",
                          "workspace/diagnostic/refresh", "window/showMessageRequest"}


def source_files(root: Path):
    for directory, children, files in os.walk(root):
        children[:] = sorted(child for child in children if child not in SKIP_DIRECTORIES)
        for name in sorted(files):
            if name.endswith(".java"):
                yield (Path(directory) / name).resolve()


def project_path(uri: str, root: Path):
    parsed = urlparse(uri)
    if parsed.scheme != "file":
        return None
    path = Path(unquote(parsed.path)).resolve()
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return None


def symbol_id(uri, symbol):
    position = symbol["selectionRange"]["start"]
    return f"{uri}#{position['line']}:{position['character']}:{symbol['kind']}"


class RpcError(RuntimeError):
    pass


def read_headers(stream):
    headers = {}
    while True:
        line = stream.readline()
        if not line:
            raise RpcError("JDT stdout closed")
        if line in (b"\r\n", b"\n"):
            return headers
        key, value = line.decode("ascii").split(":", 1)
        headers[key.lower()] = value.strip()


def read_payload(stream, remaining):
    if remaining < 0 or remaining > 256 * 1024 * 1024:
        raise RpcError("Invalid JDT message size")
    chunks = []
    while remaining:
        chunk = stream.read(remaining)
        if not chunk:
            raise RpcError("JDT stdout closed inside a message")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def read_message(stream):
    headers = read_headers(stream)
    return json.loads(read_payload(stream, int(headers["content-length"])))


@dataclass
class ClientLaunch:
    command: list
    workspace: Path
    settings: dict
    log_path: Path
    deadline: float


class JdtClient:
    def __init__(self, launch: ClientLaunch):
        self.settings = launch.settings
        self.workspace = launch.workspace
        self.deadline = launch.deadline
        self.messages = queue.Queue()
        self.responses = {}
        self.diagnostics = {}
        self.status = []
        self.progress = set()
        self.ready = False
        self.next_id = 0
        self.log = launch.log_path.open("w", encoding="utf-8")
        self.process = subprocess.Popen(launch.command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=self.log, cwd=launch.workspace)
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self):
        try:
            while True:
                self.messages.put(read_message(self.process.stdout))
        except Exception as error:
            self.messages.put(error)

    def send(self, message):
        payload = json.dumps({"jsonrpc": "2.0", **message}).encode("utf-8")
        self.process.stdin.write(f"Content-Length: {len(payload)}\r\n\r\n".encode("ascii") + payload)
        self.process.stdin.flush()

    def notify(self, method, params):
        self.send({"method": method, "params": params})

    def _setting(self, section):
        value = self.settings
        for component in (section or "").split("."):
            if not component:
                continue
            if not isinstance(value, dict):
                return None
            value = value.get(component)
        return value

    def _server_request(self, message):
        method = message["method"]
        params = message.get("params", {})
        if method == "workspace/configuration":
            result = [self._setting(item.get("section")) for item in params.get("items", [])]
        elif method == "workspace/workspaceFolders":
            result = [{"uri": self.workspace.as_uri(), "name": self.workspace.name}]
        elif method in EMPTY_CLIENT_RESPONSES:
            result = None
        elif method == "workspace/applyEdit":
            result = {"applied": False, "failureReason": "CodeGraph does not edit project source"}
        else:
            self.send({"id": message["id"], "error": {"code": -32601,
                       "message": f"Unsupported client request: {method}"}})
            self.status.append({"type": "unsupported-client-request", "method": method})
            return
        self.send({"id": message["id"], "result": result})

    def pump(self, timeout=1.0):
        if time.monotonic() >= self.deadline:
            raise RpcError("Overall JDT extraction deadline exceeded")
        try:
            message = self.messages.get(timeout=max(0.001, min(timeout, self.deadline - time.monotonic())))
        except queue.Empty:
            return
        if isinstance(message, Exception):
            raise message
        if "method" not in message:
            self.responses[message.get("id")] = message
            return
        if "id" in message:
            self._server_request(message)
            return
        self._notification(message)

    def _notification(self, message):
        method = message["method"]
        params = message.get("params", {})
        if method == "textDocument/publishDiagnostics":
            self.diagnostics[params["uri"]] = params.get("diagnostics", [])
        elif method == "language/status":
            self._status_report(params)
        elif method in {"window/logMessage", "window/showMessage"}:
            self._server_log(message)
        elif method == "$/progress":
            self._progress_report(params)

    def _write_log(self, message):
        self.log.write(json.dumps(message) + "\n")
        self.log.flush()

    def _status_report(self, params):
        self.status.append(params)
        self._write_log({"method": "language/status", "params": params})
        if params.get("type") == "ServiceReady":
            self.ready = True

    def _server_log(self, message):
        self._write_log(message)
        params = message.get("params", {})
        if params.get("type") == 1:
            self.status.append({"type": "server-error", "message": params.get("message")})

    def _progress_report(self, params):
        token = str(params.get("token"))
        kind = params.get("value", {}).get("kind")
        if kind == "begin":
            self.progress.add(token)
        elif kind == "end":
            self.progress.discard(token)

    def request(self, method, params, timeout=120):
        self.next_id += 1
        request_id = self.next_id
        self.send({"id": request_id, "method": method, "params": params})
        until = min(self.deadline, time.monotonic() + timeout)
        while request_id not in self.responses:
            if time.monotonic() >= until:
                self.notify("$/cancelRequest", {"id": request_id})
                raise RpcError(f"Timed out: {method}")
            self.pump(min(1, until - time.monotonic()))
        response = self.responses.pop(request_id)
        if "error" in response:
            raise RpcError(f"{method}: {response['error']}")
        return response.get("result")

    def start(self):
        result = self.request("initialize", {
            "processId": os.getpid(), "rootUri": self.workspace.as_uri(),
            "workspaceFolders": [{"uri": self.workspace.as_uri(), "name": self.workspace.name}],
            "capabilities": {
                "window": {"workDoneProgress": True},
                "workspace": {"configuration": True, "workspaceFolders": True},
                "textDocument": {"documentSymbol": {"hierarchicalDocumentSymbolSupport": True},
                                 "callHierarchy": {"dynamicRegistration": False}},
            },
            "initializationOptions": {"settings": self.settings,
                                      "extendedClientCapabilities": {
                                          "progressReportProvider": True}},
            "clientInfo": {"name": "CodeGraph analyzer acceptance", "version": "0.1.0"},
        }, timeout=180)
        if not result or not result.get("capabilities", {}).get("callHierarchyProvider"):
            raise RpcError("JDT did not advertise call hierarchy support")
        self.notify("initialized", {})
        self.notify("workspace/didChangeConfiguration", {"settings": self.settings})
        print("Waiting for JDT Gradle import...", file=sys.stderr)
        while not self.ready:
            self.pump()
        return result

    def close(self):
        try:
            if self.process.poll() is None:
                self.request("shutdown", None, timeout=5)
                self.notify("exit", None)
                self.process.wait(timeout=5)
        except Exception:
            pass
        finally:
            if self.process.poll() is None:
                self.process.terminate()
                try:
                    self.process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait()
            self.log.close()


def collect_symbols(symbols, uri, root, parent=None):
    for symbol in symbols:
        if "selectionRange" not in symbol or "range" not in symbol:
            raise RpcError("JDT returned flat/invalid document symbols despite hierarchical capability")
        key = symbol_id(uri, symbol)
        yield {"id": key, "uri": uri, "file": project_path(uri, root),
               "name": symbol["name"], "detail": symbol.get("detail", ""),
               "kind": symbol["kind"], "range": symbol["range"],
               "selectionRange": symbol["selectionRange"], "parent": parent}
        yield from collect_symbols(symbol.get("children", []), uri, root, key)


class ProjectCollector:
    def __init__(self, client, root, evidence):
        self.client = client
        self.root = root
        self.evidence = evidence
        self.by_uri = {}

    def run(self):
        self.client.start()
        self.build_workspace()
        self.discover_sources()
        self.collect_declarations()
        self.collect_relationships()
        while not self.client.messages.empty():
            self.client.pump(0.01)

    def build_workspace(self):
        # ServiceReady precedes background build completion. Ask JDT explicitly.
        status = self.client.request("java/buildWorkspace", False, timeout=600)
        self.evidence["buildStatus"] = status
        succeeded = status == "SUCCEED" or (type(status) is int and status == 1)
        if not succeeded:
            self.evidence["failures"].append({"stage": "workspace-build", "status": status})
            raise RpcError(f"JDT workspace build did not succeed: {status}")

    def discover_sources(self):
        result = self.client.request("workspace/executeCommand", {
            "command": "java.project.listSourcePaths", "arguments": []})
        if not isinstance(result, dict) or result.get("status") is not True:
            raise RpcError(f"Cannot retrieve JDT-owned source roots: {result}")
        self.evidence["sourceRoots"] = result.get("data", [])
        for entry in self.evidence["sourceRoots"]:
            self.add_source_root(entry)
        self.evidence["filesDiscovered"] = len(self.by_uri)

    def add_source_root(self, entry):
        source_root = Path(entry["path"]).resolve()
        if project_path(source_root.as_uri(), self.root) is None:
            return
        for path in source_files(source_root):
            self.by_uri[path.as_uri()] = (path, entry.get("projectName"))

    def collect_declarations(self):
        files = sorted(value[0] for value in self.by_uri.values())
        for index, path in enumerate(files):
            if index % 50 == 0:
                print(f"Document symbols: {index}/{len(files)}", file=sys.stderr)
            self.collect_file(path.as_uri())

    def collect_file(self, uri):
        symbols = self.client.request("textDocument/documentSymbol", {"textDocument": {"uri": uri}})
        if symbols is None:
            self.evidence["failures"].append({"stage": "document-symbol", "uri": uri,
                                             "reason": "null response"})
            return
        if not symbols:
            self.evidence["emptySymbolFiles"].append(uri)
        for declaration in collect_symbols(symbols, uri, self.root):
            declaration["project"] = self.by_uri[uri][1]
            self.evidence["declarations"].append(declaration)

    def collect_relationships(self):
        callables = [symbol for symbol in self.evidence["declarations"]
                     if symbol["kind"] in CALLABLE_KINDS]
        if not callables:
            raise RpcError("No callable project declarations were collected")
        for index, symbol in enumerate(callables):
            if index % 100 == 0:
                print(f"Call hierarchy: {index}/{len(callables)}", file=sys.stderr)
            self.collect_callable(symbol)

    def collect_callable(self, symbol):
        params = {"textDocument": {"uri": symbol["uri"]},
                  "position": symbol["selectionRange"]["start"]}
        prepared = self.client.request("textDocument/prepareCallHierarchy", params)
        if not prepared:
            self.evidence["failures"].append({"stage": "prepare-call-hierarchy", "id": symbol["id"],
                                             "reason": "null or empty hierarchy"})
            return
        for item in prepared:
            self.collect_outgoing(symbol, item)

    def collect_outgoing(self, symbol, item):
        outgoing = self.client.request("callHierarchy/outgoingCalls", {"item": item})
        if outgoing is None:
            self.evidence["failures"].append({"stage": "outgoing-calls", "id": symbol["id"],
                                             "reason": "null response"})
            return
        for call in outgoing:
            self.add_relationship(symbol, call)

    def add_relationship(self, symbol, call):
        target = call["to"]
        if target["uri"] not in self.by_uri:
            self.evidence["externalRelationshipsExcluded"] += 1
            return
        self.evidence["edges"].append({"from": symbol["id"], "to": symbol_id(target["uri"], target),
                                      "certainty": "analyzer-reported", "target": target,
                                      "fromRanges": call.get("fromRanges", [])})


def extract(client: JdtClient, root: Path, evidence):
    ProjectCollector(client, root, evidence).run()


def parse_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdt", type=Path, required=True, help="Extracted JDT LS distribution")
    parser.add_argument("--jdk", type=Path, required=True, help="JDK 25+ home")
    parser.add_argument("--root", type=Path, required=True, help="Gradle project root")
    parser.add_argument("--workspace", type=Path, required=True, help="Private JDT workspace cache")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--timeout", type=int, default=1800, help="Overall deadline in seconds")
    parser.add_argument("--gradle-home", type=Path, help="Existing Gradle installation for builds without wrappers")
    parser.add_argument("--lombok-agent", type=Path, help="Existing Lombok jar for JDT generated-member support")
    args = parser.parse_args()
    root, jdt = args.root.resolve(), args.jdt.resolve()
    launcher = sorted((jdt / "plugins").glob("org.eclipse.equinox.launcher_*.jar"))
    if len(launcher) != 1:
        parser.error("JDT distribution must contain exactly one Equinox launcher jar")
    if not (args.jdk / "bin" / "javac").is_file():
        parser.error("--jdk must name a full JDK containing bin/javac")
    if args.gradle_home and not (args.gradle_home / "bin" / "gradle").is_file():
        parser.error("--gradle-home must contain bin/gradle")
    if args.lombok_agent and not args.lombok_agent.is_file():
        parser.error("--lombok-agent must point to an existing Lombok jar")
    return args


def prepare_directories(args):
    args.workspace.mkdir(parents=True, exist_ok=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)


def analyzer_settings(args):
    settings = {"java": {
        "import": {"mode": "full", "maven": {"enabled": False},
                   "gradle": {"enabled": True, "wrapper": {"enabled": True},
                              "java": {"home": str(args.jdk.resolve())},
                              "annotationProcessing": {"enabled": True}}},
        "symbols": {"includeGeneratedCode": True, "includeSourceMethodDeclarations": True},
        "configuration": {"updateBuildConfiguration": "automatic"},
        "autobuild": {"enabled": True},
    }}
    if args.gradle_home:
        settings["java"]["import"]["gradle"]["home"] = str(args.gradle_home.resolve())
    return settings


def launch_command(args):
    jdt = args.jdt.resolve()
    launcher = sorted((jdt / "plugins").glob("org.eclipse.equinox.launcher_*.jar"))
    config = "config_mac_arm" if (jdt / "config_mac_arm").is_dir() else "config_mac"
    command = [str(args.jdk.resolve() / "bin" / "java"), "-Xmx8G",
               "-Declipse.application=org.eclipse.jdt.ls.core.id1", "-Dosgi.bundles.defaultStartLevel=4",
               "-Declipse.product=org.eclipse.jdt.ls.core.product", "-Dlog.level=INFO",
               "--add-modules=ALL-SYSTEM", "--add-opens", "java.base/java.util=ALL-UNNAMED",
               "--add-opens", "java.base/java.lang=ALL-UNNAMED", "-jar", str(launcher[0]),
               "-configuration", str(jdt / config), "-data", str(args.workspace.resolve())]
    if args.lombok_agent:
        command.insert(1, "-javaagent:" + str(args.lombok_agent.resolve()))
    return command


def new_evidence(root):
    return {"analyzer": "Eclipse JDT LS", "root": str(root), "complete": False,
                "declarations": [], "edges": [], "failures": [], "emptySymbolFiles": [],
                "externalRelationshipsExcluded": 0,
                "limitations": ["No DI, reflection or framework callback inference.",
                                "JDT call hierarchy does not classify certainty or callback edges.",
                                "Standalone initializer blocks are not enumerated by document symbols."]}


def finish_evidence(evidence, start):
    evidence["elapsedSeconds"] = round(time.monotonic() - start, 3)
    server_errors = [status for status in evidence.get("serverStatus", [])
                     if status.get("type") == "server-error"]
    evidence["serverErrorCount"] = len(server_errors)
    errors = sum(d.get("severity") == 1 for values in evidence.get("diagnostics", {}).values()
                 for d in values)
    evidence["errorDiagnosticCount"] = errors
    evidence["complete"] = (bool(evidence["declarations"]) and not evidence["failures"]
                            and errors == 0 and not server_errors)


def run_extraction(args, evidence):
    client = None
    start = time.monotonic()
    launch = ClientLaunch(launch_command(args), args.root.resolve(), analyzer_settings(args),
                          args.output.with_suffix(".log"), start + args.timeout)
    try:
        client = JdtClient(launch)
        extract(client, launch.workspace, evidence)
    except Exception as error:
        evidence["failures"].append({"stage": "extraction", "reason": str(error)})
    finally:
        collect_client_evidence(client, evidence)
        finish_evidence(evidence, start)
        args.output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")


def collect_client_evidence(client, evidence):
    if client:
        evidence["diagnostics"] = client.diagnostics
        evidence["serverStatus"] = client.status
        client.close()


def main():
    args = parse_arguments()
    prepare_directories(args)
    evidence = new_evidence(args.root.resolve())
    run_extraction(args, evidence)
    print(json.dumps({"complete": evidence["complete"], "declarations": len(evidence["declarations"]),
                      "edges": len(evidence["edges"]), "failures": len(evidence["failures"]),
                      "errors": evidence["errorDiagnosticCount"]}), file=sys.stderr)
    return 0 if evidence["complete"] else 1


if __name__ == "__main__":
    sys.exit(main())
