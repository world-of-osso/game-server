#!/usr/bin/env python3
"""Merge train: one integrator builds every queued agent commit together.

Agents stop building on their own. They commit, then `submit` the commit with the
tests that prove it and optional parity measurements. Each cycle the train merges
every queued commit onto origin/master in one tree, runs one warm check and one test
build, and runs every entry's tests from that build. Entries that conflict, break the
check, or fail their tests are rejected with the log; the rest are pushed to master
together when they asked to land. Measurements then run in parallel from one parity
executable. `--probe` entries (e.g. a RED proof) are built and tested but never pushed.

  merge_train.py submit --commit SHA --test '-p server-simulation --lib -- name' [--land]
                        [--measure spec=253:world_db=PATH:output=DIR:actor=player:targets=1,3]
  merge_train.py wait ID      # blocks until the entry's result exists, prints it
  merge_train.py run          # the integrator loop (lead only)
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shlex
import subprocess
import sys
import time
import uuid

ROOT = Path(__file__).resolve().parent.parent
HOME = Path("/home/osso/.worktrees/train")
QUEUE, RUNNING, RESULTS, LOGS = (HOME / name for name in ("queue", "running", "results", "logs"))
# The lead landing worktree; its warm target-lead cache is reused (another target costs 50-130 GiB).
TREE = Path("/home/osso/.worktrees/game-server-land")
TARGET = TREE / "target-lead"
CAPACITY = Path("/home/osso/.worktrees/logs/capacity.sh")
AGENT_RUN = Path("/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run")
# Collect submissions for a while so one build covers several agents.
GATHER_SECONDS = 300
MEASURE_PARALLEL = 8
ASSISTED_DB2 = "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/dps-parity-2026-10-21/native-player-inputs/assisted-70448/"
RUNNER = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER"


def git(*args, check=True):
    return subprocess.run(["git", *args], cwd=TREE, text=True, capture_output=True, check=check)


def base_env():
    return os.environ | {"CARGO_TARGET_DIR": str(TARGET), "DPS_PARITY_ASSISTED_DB2": ASSISTED_DB2}


def test_env(entry, test):
    """Native parity tests read the entry's first --measure inputs and run in the
    read-only startup-DB view of dps_parity_native.py; --env overrides both."""
    env = {}
    if entry["measure"]:
        m = entry["measure"][0]
        env = {"DPS_PARITY_WORLD_DB": m["world_db"], "DPS_PARITY_OUTPUT": m["output"],
               "DPS_PARITY_SPEC": m["spec"], "DPS_PARITY_ACTOR": m.get("actor", "follower"),
               "DPS_PARITY_TARGETS": m["targets"].split(",")[0]}
        # The view wrapper accepts only server_simulation executables.
        if "server-simulation" in test:
            env[RUNNER] = f"python3 {TREE / 'scripts/dps_parity_native.py'}"
    return env | entry.get("env", {})


def cargo(log, *args, env=None):
    """Run cargo in the train tree inside agents.slice; append output to log; return exit code."""
    with log.open("a") as out:
        out.write(f"\n$ cargo {shlex.join(args)}\n")
        out.flush()
        return subprocess.run([str(AGENT_RUN), "merge-train", "cargo", *args], cwd=TREE, env=base_env() | (env or {}),
                              stdout=out, stderr=subprocess.STDOUT, check=False).returncode


def wait_for_capacity():
    env = os.environ | {"CAPACITY_ID": "lead_merge_train"}
    while True:
        verdict = subprocess.run(["sh", str(CAPACITY)], env=env, text=True,
                                 capture_output=True, check=False).stdout
        if "ADD" in verdict:
            return
        time.sleep(30)


def parse_measure(text):
    # Colon-separated so targets=1,3 keeps its comma.
    fields = dict(part.split("=", 1) for part in text.split(":"))
    fields.setdefault("targets", "1,3")
    for key in ("spec", "world_db", "output"):
        if key not in fields:
            raise SystemExit(f"--measure needs {key}=")
    return fields


def submit(args):
    commit = subprocess.run(["git", "rev-parse", "--verify", f"{args.commit}^{{commit}}"],
                            text=True, capture_output=True, check=True).stdout.strip()
    entry = {"id": uuid.uuid4().hex[:10], "owner": args.owner, "commit": commit,
             "land": args.land, "env": dict(pair.split("=", 1) for pair in args.env), "tests": [shlex.split(test) for test in args.test],
             "measure": [parse_measure(m) for m in args.measure], "submitted": time.time()}
    QUEUE.mkdir(parents=True, exist_ok=True)
    (QUEUE / f"{entry['id']}.json").write_text(json.dumps(entry, indent=2))
    print(entry["id"])


def wait(args):
    result = RESULTS / f"{args.id}.json"
    while not result.exists():
        time.sleep(30)
    print(result.read_text())


def reset_tree():
    git("fetch", "-q", "origin")
    git("reset", "-q", "--hard", "origin/master")
    git("clean", "-qfd", "-e", "target-lead")


def merge(entries, log):
    """Merge entries in order; return those that merged and those that conflicted."""
    merged, conflicted = [], []
    for entry in entries:
        result = git("merge", "--no-ff", "--no-edit", "-q", entry["commit"], check=False)
        if result.returncode:
            git("merge", "--abort", check=False)
            entry["reason"] = f"merge conflict:\n{result.stdout}{result.stderr}"
            conflicted.append(entry)
        else:
            merged.append(entry)
    with log.open("a") as out:
        out.write(f"merged {[e['id'] for e in merged]} conflicted {[e['id'] for e in conflicted]}\n")
    return merged, conflicted


def check(log):
    return cargo(log, "check", "--locked", "--workspace", "--all-targets") == 0 \
        and cargo(log, "fmt", "--check") == 0


def build_checked(entries, log):
    """Return (tree entries that check together, rejected). Bisect linearly on failure."""
    reset_tree()
    merged, rejected = merge(entries, log)
    if check(log):
        return merged, rejected
    # The joint tree fails: add entries one by one so only the breaking ones are dropped.
    reset_tree()
    kept = []
    for entry in merged:
        # --no-ff keeps HEAD~1 the previous tree, so a failing entry can be dropped.
        if git("merge", "--no-ff", "--no-edit", "-q", entry["commit"], check=False).returncode:
            git("merge", "--abort", check=False)
            entry["reason"] = "merge conflict"
            rejected.append(entry)
            continue
        if check(log):
            kept.append(entry)
        else:
            entry["reason"] = f"check or fmt fails with this commit; see {log}"
            rejected.append(entry)
            git("reset", "-q", "--hard", "HEAD~1")
    return kept, rejected


def run_tests(entries, log):
    passed, failed = [], []
    for entry in entries:
        entry_log = LOGS / f"{entry['id']}.tests.log"
        codes = [cargo(entry_log, "test", "--locked", *test, env=test_env(entry, test))
                 for test in entry["tests"]]
        entry["test_log"] = str(entry_log)
        if any(codes):
            entry["reason"] = f"tests failed; see {entry_log}"
            failed.append(entry)
        else:
            passed.append(entry)
    return passed, failed


def push(entries, log, built):
    """Push master plus the landing entries. Unless the tree already holds exactly those
    entries (built), rebuild and retest them first. Retry on races."""
    while True:
        if built:
            tree, rejected, failed = entries, [], []
            built = False
        else:
            tree, rejected = build_checked(entries, log)
            tree, failed = run_tests(tree, log)
        if rejected or failed:
            for entry in rejected + failed:
                entry["status"] = "rejected"
            entries = tree
            write_results(rejected + failed)
            if not entries:
                return None
            continue
        if not tree:
            return None
        result = git("push", "origin", "HEAD:master", check=False)
        if result.returncode == 0:
            return git("rev-parse", "HEAD").stdout.strip()
        with log.open("a") as out:
            out.write(f"push raced, rebuilding:\n{result.stderr}\n")


def parity_executable(log):
    out = subprocess.run(
        [str(AGENT_RUN), "merge-train", "cargo", "test", "--locked", "-p", "server-simulation",
         "--lib", "--no-run", "--message-format=json-render-diagnostics"],
        cwd=TREE, env=base_env(), text=True, capture_output=True, check=False)
    with log.open("a") as f:
        f.write(out.stderr)
    for line in out.stdout.splitlines():
        if line.startswith("{"):
            message = json.loads(line)
            if message.get("reason") == "compiler-artifact" and message.get("executable") \
                    and message["target"]["name"] == "server_simulation":
                return message["executable"]
    return None


def measure(entry, executable):
    lines = []
    for m in entry["measure"]:
        command = [sys.executable, str(TREE / "scripts/dps_parity_measure.py"), "--executable", executable,
                   "--spec", m["spec"], "--targets", m["targets"], "--actor", m.get("actor", "follower"),
                   "--world-db", m["world_db"], "--output", m["output"]]
        out = subprocess.run(command, cwd=TREE, env=base_env(), text=True, capture_output=True, check=False)
        lines += [line for line in out.stdout.splitlines() if "NATIVE_DPS" in line or "FAILED" in line]
    entry["measurements"] = lines


def write_results(entries):
    RESULTS.mkdir(parents=True, exist_ok=True)
    for entry in entries:
        (RESULTS / f"{entry['id']}.json").write_text(json.dumps(entry, indent=2))
        (RUNNING / f"{entry['id']}.json").unlink(missing_ok=True)


def take_queue():
    RUNNING.mkdir(parents=True, exist_ok=True)
    entries = []
    for path in sorted(QUEUE.glob("*.json")):
        target = RUNNING / path.name
        path.rename(target)
        entries.append(json.loads(target.read_text()))
    return sorted(entries, key=lambda e: e["submitted"])


def cycle():
    entries = take_queue()
    log = LOGS / f"cycle-{time.strftime('%Y%m%dT%H%M%S')}.log"
    wait_for_capacity()
    # One tree with every entry: probes get their test results from it.
    tree, rejected = build_checked(entries, log)
    tested, failed = run_tests(tree, log)
    for entry in rejected + failed:
        entry["status"] = "rejected"
    probes = [e for e in tested if not e["land"]]
    landing = [e for e in tested if e["land"]]
    for entry in probes:
        entry["status"] = "probe passed"
    executable = parity_executable(log) if any(e["measure"] for e in probes) else None
    if executable:
        with ThreadPoolExecutor(MEASURE_PARALLEL) as pool:
            list(pool.map(lambda e: measure(e, executable), [e for e in probes if e["measure"]]))
    write_results(rejected + failed + probes)
    # Landing tree excludes probes, so it is rebuilt and retested on its own.
    # Every probe or failed entry is still merged in this tree; only a tree of exactly the
    # landing entries may be pushed as built.
    master = push(landing, log, built=not (probes or failed)) if landing else None
    if master:
        for entry in landing:
            entry["status"] = f"landed in {master}"
        executable = parity_executable(log) if any(e["measure"] for e in landing) else None
        if executable:
            with ThreadPoolExecutor(MEASURE_PARALLEL) as pool:
                list(pool.map(lambda e: measure(e, executable), [e for e in landing if e["measure"]]))
        write_results(landing)


def run(_args):
    for directory in (QUEUE, RUNNING, RESULTS, LOGS):
        directory.mkdir(parents=True, exist_ok=True)
    # Entries left in running/ by an interrupted cycle go back to the queue.
    for path in RUNNING.glob("*.json"):
        path.rename(QUEUE / path.name)
    while True:
        queued = list(QUEUE.glob("*.json"))
        if queued and time.time() - min(p.stat().st_mtime for p in queued) >= GATHER_SECONDS:
            cycle()
        else:
            time.sleep(30)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    s = sub.add_parser("submit")
    s.add_argument("--commit", required=True)
    s.add_argument("--owner", default=os.environ.get("PI_AGENT_ID", "unknown"))
    s.add_argument("--test", action="append", default=[], help="cargo test arguments, quoted")
    s.add_argument("--measure", action="append", default=[],
                   help="also supplies DPS_PARITY_* and the DB view to server-simulation tests")
    s.add_argument("--env", action="append", default=[], help="KEY=VALUE for this entry's tests")
    s.add_argument("--land", action="store_true", help="push to master when green")
    s.set_defaults(func=submit)
    w = sub.add_parser("wait")
    w.add_argument("id")
    w.set_defaults(func=wait)
    sub.add_parser("run").set_defaults(func=run)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
