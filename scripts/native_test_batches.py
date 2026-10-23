#!/usr/bin/env python3
"""Run a Cargo libtest inventory in fresh module processes (two processes, three threads each).

Build through the normal project helper first. Pass its Cargo JSON artifact receipt;
this runner only executes test binaries and never changes compilation settings.
The caller owns admission, host safety floors and immutable-input checks.
"""
import argparse
from collections import Counter, defaultdict, deque
import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import time

FILTERS = (
    "followers::", "maps::", "pull_sources", "ai::", "kit_area", "creature_ai",
    "combat::", "scripted_ai::", "spell_cast::", "dungeon_simulator::", "utility",
    "interrupt", "dispel", "healer", "resurrection", "positioning", "navigation",
    "hazard", "followers::kit", "rotation_fidelity", "kit_overrides", "overrides",
    "warlock", "demon", "guardian", "imp", "tyrant", "summon",
)
SKIP = "opening_admission_does_not_recruit_unprojected_source_packs"
PROCESSES = 2
THREADS = 3
TIMER = """import json,resource,subprocess,sys,time
start=time.monotonic()
result=subprocess.run(sys.argv[2:])
usage=resource.getrusage(resource.RUSAGE_CHILDREN)
row={'wall_seconds':time.monotonic()-start,'cpu_seconds':usage.ru_utime+usage.ru_stime,'max_rss_kib':usage.ru_maxrss,'exit_code':result.returncode}
open(sys.argv[1],'w').write(json.dumps(row))
sys.exit(result.returncode)
"""


def read_events(path):
    return [json.loads(line) for line in path.read_text().splitlines() if line.startswith("{")]


def read_binaries(artifacts):
    binaries = {}
    for row in read_events(artifacts):
        if row.get("reason") == "compiler-artifact" and row.get("executable") and row["profile"]["test"]:
            binaries[row["executable"]] = str(Path(row["manifest_path"]).parent)
    if not binaries:
        raise ValueError("Cargo artifact receipt contains no test binaries")
    return binaries


def discover_groups(binaries, filters, skips, output, env):
    groups = defaultdict(list)
    expected = Counter()
    for binary, cwd in binaries.items():
        tag = hashlib.sha256(binary.encode()).hexdigest()[:12]
        command = [binary, *filters, "--list", "-Z", "unstable-options", "--format", "json"]
        for skip in skips:
            command.extend(["--skip", skip])
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, check=True)
        path = output / (tag + "-discovery.jsonl")
        path.write_text(result.stdout)
        (output / (tag + "-discovery.log")).write_text(result.stderr)
        for row in read_events(path):
            if row.get("type") == "test" and row.get("event") == "discovered":
                name = row["name"]
                groups[(binary, name.split("::")[0])].append(name)
                expected[(binary, name, row["ignore"])] += 1
    if not expected:
        raise ValueError("filters select no tests")
    plan = []
    for (binary, module), names in groups.items():
        tag = hashlib.sha256((binary + "\0" + module).encode()).hexdigest()[:12]
        plan.append({"binary": binary, "cwd": binaries[binary], "module": module, "tests": names, "tag": tag})
    plan.sort(key=lambda group: -len(group["tests"]))
    (output / "plan.json").write_text(json.dumps(plan, indent=2))
    return plan, expected


def launch_group(group, output, env):
    tag = group["tag"]
    command = [group["binary"], *group["tests"], "--exact", "--test-threads", str(THREADS),
               "-Z", "unstable-options", "--format", "json", "--report-time"]
    phase_env = dict(env, DUNGEON_SIMULATOR_OUTPUT=str(output / (tag + "-reports")))
    with (output / (tag + ".jsonl")).open("w") as stdout, (output / (tag + ".log")).open("w") as stderr:
        process = subprocess.Popen(
            [sys.executable, "-c", TIMER, str(output / (tag + ".time")), *command],
            cwd=group["cwd"], env=phase_env, stdout=stdout, stderr=stderr, start_new_session=True,
        )
    return {"group": group, "process": process}


def finish_group(job, output):
    group = job["group"]
    timing = json.loads((output / (group["tag"] + ".time")).read_text())
    events = [row for row in read_events(output / (group["tag"] + ".jsonl"))
              if row.get("type") == "test" and row.get("event") in ["ok", "failed", "ignored"]]
    row = {"module": group["module"], "binary": group["binary"], "tag": group["tag"],
           "cases": len(events), **timing}
    return row, events


def sample_memory(members, started):
    rss = swap = 0
    for pid in members.read_text().split():
        try:
            fields = Path("/proc", pid, "status").read_text().splitlines()
            rss += sum(int(line.split()[1]) for line in fields if line.startswith("VmRSS:"))
            swap += sum(int(line.split()[1]) for line in fields if line.startswith("VmSwap:"))
        except (FileNotFoundError, ProcessLookupError):
            continue
    available = int(next(line.split()[1] for line in Path("/proc/meminfo").read_text().splitlines()
                         if line.startswith("MemAvailable:")))
    return {"unix_seconds": time.time(), "elapsed_seconds": time.monotonic() - started,
            "owned_rss_kib": rss, "owned_swap_kib": swap, "host_available_kib": available}


