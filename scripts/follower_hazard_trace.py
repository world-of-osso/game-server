"""Summarize opt-in follower hazard records; empty input has no latency evidence."""
import json

PREFIX = "FOLLOWER_HAZARD_TRACE "


def read_records(lines):
    records = []
    for number, line in enumerate(lines, 1):
        if PREFIX not in line:
            continue
        try:
            record = json.loads(line.split(PREFIX, 1)[1])
        except json.JSONDecodeError as error:
            raise ValueError(f"hazard trace line {number}: {error}") from error
        if record.get("schema") != 1:
            raise ValueError(f"hazard trace line {number}: unsupported schema")
        records.append(record)
    return records


def analyze(records):
    empty = {"n": 0, "min": None, "p50": None, "p95": None, "max": None}
    distributions = {name: {"simulation_ms": dict(empty), "wall_ms": dict(empty)}
                     for name in ("publication", "first_move", "physical_exit")}
    return {"schema": 1, "episodes": [], "distributions": distributions,
            "policy_violations": [], "cancelled": 0}
