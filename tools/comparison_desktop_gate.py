"""A private Git PATH gate can delay the selected base commit of one owned fixture."""
from dataclasses import dataclass, field
import json
import os
from pathlib import Path
import time

from csharp_flow_acceptance import require
from csharp_desktop_acceptance import write_json
from comparison_desktop_cleanup import query_leases, processes, group_alive


def resolution_arguments(project, base_commit):
    return ["--no-pager", "-C", str(project), "rev-parse", "--verify",
            "--end-of-options", f"{base_commit}^{{commit}}"]


def alive(pid):
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False


def known_token(token):
    return len(token) == 32 and all(c in "0123456789abcdef" for c in token)


@dataclass
class Gate:
    project: Path
    directory: Path
    token: str
    base_commit: str
    engine: Path | None = None
    observed: dict | None = field(default=None, init=False)

    def __post_init__(self):
        require(self.project.is_dir() and self.project == self.project.resolve(),
                "Git gate requires its canonical owned project")
        require(self.directory.is_dir() and self.directory == self.directory.resolve(),
                "Git gate requires its canonical owned output directory")
        require(known_token(self.token), "Git gate token must be explicit")
        require(len(self.base_commit) in (40, 64)
                and all(c in "0123456789abcdef" for c in self.base_commit),
                "Git gate requires a full base commit hash")

    def install(self):
        private = self.directory / "path"
        private.mkdir()
        wrapper = private / "git"
        wrapper.write_text(self.script())
        wrapper.chmod(0o755)
        return str(private) + ":/usr/bin:/bin"

    def script(self):
        return f'''#!/usr/bin/python3
import json,os,pathlib,sys,time
arguments=sys.argv[1:]
expected={resolution_arguments(self.project, self.base_commit)!r}
root=pathlib.Path({str(self.directory)!r})
armed=root/'gate-armed.json'
if arguments==expected and armed.exists():
 state=json.loads(armed.read_text())
 if state=={{'token':{self.token!r}}}:
  proof={{'pid':os.getpid(),'parent':os.getppid(),'group':os.getpgrp(),'arguments':arguments,'token':{self.token!r}}}
  temporary=root/'.gate-held.tmp'
  temporary.write_text(json.dumps(proof))
  temporary.replace(root/'gate-held.json')
  while armed.exists():time.sleep(0.01)
os.execv('/usr/bin/git',['git',*arguments])
'''

    def arm(self):
        require(not any((self.directory / name).exists() for name in
                        ("gate-armed.json", "gate-held.json")), "Git gate cannot reuse stale state")
        write_json(self.directory / "gate-armed.json", {"token": self.token})

    def wait(self, process, timeout):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            held = self.directory / "gate-held.json"
            if held.exists():
                proof = json.loads(held.read_text())
                require(proof.get("token") == self.token
                        and proof.get("arguments") == resolution_arguments(self.project, self.base_commit)
                        and type(proof.get("pid")) is int and proof["pid"] > 0,
                        "Git gate proof belongs to another request")
                self.verify_live(proof, process)
                self.observed = proof
                return proof
            require(process.poll() is None, "Native app exited before owned Git gate")
            time.sleep(0.02)
        raise RuntimeError("Owned Git gate did not receive the fixture base commit resolution")

    def verify_live(self, proof, app):
        require(self.engine is not None, "Git gate needs its owned bundled engine identity")
        leases = query_leases(app, self.engine)
        query = next((row for row in leases if row.pid == proof.get("parent")
                      and row.group == proof.get("group")), None)
        require(query is not None, "Gated reader is not a child of the owned query session")
        reader = next((row for row in processes() if row.pid == proof["pid"]), None)
        expected = str(self.directory / "path/git") + " " + " ".join(proof["arguments"])
        require(reader is not None and reader.parent == query.pid and reader.group == query.group
                and reader.command.endswith(expected), "Gated reader argv or process ancestry changed")

    def disarm(self):
        armed = self.directory / "gate-armed.json"
        if armed.exists():
            require(not armed.is_symlink() and json.loads(armed.read_text()) == {"token": self.token},
                    "Git gate ownership changed; refusing disarm")
            armed.unlink()

    def release(self):
        held = self.directory / "gate-held.json"
        require(held.is_file(), "Cannot release a Git gate that was never entered")
        proof = json.loads(held.read_text())
        require(self.observed == proof and proof.get("token") == self.token, "Git gate proof changed")
        require(not alive(proof["pid"]) and not group_alive(proof["group"]),
                "Cancelled owned Git reader or query group is still alive")
        self.disarm()
        return proof
