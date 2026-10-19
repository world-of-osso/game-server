"""Summarize opt-in hazard records; physical travel is not a250ms SLA."""
import argparse
import json
import math
from pathlib import Path

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


def summarize(values):
    ordered = sorted(values)
    if not ordered:
        return {"n": 0, "min": None, "p50": None, "p95": None, "max": None}
    return {"n": len(ordered), "min": ordered[0], "p50": ordered[math.ceil(len(ordered) * 0.5) - 1],
            "p95": ordered[math.ceil(len(ordered) * 0.95) - 1], "max": ordered[-1]}


def new_episode(record, ready_route):
    return {"actor": record["actor"], "field": record["data"]["field"],
            "detection": record, "react_at": record["data"]["react_at"],
            "current_request": None, "ready_route": ready_route,
            "publication": None, "first_move": None, "physical_exit": None,
            "last_position": record["data"].get("position"), "status": "incomplete"}


def violation(episode, record, kind):
    return {"kind": kind, "actor": episode["actor"], "field": episode["field"],
            "simulation_s": record["simulation_s"], "monotonic_ns": record["monotonic_ns"]}


def observe_decision(episode, record):
    problems = []
    data = record["data"]
    at = record["simulation_s"]
    if record["event"] == "hold":
        position = data.get("position")
        if position is not None and episode["last_position"] is not None and position != episode["last_position"]:
            problems.append(violation(episode, record, "hold_displacement"))
        ready = episode["ready_route"]
        if ready is not None and at is not None and at >= episode["react_at"] and data["reason"] in {"reaction", "pending"}:
            problems.append(violation(episode, record, "late_start_after_ready"))
    elif episode["first_move"] is None:
        episode["first_move"] = record
        if at is not None and at < episode["react_at"]:
            problems.append(violation(episode, record, "movement_before_reaction"))
        if episode["ready_route"] is None:
            problems.append(violation(episode, record, "movement_without_current_publication"))
        else:
            episode["publication"] = episode["ready_route"]
    episode["last_position"] = data.get("position")
    return problems


def observe_record(state, record):
    data = record["data"]
    key = (record["actor"], data["field"])
    event = record["event"]
    if event == "detected":
        episode = new_episode(record, state["routes"].get(key))
        state["episodes"].append(episode)
        state["active"][key] = episode
        return
    episode = state["active"].get(key)
    if episode is None:
        state["orphan_events"] += 1
        return
    request_id = data.get("request_id")
    if event == "query_submitted":
        episode["current_request"] = request_id
        episode["ready_route"] = None
    elif event == "query_published":
        if request_id == episode["current_request"]:
            state["publications"][(record["actor"], request_id)] = record
        else:
            state["stale_publications"] += 1
    elif event == "corridor_adopted":
        published = state["publications"].get((record["actor"], request_id))
        if published is None or published["data"]["status"] != "ready" or request_id != episode["current_request"]:
            state["violations"].append(violation(episode, record, "adoption_without_current_publication"))
        else:
            episode["ready_route"] = published
            if episode["first_move"] is None:
                episode["publication"] = published
            state["routes"][key] = published
    elif event in {"hold", "move"}:
        state["violations"].extend(observe_decision(episode, record))
    elif event == "exit":
        if data.get("physical_clear") is True:
            episode["physical_exit"] = record
            episode["status"] = "escaped"
        else:
            episode["status"] = "predicted_exit_without_physical_clear"
        del state["active"][key]
    elif event == "cancel":
        episode["status"] = "cancelled"
        del state["active"][key]
    else:
        state["unknown_events"] += 1


def elapsed(episode, name, clock):
    end = episode[name]
    if end is None or end[clock] is None or episode["detection"][clock] is None:
        return None
    return end[clock] - episode["detection"][clock]


def is_prepublished(episode):
    delay = elapsed(episode, "publication", "monotonic_ns")
    return delay is not None and delay < 0


def distributions(episodes):
    result = {}
    for name in ("publication", "first_move", "physical_exit"):
        clocks = {}
        for clock, unit, scale in (("simulation_s", "simulation_ms", 1000.0), ("monotonic_ns", "wall_ms", 0.000001)):
            eligible = [episode for episode in episodes if name != "publication" or not is_prepublished(episode)]
            values = [elapsed(episode, name, clock) for episode in eligible]
            if any(value is not None and value < 0 for value in values):
                raise ValueError(f"negative {name} latency on {clock}")
            clocks[unit] = summarize([value * scale for value in values if value is not None])
        result[name] = clocks
    return result


def recontacts(episodes):
    prior = {}
    results = []
    for episode in episodes:
        key = (episode["actor"], episode["field"])
        previous = prior.get(key)
        data = episode["detection"]["data"]
        if previous is not None and data.get("position") is not None and data.get("center") is not None:
            center = data["center"]
            before = math.dist(previous["data"]["position"], center)
            after = math.dist(data["position"], center)
            results.append({"actor": key[0], "field": key[1], "radial_delta": after - before,
                            "kind": "growth_recontact" if after >= before else "inward_position_recontact"})
        if episode["physical_exit"] is not None:
            prior[key] = episode["physical_exit"]
    return results


def analyze(records):
    state = {"episodes": [], "active": {}, "publications": {}, "routes": {},
             "violations": [], "orphan_events": 0, "stale_publications": 0, "unknown_events": 0}
    previous = -1
    for record in records:
        if record["monotonic_ns"] < previous:
            raise ValueError("hazard trace monotonic timestamps are out of order")
        previous = record["monotonic_ns"]
        observe_record(state, record)
    episodes = state["episodes"]
    return {"schema": 1, "records": len(records), "episodes": episodes,
            "distributions": distributions(episodes), "quantiles": "nearest-rank",
            "policy_violations": state["violations"], "recontacts": recontacts(episodes),
            "cancelled": sum(episode["status"] == "cancelled" for episode in episodes),
            "incomplete": sum(episode["status"] == "incomplete" for episode in episodes),
            "physical_escapes": sum(episode["physical_exit"] is not None for episode in episodes),
            "worker_correlated_escapes": sum(all(episode[name] is not None for name in
                                                ("publication", "first_move", "physical_exit")) for episode in episodes),
            "prepublished_routes": sum(is_prepublished(episode) for episode in episodes),
            "orphan_events": state["orphan_events"], "stale_publications": state["stale_publications"],
            "unknown_events": state["unknown_events"],
            "scope": "diagnostic observations only; physical travel is not a250ms SLA or stress-capacity proof"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("log", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    with args.log.open() as source:
        result = analyze(read_records(source))
    args.output.write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
