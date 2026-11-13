#!/usr/bin/env python3
"""Measure saved-edit freshness via the public CLI in disposable compiler projects.

Each scenario analyzes a pristine baseline once. Every sample restores its saved
inputs and exact baseline artifact, applies the same edit, verifies staleness, and
refreshes. The first refreshed artifact is checked against a second independent
full analysis; later repetitions compare against that exact reference. Compiler
and inference caches remain warm. Cold means first timed scenario repetition,
not cold dependencies or a clean compiler build.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import time

from freshness_measurement import response, run, summarize
from freshness_scenarios import Project, SCENARIOS, assert_observable_edit


class AnalysisFailure(RuntimeError):
    def __init__(self, observation, events):
        super().__init__(json.dumps({"measurement": observation, "events": events})[-6000:])
        self.observation = observation
        self.events = events

def selected_targets(project, discovered):
    """Select the exact owned build identity; Java discovery names directories."""
    kind = {"csharp": "project", "typescript": "typescript"}.get(project.language, project.language)
    expected = project.root if project.language in ("maven", "gradle") else project.root / project.target
    targets = [target for target in discovered["targets"]
               if target["kind"] == kind and Path(target["path"]).resolve() == expected.resolve()]
    if len(targets) != 1:
        raise RuntimeError(f"Expected one {kind} target at {expected}, found {targets}")
    return targets

class Session:
    """One explicit CLI/project binding for analysis and generation queries."""
    def __init__(self, executable, project, timeout):
        self.executable, self.project, self.timeout = executable, project, timeout
        discovery = response(run([str(executable), "discover", str(project.root)]))
        self.targets = selected_targets(project, discovery)

    def analyze(self):
        observation = run([str(self.executable), "serve-once"], {"command": "analyze", "version": 1,
            "project": str(self.project.root), "roots": self.project.roots, "targets": self.targets,
            "infer_linked_roots": False}, timeout=self.timeout, keep_open=True)
        events = [json.loads(line) for line in observation["stdout"].splitlines()]
        complete = next((event for event in events if event.get("event") == "complete"), None)
        if observation["exit_code"] or complete is None:
            raise AnalysisFailure(observation, events)
        observation["metrics"] = complete["metrics"]
        observation["generation"] = complete["digest"]
        return observation

    def query(self, generation=None):
        request = {"version": 1, "project": str(self.project.root), "roots": self.project.roots, "query": {"kind": "status"}}
        if generation is not None:
            request["expected_generation"] = generation
        return run([str(self.executable), "query"], request)

    def sample(self, scenario, iteration, context):
        self.project.reset(scenario)
        artifact = self.project.root / ".codecallnavigator/graph.json"
        artifact.write_bytes(context["baseline_bytes"])
        saved = time.monotonic()
        self.project.edit(scenario, 0)
        edit_ms = (time.monotonic() - saved) * 1000
        detection = self.query()
        self.assert_stale(detection, scenario)
        refresh = self.analyze()
        candidate = artifact.read_bytes()
        adoption = self.query(refresh["generation"])
        observed = self.adopt(adoption, refresh["generation"])
        save_to_adoption_ms = (time.monotonic() - saved) * 1000
        reference = self.reference(scenario, context)
        equivalent = candidate == context["reference_bytes"]
        if not equivalent:
            directory = Path(tempfile.mkdtemp(prefix="ccn-freshness-mismatch-"))
            (directory / "candidate.json").write_bytes(candidate)
            (directory / "reference.json").write_bytes(context["reference_bytes"])
            raise RuntimeError(f"Full refresh differs from clean-project reference; inspect {directory}")
        observable = assert_observable_edit(scenario, json.loads(context["baseline_bytes"])["graph"], json.loads(candidate)["graph"])
        return {"phase": "cold" if iteration == 0 else "warm", "iteration": iteration,
                "edit_ms": edit_ms, "detection": detection, "refresh": refresh, "adoption": adoption,
                "reference": reference, "reference_kind": "independent clean disposable project; shared dependency caches only", "observable_edit": observable, "reference_sha256": hashlib.sha256(context["reference_bytes"]).hexdigest(),
                "equivalent": equivalent, "prior_generation": context["baseline_generation"],
                "freshness_after": observed["freshness"], "nodes": observed["result"]["nodes"],
                "edges": observed["result"]["edges"], "authored_source_files": sum(1 for path in self.project.source.parent.iterdir() if path.suffix in (".ts", ".java", ".cs")),
                "save_to_adoption_ms": save_to_adoption_ms,
                "unavailable": ["native filesystem event latency", "desktop frame adoption", "separate publication timestamp"]}

    def assert_stale(self, detection, scenario):
        stale = response(detection)
        if stale["freshness"]["input_status"] != "stale":
            raise RuntimeError(f'Saved {scenario} was not stale: {stale["freshness"]}')

    def adopt(self, adoption, generation):
        observed = response(adoption)
        if observed.get("error") or observed["generation"] != generation:
            raise RuntimeError(f"Generation-bound adoption failed: {observed}")
        return observed

    def reference(self, scenario, context):
        if "reference_bytes" in context:
            return None
        with tempfile.TemporaryDirectory(prefix=f"ccn-freshness-{self.project.language}-reference-") as directory:
            root = Path(directory)
            # Gradle project labels deliberately include the checkout basename.
            if self.project.language == "gradle":
                root = root / self.project.root.name
            clean = Project(root, self.project.language, self.project.modules, self.project.repository)
            clean.reset(scenario)
            clean.edit(scenario, 0)
            reference = Session(self.executable, clean, self.timeout).analyze()
            context["reference_bytes"] = (clean.root / ".codecallnavigator/graph.json").read_bytes()
        return reference

    def failure_journey(self):
        self.project.reset()
        complete = self.analyze()
        artifact = self.project.root / ".codecallnavigator/graph.json"
        before = artifact.read_bytes()
        self.project.source.write_text(self.project.source.read_text() + '\nthis is invalid {\n')
        failure = self.invalid_code_failure()
        if artifact.read_bytes() != before:
            raise RuntimeError("Invalid code did not preserve the previous artifact")
        self.assert_generation_mismatch()
        return {"invalid_code_rejected": True, "previous_artifact_preserved": True,
                "generation_mismatch_rejected": True, "generation": complete["generation"], "failure": failure}

    def invalid_code_failure(self):
        try:
            self.analyze()
        except AnalysisFailure as error:
            failed = any(event.get("event") == "failed" for event in error.events)
            if error.observation["timed_out"] or not failed:
                raise RuntimeError("Invalid-code measurement did not complete a reported analysis failure") from error
            return str(error)
        raise RuntimeError("Invalid code was accepted")

    def assert_generation_mismatch(self):
        mismatch = self.query("0" * 64)
        envelope = json.loads(mismatch["stdout"])
        if envelope.get("error", {}).get("code") != "generation_mismatch":
            raise RuntimeError(f"Expected generation mismatch: {envelope}")

def metadata(executable, repository):
    commands = {"node": ["node", "--version"], "java": ["java", "-version"], "dotnet": ["dotnet", "--list-sdks"], "maven": ["mvn", "--version"]}
    versions = {}
    for name, command in commands.items():
        try:
            observation = run(command, timeout=30)
            versions[name] = observation["stdout"] + observation["stderr"]
        except OSError as error:
            versions[name] = str(error)
    return {"platform": platform.platform(), "machine": platform.machine(), "cpu_count": os.cpu_count(),
            "executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
            "analyzer_lock_sha256": hashlib.sha256((repository / "analyzers.lock.json").read_bytes()).hexdigest(),
            "source_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repository, text=True).strip(),
            "toolchains": versions, "dependency_cache": "shared existing toolchain caches; no downloads intentionally cleared",
            "sampling": "50ms process-tree RSS sampling; shared pages can be counted repeatedly; brief peaks may be missed",
            "cpu": "RUSAGE_CHILDREN user+system delta, includes resource sampler subprocesses",
            "detection": "immediate one-shot status after save; excludes polling wait; not a native event measurement",
            "stages": "CLI Complete metrics; nested stage durations must not be added",
            "idle_monitor": "unavailable: no production continuous monitor exists",
            "reset_protocol": "baseline analyzed once per scenario; each sample restores pristine source bytes and exact baseline artifact; identical mutation; full refresh compared against independent clean-project first-sample full reference; candidate build/dependency/inference caches retained"}

class Experiment:
    """Collect independent language/scenario measurements without mixing failures."""
    def __init__(self, arguments, repository, executable):
        self.arguments, self.repository, self.executable = arguments, repository, executable
        self.records, self.journeys = [], {}

    def language(self, language, output):
        with tempfile.TemporaryDirectory(prefix=f"ccn-freshness-{language}-") as directory:
            try:
                project = Project(Path(directory), language, self.arguments.typescript_modules, self.repository)
                project.reset()
                session = Session(self.executable, project, self.arguments.timeout)
                for scenario in self.arguments.scenarios:
                    self.scenario(session, scenario, output)
                self.journeys[language] = session.failure_journey()
            except (OSError, RuntimeError, ValueError) as error:
                self.journeys[language] = {"unavailable": str(error)}

    def scenario(self, session, scenario, output):
        session.project.reset(scenario)
        baseline = session.analyze()
        context = {"baseline_bytes": (session.project.root / ".codecallnavigator/graph.json").read_bytes(), "baseline_generation": baseline["generation"]}
        for iteration in range(self.arguments.runs + 1):
            record = self.sample(session, scenario, iteration, context)
            if iteration == 0:
                record["baseline_measurement"] = baseline
            self.records.append(record)
            output.write(json.dumps(record) + '\n')
            output.flush()
            if "failure" in record:
                break

    def sample(self, session, scenario, iteration, context):
        try:
            record = session.sample(scenario, iteration, context)
        except (OSError, RuntimeError, ValueError) as error:
            record = {"phase": "cold" if iteration == 0 else "warm", "iteration": iteration, "failure": str(error)}
        record.update(language=session.project.language, scenario=scenario)
        print(f'{session.project.language}/{scenario} {iteration}/{self.arguments.runs}: {record.get("failure", "ok")[:250]}', flush=True)
        return record

    def failed(self):
        return any("failure" in record for record in self.records) or any("unavailable" in item for item in self.journeys.values())


def execute(arguments):
    repository = Path(__file__).resolve().parents[1]
    arguments.output.mkdir(parents=True, exist_ok=False)
    executable = arguments.executable.resolve(strict=True)
    info = metadata(executable, repository)
    (arguments.output / "metadata.json").write_text(json.dumps(info, indent=2) + '\n')
    experiment = Experiment(arguments, repository, executable)
    with (arguments.output / "samples.jsonl").open("w") as output:
        for language in arguments.languages:
            experiment.language(language, output)
    result = {"schema": "codecallnavigator.freshness-research.v1", "requested_warm_runs": arguments.runs,
              "summary": summarize(experiment.records), "failure_journeys": experiment.journeys}
    (arguments.output / "summary.json").write_text(json.dumps(result, indent=2) + '\n')
    return int(experiment.failed())

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--typescript-modules", type=Path)
    parser.add_argument("--languages", type=lambda value: value.split(','), default=["typescript", "csharp", "maven", "gradle"])
    parser.add_argument("--scenarios", type=lambda value: value.split(','), default=list(SCENARIOS))
    parser.add_argument("--runs", type=int, default=20)
    parser.add_argument("--timeout", type=float, default=120)
    arguments = parser.parse_args()
    if arguments.runs < 1:
        parser.error("runs must be positive")
    if arguments.timeout <= 0:
        parser.error("timeout must be positive")
    if set(arguments.scenarios) - set(SCENARIOS):
        parser.error("scenarios must be documented names")
    raise SystemExit(execute(arguments))


if __name__ == "__main__":
    main()
