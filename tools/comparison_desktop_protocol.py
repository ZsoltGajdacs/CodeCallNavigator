"""Only these sequenced operations may mutate the helper-owned fixture or capture its window."""
import json
from dataclasses import dataclass
import time

from csharp_flow_acceptance import require
from csharp_desktop_acceptance import capture_image, require_viewport, write_json
from comparison_desktop_fixture import edit_worktree, git, HEAD_REF

ORDER = (("capture", "unavailable-flow"), ("capture", "restored-pair"),
         ("move_after_ref", "after-ref-moved"), ("arm_git_gate", "gate-armed"),
         ("wait_git_gate", "gate-held"), ("release_git_gate", "gate-released"))
WORKTREE_EDIT = ("edit_worktree", "worktree-edited")


def validate(request, expected, fixture, window_id):
    sequence, operation, checkpoint = expected
    required = {"version", "sequence", "operation", "checkpoint", "project"}
    if operation == "capture":
        required |= {"window_id", "comparison_id"}
    require(set(request) == required, "Comparison request has missing or unknown fields")
    require(type(request["version"]) is int and request["version"] == 2
            and type(request["sequence"]) is int and request["sequence"] == sequence,
            "Comparison request has another version or sequence")
    require((request["operation"], request["checkpoint"]) == (operation, checkpoint),
            "Comparison request has another operation/checkpoint")
    require(request["project"] == str(fixture.project), "Comparison operation escaped owned fixture")
    if operation == "capture":
        require(type(request["window_id"]) is int and request["window_id"] == window_id,
                "Comparison capture escaped the owned native window")
        require(request["comparison_id"] == fixture.config["initial"]["comparison_id"],
                "Comparison capture has another binding")


def read_request(path):
    with path.open("rb") as stream:
        raw = stream.read(8193)
    require(len(raw) <= 8192, "Comparison request exceeds 8 KiB")
    value = json.loads(raw)
    require(isinstance(value, dict), "Comparison request must be an object")
    return value


def acknowledge(directory, request, success, error=None):
    value = {key: request[key] for key in ("version", "sequence", "operation", "checkpoint")}
    value["success"] = success
    if error is not None:
        value["error"] = str(error)[:2048]
    write_json(directory / f"external-{request['sequence']:02}-ack.json", value)


@dataclass
class NativeOperations:
    fixture: object
    gate: object
    process: object
    directory: object
    window_id: int
    timeout: int
    order: tuple = ORDER

    def perform(self, request):
        operation = request["operation"]
        if operation == "move_after_ref":
            old = self.fixture.config["initial"]["head_commit"]
            require(git(self.fixture.project, "rev-parse", HEAD_REF) == old, "Owned fixture ref changed unexpectedly")
            git(self.fixture.project, "update-ref", HEAD_REF, self.fixture.moved_commit, old)
            return {"old": old, "new": self.fixture.moved_commit, "reference": HEAD_REF}
        if operation == "arm_git_gate":
            self.gate.arm()
            return {"armed": True}
        if operation == "wait_git_gate":
            return self.gate.wait(self.process, self.timeout)
        if operation == "release_git_gate":
            return self.gate.release()
        if operation == "edit_worktree":
            return edit_worktree(self.fixture.project)
        require(operation == "capture", "Unknown comparison operation")
        return capture_image(self.window_id, self.directory / f"native-{request['checkpoint']}.png")

    def serve(self):
        deadline = time.monotonic() + self.timeout
        operations = []
        for sequence in range(1, len(self.order) + 1):
            request_path = self.directory / f"external-{sequence:02}-request.json"
            while not request_path.exists():
                require(self.process.poll() is None, f"Native app exited before request {sequence}")
                require(not (self.directory / "report.json").exists(), "Native probe failed before all external operations")
                require(time.monotonic() < deadline, "Native comparison operation sequence timed out")
                time.sleep(0.025)
            request = read_request(request_path)
            try:
                validate(request, (sequence, *self.order[sequence - 1]), self.fixture, self.window_id)
                proof = self.perform(request)
            except Exception as error:
                self.failed_request(request, sequence, error)
                raise
            acknowledge(self.directory, request, True)
            operations.append({"sequence": sequence, "operation": request["operation"],
                               "checkpoint": request["checkpoint"], "proof": proof})
        return operations

    def failed_request(self, request, sequence, error):
        # Only a trusted envelope can choose the expected acknowledgment path.
        if request.get("sequence") == sequence and request.get("version") == 2:
            safe = {"version": 2, "sequence": sequence,
                    "operation": self.order[sequence - 1][0], "checkpoint": self.order[sequence - 1][1]}
            acknowledge(self.directory, safe, False, error)
