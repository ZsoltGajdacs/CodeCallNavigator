"""Evaluate actual native camera presentation cadence independently of CPU submissions."""
import json

DRAG_SECONDS = 10.0
INTERVAL_BUDGET_MS = 1000.0 / 30.0
PREFIX = "codecallnavigator."


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(len(ordered) * fraction))] if ordered else None


def distribution(values):
    return {"samples": len(values), "median": percentile(values, 0.5),
            "p95": percentile(values, 0.95), "max": max(values) if values else None}


def read_rows(path):
    with path.open() as stream:
        return [json.loads(line) for line in stream if line.strip()]


def update_channels(row, names, values):
    for channel in row.get("cm", []):
        names[channel["id"]] = channel["name"]
    values.update(row.get("c", []))
    for key in row.get("cg", []):
        values.pop(key, None)


def channel_frames(rows):
    names, values = {}, {}
    for row in rows:
        update_channels(row, names, values)
        if "t" in row:
            channels = {names[key].removeprefix(PREFIX): value for key, value in values.items()
                        if names.get(key, "").startswith(PREFIX)}
            yield row, channels


def distinct_presentations(frames):
    distinct, previous = [], None
    for row, channels in frames:
        if "present_ms" not in row:
            continue
        camera = tuple(channels.get(name) for name in ("camera.x", "camera.y", "camera.zoom"))
        changed = camera != previous and None not in camera
        if changed:
            distinct.append((row["present_ms"], channels.get("drag_seconds", -1)))
        previous = camera
    return distinct


def drag_result(index, frames):
    distinct = distinct_presentations(frames)
    intervals = [b[0] - a[0] for a, b in zip(distinct, distinct[1:])]
    cadence = distribution(intervals)
    coverage = covers_drag(distinct)
    presented_seconds = (distinct[-1][0] - distinct[0][0]) / 1000.0 if len(distinct) > 1 else 0.0
    fps = (len(distinct) - 1) / presented_seconds if presented_seconds > 0 else 0.0
    input_window_fps = len(distinct) / DRAG_SECONDS
    ordered = all(interval > 0 for interval in intervals)
    return {"drag": index, "seconds": DRAG_SECONDS, "distinct_camera_presentations": len(distinct),
            "fps": fps, "input_window_fps": input_window_fps,
            "presented_seconds": presented_seconds,
            "full_drag_covered": coverage, "present_interval_ms": cadence,
            "passed": meets_budget(coverage, ordered, (fps, input_window_fps), cadence["p95"])}


def meets_budget(coverage, ordered, rates, p95):
    rates_pass = min(rates) >= 30.0
    return coverage and ordered and rates_pass and within_budget(p95)


def covers_drag(distinct):
    return bool(distinct) and distinct[0][1] <= 0.1 and distinct[-1][1] >= 9.9


def within_budget(p95):
    # Native trace timestamps are rounded to 0.001ms; allow only their rounding error.
    return p95 is not None and 0 < p95 <= INTERVAL_BUDGET_MS + 0.002


def validate_header(header):
    if header.get("clock") != "wall":
        return {"status": "invalid", "reason": "FPS acceptance requires a native wall-clock trace, never headless --fps."}
    return None


def native_surface(frames):
    active = [channels for _, channels in frames if channels.get("drag", -1) >= 0]
    surface = {}
    for name in ["backing_scale", "viewport.width", "viewport.height", "target.width", "target.height"]:
        values = {channels.get(name) for channels in active}
        if len(values) != 1 or None in values:
            return None
        surface[name] = values.pop()
    return surface


def validate_surface(surface, expected_scale):
    if surface is None:
        return {"status": "invalid", "reason": "Live native backing scale/viewport samples are missing or changed during the drags."}
    scale = surface["backing_scale"]
    if expected_scale is not None and scale != expected_scale:
        return {"status": "invalid", "reason": f"Native backing scale is {scale}, expected {expected_scale}. --scale cannot change the monitor backing scale."}
    if not target_matches_surface(surface):
        return {"status": "invalid", "reason": "Native target pixel size disagrees with the reported viewport and backing scale."}
    return None