def stop_groups(active):
    for job in active:
        process = job["process"]
        if process.poll() is None:
            try:
                os.killpg(process.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass
    for job in active:
        job["process"].wait()


def sample_and_write_memory(members, started, output, timeline, peaks):
    sample = sample_memory(members, started)
    timeline.write(json.dumps(sample) + "\n")
    peaks["peak_tree_rss_kib"] = max(peaks["peak_tree_rss_kib"], sample["owned_rss_kib"])
    peaks["peak_tree_swap_kib"] = max(peaks["peak_tree_swap_kib"], sample["owned_swap_kib"])
    peaks["peak_tree_rss_plus_swap_kib"] = max(
        peaks["peak_tree_rss_plus_swap_kib"], sample["owned_rss_kib"] + sample["owned_swap_kib"])
    peaks["min_host_available_kib"] = min(peaks["min_host_available_kib"], sample["host_available_kib"])
    (output / "memory.json").write_text(json.dumps(peaks, indent=2))


def read_finished_groups(active, output):
    remaining, rows, events = [], [], []
    for job in active:
        if job["process"].poll() is None:
            remaining.append(job)
            continue
        row, completed = finish_group(job, output)
        rows.append(row)
        events.extend((job["group"]["binary"], event) for event in completed)
    return remaining, rows, events


def execute_groups(plan, output, env):
    pending = deque(plan)
    active = []
    rows = []
    events = []
    started = time.monotonic()
    group = Path("/proc/self/cgroup").read_text().split("::", 1)[1].strip()
    members = Path("/sys/fs/cgroup") / group.lstrip("/") / "cgroup.procs"
    peaks = {"peak_tree_rss_kib": 0, "peak_tree_swap_kib": 0,
             "peak_tree_rss_plus_swap_kib": 0, "min_host_available_kib": 2**63}
    try:
        with (output / "host-memory.jsonl").open("w", buffering=1) as timeline:
            while pending or active:
                while pending and len(active) < PROCESSES:
                    active.append(launch_group(pending.popleft(), output, env))
                sample_and_write_memory(members, started, output, timeline, peaks)
                remaining, finished, completed = read_finished_groups(active, output)
                active = remaining
                rows.extend(finished)
                events.extend(completed)
                if finished:
                    (output / "groups.json").write_text(json.dumps(rows, indent=2))
                if active:
                    time.sleep(0.1)
    finally:
        stop_groups(active)
    return rows, events, peaks, time.monotonic() - started


def serialize_missing(counter):
    return [{"binary": binary, "test": name, "ignored": ignored}
            for binary, name, ignored in counter.elements()]


def main():
    started = time.monotonic()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--skip", action="append", default=[])
    parser.add_argument("--full-suite", action="store_true", help="all discovered tests; no implicit union filters or skip")
    parser.add_argument("--known-failure", action="append", default=[], metavar="EXACT_NAME", help="label, never suppress, an expected failing identity")
    parser.add_argument("filters", nargs="*")
    args = parser.parse_args()
    if args.full_suite and args.filters:
        parser.error("--full-suite cannot be combined with test filters")
    filters = args.filters or (() if args.full_suite else FILTERS)
    skips = args.skip if args.full_suite else [SKIP, *args.skip]
    args.output.mkdir(parents=True)
    env = dict(os.environ, RUSTC_BOOTSTRAP="1")
    binaries = read_binaries(args.artifacts)
    plan, expected = discover_groups(binaries, filters, skips, args.output, env)
    rows, events, peaks, execution_wall = execute_groups(plan, args.output, env)
    observed = Counter((binary, event["name"], event["event"] == "ignored") for binary, event in events)
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    failures = {event["name"] for _, event in events if event["event"] == "failed"}
    known = set(args.known_failure)
    summary = {"passed": sum(event["event"] == "ok" for _, event in events),
               "failed": sum(event["event"] == "failed" for _, event in events),
               "ignored": sum(event["event"] == "ignored" for _, event in events),
               "known_failures": sorted(failures & known),
               "unexpected_failures": sorted(failures - known),
               "full_suite": args.full_suite, "filters": list(filters), "skips": skips,
               "missing": serialize_missing(expected - observed),
               "extra": serialize_missing(observed - expected), "groups": rows,
               "wall_seconds": time.monotonic() - started, "execution_wall_seconds": execution_wall,
               "cpu_seconds": usage.ru_utime + usage.ru_stime, "processes": PROCESSES,
               "threads_per_process": THREADS, **peaks}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2))
    return int(bool(summary["failed"] or summary["missing"] or summary["extra"]
                    or any(row["exit_code"] != 0 for row in rows)))


def terminate(signum, frame):
    raise SystemExit(128 + signum)


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, terminate)
    raise SystemExit(main())