def target_matches_surface(surface):
    for axis in ["width", "height"]:
        expected = surface["viewport." + axis] * surface["backing_scale"]
        if abs(surface["target." + axis] - expected) > 1.0:
            return False
    return True


def cpu_values(rows, name):
    return [row[name] for row in rows if row.get(name) is not None]


def queue_waits(rows):
    timed = [row for row in rows if row.get("frame_started_ms") is not None]
    return [row["frame_started_ms"] - row["synthetic_request_ms"] for row in timed
            if row.get("synthetic_request_ms") is not None]


def gpu_samples(frames):
    gpu = {}
    for _, channels in frames:
        if valid_gpu_sample(channels):
            gpu[channels["gpu_sample"]] = channels["gpu_ms"]
    return gpu


def valid_gpu_sample(channels):
    return "gpu_sample" in channels and channels.get("gpu_ms", 0) > 0


def timing_summary(directory, frames):
    path = directory / "encoded.jsonl"
    encoded = read_rows(path) if path.is_file() else []
    active = {channels.get("sample") for _, channels in frames if channels.get("drag", -1) >= 0}
    encoded = [row for row in encoded if row.get("sample") in active]
    gpu = gpu_samples(frames)
    return {"update_ms": distribution(cpu_values(encoded, "update_ms")),
            "encode_ms": distribution(cpu_values(encoded, "encode_ms")),
            "synthetic_queue_wait_ms": distribution(queue_waits(encoded)),
            "gpu_draw_ms": distribution([value for sample, value in gpu.items() if sample in active]),
            "gpu_invalid_samples": max((channels.get("gpu_invalid_samples", 0) for _, channels in frames), default=0)}


def availability(result, frames):
    result["unavailable_metrics"] = ["OS input-arrival latency and drawable-acquisition waits are not exposed by Rune's public hooks."]
    if not any("present_ms" in row for row, _ in frames):
        result.update(status="unavailable", reason="Native trace contains no actual Metal presentation timestamps.")
    if result["gpu_draw_ms"]["samples"] == 0:
        result["unavailable_metrics"].append("GPU timestamp samples were unsupported or unavailable.")
    if any(channels.get("gpu_map_failed") for _, channels in frames):
        result["unavailable_metrics"].append("At least one asynchronous GPU timestamp readback failed.")
    if result["gpu_invalid_samples"]:
        result["unavailable_metrics"].append("Zero or reversed GPU timestamp pairs were rejected; GPU timings are incomplete and cannot establish a whole-run GPU distribution.")


def summarize(directory, expected_scale=None):
    trace = directory / "native.jsonl"
    if not trace.is_file():
        return {"status": "unavailable", "reason": "No native trace was produced; GPU/window creation may have failed."}
    rows = read_rows(trace)
    header = next((row for row in rows if row.get("type") == "header"), {})
    error = validate_header(header)
    if error:
        return error
    frames = list(channel_frames(rows))
    surface = native_surface(frames)
    error = validate_surface(surface, expected_scale)
    if error:
        return error
    drags = [drag_result(index, [(row, channels) for row, channels in frames if channels.get("drag") == index]) for index in range(3)]
    result = {"status": "passed" if all(drag["passed"] for drag in drags) else "failed",
              "native_scale": surface["backing_scale"],
              "native_size_points": [surface["viewport.width"], surface["viewport.height"]],
              "native_target_pixels": [surface["target.width"], surface["target.height"]],
              "startup_header": header,
              "input_model": "Synthetic pointer events queued through production Runtime drag handlers; OS mouse delivery is not measured.",
              "drags": drags}
    result.update(timing_summary(directory, frames))
    availability(result, frames)
    return result
