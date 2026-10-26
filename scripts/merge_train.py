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
import re
import shlex
import shutil
import subprocess
import sys
import time
import tomllib
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
    # Native tests and measurement read their inputs from the output directory; a missing input
    # used to cost a whole cycle before failing.
    missing = [p for p in (Path(fields["world_db"]), Path(fields["output"]) / f"{fields['spec']}.profile.json",
                           Path(fields["output"]) / f"{fields['spec']}.selected-traits-active.json")
               if not p.exists()]
    if missing:
        raise SystemExit(f"--measure inputs missing: {', '.join(map(str, missing))}")
    return fields


def submit(args):
    commit = subprocess.run(["git", "rev-parse", "--verify", f"{args.commit}^{{commit}}"],
                            text=True, capture_output=True, check=True).stdout.strip()
    entry = {"id": uuid.uuid4().hex[:10], "owner": args.owner, "commit": commit,
             "land": args.land, "red": args.red and subprocess.run(
                 ["git", "rev-parse", "--verify", f"{args.red}^{{commit}}"], text=True,
                 capture_output=True, check=True).stdout.strip(), "env": dict(pair.split("=", 1) for pair in args.env), "tests": [shlex.split(test) for test in args.test],
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
            # Clean against master means it only collides with another queued entry:
            # retry next cycle, after that entry lands or is rejected.
            entry["defer"] = git("merge-tree", "--write-tree", "origin/master", entry["commit"],
                                 check=False).returncode == 0
            entry["reason"] = f"merge conflict:\n{result.stdout}{result.stderr}"
            conflicted.append(entry)
        else:
            merged.append(entry)
    with log.open("a") as out:
        out.write(f"merged {[e['id'] for e in merged]} conflicted {[e['id'] for e in conflicted]}\n")
    return merged, conflicted


def check(log):
    """Compile check. Formatting is applied, not checked: an unformatted entry would
    otherwise cost a bisect, and formatter output is never a behavior change."""
    if cargo(log, "check", "--locked", "--workspace", "--all-targets") != 0:
        return False
    # The workspace check unifies test-support features into every crate; the deployed server
    # binary builds without them (an ungated simc_apl_native call broke it on 2026-10-26).
    if cargo(log, "check", "--locked", "-p", "server", "--bins") != 0:
        return False
    if cargo(log, "fmt") != 0:
        return False
    # cargo fmt does not follow include!() or macro-generated #[path] mods, so the modules declared
    # in the union-merged module_list.in and server-spell-scripts' modules.rs are formatted directly.
    included = sorted(str(p) for d in (PARITY_DIR, SCRIPT_MODULES_DIR) for p in (TREE / d).glob("*.rs"))
    if subprocess.run(["rustfmt", "--edition", "2024", *included], cwd=TREE, check=False).returncode != 0:
        return False
    if git("status", "--porcelain", "--untracked-files=no").stdout.strip():
        git("commit", "-qam", "merge train: cargo fmt")
    return True


PARITY_DIR = "crates/server-simulation/src/dungeon_simulator/dps_parity"
SCRIPT_MODULES_DIR = "crates/server-spell-scripts/src/modules"


class MasterBroken(Exception):
    pass


def build_checked(entries, log):
    """Return (tree entries that check together, rejected), with the tree checked out.
    A failing set is split in halves, so a few broken entries cost O(k log n) checks."""
    reset_tree()
    merged, rejected = merge(entries, log)
    good = []

    def tree_checks(candidates):
        reset_tree()
        merge(good + candidates, log)
        return check(log)

    def admit(candidates):
        if not candidates:
            return
        if tree_checks(candidates):
            good.extend(candidates)
        elif len(candidates) == 1:
            candidates[0]["reason"] = f"check or fmt fails with this commit; see {log}"
            rejected.append(candidates[0])
        else:
            half = len(candidates) // 2
            admit(candidates[:half])
            admit(candidates[half:])

    if check(log):
        return merged, rejected
    # A broken master rejects everything; report that instead of blaming entries.
    reset_tree()
    if not check(log):
        raise MasterBroken(f"origin/master itself fails check; see {log}")
    admit(merged)
    reset_tree()
    merge(good, log)
    return good, rejected


TEST_PARALLEL = 8


def test_executables(entry, test, entry_log):
    """Build (under Cargo's lock) and list the test executables for one --test.
    Returns [(executable, cwd)] or None when the build fails."""
    cargo_args, _, _ = split_test(test)
    out = subprocess.run(
        [str(AGENT_RUN), "merge-train", "cargo", "test", "--locked", "--no-run",
         "--message-format=json-render-diagnostics", *cargo_args],
        cwd=TREE, env=base_env() | test_env(entry, test), text=True, capture_output=True, check=False)
    with entry_log.open("a") as f:
        f.write(f"\n$ cargo test --no-run {shlex.join(cargo_args)}\n{out.stderr}")
    if out.returncode:
        return None
    found = []
    for line in out.stdout.splitlines():
        if line.startswith("{"):
            message = json.loads(line)
            if message.get("reason") == "compiler-artifact" and message.get("executable") \
                    and message["profile"]["test"]:
                found.append((message["executable"], str(Path(message["manifest_path"]).parent)))
    return found


# Cargo test options that take a value; any other bare word before `--` is the TESTNAME filter.
CARGO_VALUE_FLAGS = {"-p", "--package", "-F", "--features", "--config", "--test", "--bin", "--example",
                     "--bench", "--target", "--target-dir", "-j", "--jobs", "--manifest-path", "--profile",
                     "--message-format", "--exclude", "--color", "-Z"}


def split_test(test):
    """Split `--test` args into (cargo args, "--", test binary args). Cargo forwards its
    positional TESTNAME filter to the binaries, so it moves to the binary args."""
    i = test.index("--") if "--" in test else len(test)
    cargo_args, binary_args, filters = [], list(test[i + 1:]), []
    takes_value = False
    previous = None
    for arg in test[:i]:
        if takes_value:
            # Gameplay tests moved from `server` to `server-combat` (b316f4d9b); `server` keeps
            # only binaries and site export, so old submissions are retargeted.
            cargo_args.append("server-combat" if previous in ("-p", "--package") and arg == "server" else arg)
            takes_value = False
        elif arg.startswith("-"):
            cargo_args.append(arg)
            takes_value = arg in CARGO_VALUE_FLAGS
            previous = arg
        else:
            filters.append(arg)
    return cargo_args, "--", filters + binary_args


def config_overrides(cargo_args):
    """Env and runner from `--config` TOML snippets, which Cargo would apply when it ran the tests."""
    env, runner = {}, None
    for i, arg in enumerate(cargo_args[:-1]):
        if arg != "--config":
            continue
        try:
            table = tomllib.loads(cargo_args[i + 1])
        except tomllib.TOMLDecodeError:
            continue
        for key, value in table.get("env", {}).items():
            # `force`/`relative` arrive as separate snippets without a value.
            if not isinstance(value, dict):
                env[key] = value
            elif "value" in value:
                env[key] = value["value"]
        for target in table.get("target", {}).values():
            if "runner" in target:
                runner = target["runner"]
    if isinstance(runner, str):
        runner = shlex.split(runner)
    return env, runner


def run_executable(entry, test, executable, cwd, entry_log):
    """Run one test binary outside Cargo, so entries run in parallel without its build lock."""
    env = base_env() | test_env(entry, test)
    runner = shlex.split(env.pop(RUNNER, ""))
    config_env, config_runner = config_overrides(split_test(test)[0])
    env |= config_env
    runner = config_runner or runner
    _, _, test_args = split_test(test)
    command = [*runner, executable, *test_args]
    out = subprocess.run(command, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                         stderr=subprocess.STDOUT, check=False)
    with entry_log.open("a") as f:
        f.write(f"\n$ {shlex.join(command)}\n{out.stdout}")
    ran = sum(int(m) for m in re.findall(r"test result: \w+\. (\d+) passed; (?:\d+) failed", out.stdout))
    ran += sum(int(m) for m in re.findall(r"test result: \w+\. \d+ passed; (\d+) failed", out.stdout))
    return out.returncode, ran


def run_tests(entries, log):
    # Build every entry's test binaries first (serialized by Cargo), then run them all in parallel.
    jobs, broken = [], set()
    for entry in entries:
        entry_log = LOGS / f"{entry['id']}.tests.log"
        entry_log.unlink(missing_ok=True)
        entry["test_log"] = str(entry_log)
        for test in entry["tests"]:
            found = test_executables(entry, test, entry_log)
            if found is None:
                broken.add(entry["id"])
            else:
                jobs += [(entry, test, exe, cwd, entry_log) for exe, cwd in found]
    with ThreadPoolExecutor(TEST_PARALLEL) as pool:
        codes = list(pool.map(lambda job: ((job[0]["id"], tuple(job[1])), run_executable(*job)), jobs))
    failing = broken | {key[0] for key, (code, _) in codes if code}
    # A --test that runs no test at all (a stale filter or package) is a failure, not a pass.
    ran = {}
    for key, (_, count) in codes:
        ran[key] = ran.get(key, 0) + count
    empty = {e["id"] for e in entries for t in e["tests"]
             if e["id"] not in broken and ran.get((e["id"], tuple(t)), 0) == 0}
    passed, failed = [], []
    for entry in entries:
        if entry["id"] in failing or entry["id"] in empty:
            entry["reason"] = (f"tests failed; see {entry['test_log']}" if entry["id"] in failing
                               else f"tests failed: a --test ran 0 tests (stale filter or package?); see {entry['test_log']}")
            failed.append(entry)
        else:
            passed.append(entry)
    return passed, failed


def push(entries, log):
    """Build, test and push master plus the entries. Failing entries are rejected and the
    rest rebuilt; a push race rebuilds on the new master. Returns (master, pushed entries);
    entries deferred back to the queue are in neither."""
    while entries:
        tree, rejected = build_and_test(entries, log)
        if rejected:
            entries = tree
            continue
        if not tree:
            break
        result = git("push", "origin", "HEAD:master", check=False)
        if result.returncode == 0:
            return git("rev-parse", "HEAD").stdout.strip(), tree
        entries = tree
        with log.open("a") as out:
            out.write(f"push raced, rebuilding:\n{result.stderr}\n")
    return None, []


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


def measure(entry, executable, target):
    lines = []
    for m in entry["measure"]:
        command = [sys.executable, str(TREE / "scripts/dps_parity_measure.py"), "--executable", executable,
                   "--spec", m["spec"], "--targets", m["targets"], "--actor", m.get("actor", "follower"),
                   "--world-db", m["world_db"], "--output", m["output"]]
        out = subprocess.run(command, cwd=TREE, env=base_env() | {"CARGO_TARGET_DIR": str(target)},
                             text=True, capture_output=True, check=False)
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
    return drop_superseded(sorted(entries, key=lambda e: e["submitted"]))


def drop_superseded(entries):
    """A landing entry whose commit is an ancestor of a later landing entry from the same
    owner is a stale version of it (owners resubmit after fixing compile errors); a
    resubmitted identical commit is a duplicate."""
    landing = [e for e in entries if e["land"]]
    stale = []
    for old in landing:
        for new in landing:
            if new["submitted"] <= old["submitted"]:
                continue
            # The same commit with the same tests (any owner) is one landing; keep the newest.
            same = new["commit"] == old["commit"]
            if same and new["tests"] == old["tests"] or not same and new["owner"] == old["owner"] \
                    and git("merge-base", "--is-ancestor", old["commit"], new["commit"], check=False).returncode == 0:
                old["status"], old["reason"] = "rejected", f"superseded by {new['id']}"
                stale.append(old)
                break
    write_results(stale)
    return [e for e in entries if e not in stale]


def build_and_test(entries, log):
    """Check (bisecting failures) and test entries on master. Returns (passed, rejected);
    entries that only conflict with other queued entries go back to the queue."""
    tree, rejected = build_checked(entries, log)
    deferred = [e for e in rejected if e.pop("defer", False)]
    rejected = [e for e in rejected if e not in deferred]
    # Owners get conflicts and check failures now, not when the whole cycle ends.
    for entry in rejected:
        entry["status"] = "rejected"
    write_results([e for e in rejected if not e["id"].endswith("-red")])
    for entry in deferred:
        del entry["reason"]
        queued = RUNNING / f"{entry['id']}.json"
        if queued.exists():  # RED twins have no queue file; their entry is requeued by cycle()
            queued.rename(QUEUE / queued.name)
    passed, failed = run_tests(tree, log)
    for entry in failed:
        entry["status"] = "rejected"
    write_results([e for e in failed if not e["id"].endswith("-red")])
    return passed, rejected + failed


# Measurements run in the background so the next cycle can build meanwhile.
MEASURER = ThreadPoolExecutor(MEASURE_PARALLEL)
MEASURING = set()  # futures; a reload waits until they finish


def measure_then_write(entries, log):
    """Write results now for entries without --measure; measure the others from a private
    copy of this cycle's executable (the next build overwrites target-lead), then write."""
    measured = [e for e in entries if e["measure"]]
    write_results([e for e in entries if not e["measure"]])
    executable = parity_executable(log) if measured else None
    if executable is None:
        for entry in measured:
            entry["measurements"] = ["FAILED: parity executable did not build"]
        write_results(measured)
        return
    # dps_parity_native.py accepts executables under CARGO_TARGET_DIR only.
    target = HOME / "bin" / uuid.uuid4().hex[:10]
    target.mkdir(parents=True)
    copy = target / Path(executable).name
    shutil.copy2(executable, copy)
    pending = [len(measured)]

    def finish(entry):
        try:
            measure(entry, str(copy), target)
        except Exception as error:  # a failed measurement must still produce a result
            entry["measurements"] = [f"FAILED: {error}"]
        write_results([entry])
        pending[0] -= 1
        if pending[0] == 0:
            shutil.rmtree(target, ignore_errors=True)

    for entry in measured:
        future = MEASURER.submit(finish, entry)
        MEASURING.add(future)
        future.add_done_callback(MEASURING.discard)


def red_twin(entry):
    """The entry's tests at its --red commit (tests without the fix): they must fail."""
    # Keeps --measure: it supplies the native test inputs. Twins are never measured.
    return entry | {"id": f"{entry['id']}-red", "commit": entry["red"], "land": False}


def land(entries, log):
    """Build exactly these entries on master, push the passing ones, then measure them."""
    master, pushed = push(entries, log)
    for entry in pushed:
        entry["status"] = f"landed in {master}"
    measure_then_write(pushed, log)


def run_probes(probes, gated, log):
    """Probes, then the RED twins of gated landings, each in a tree that is never pushed.
    Returns the gated landings whose RED was observed."""
    # A RED twin must not share a tree with another commit that contains its fix (an owner's
    # later probe usually does), so twins get their own tree. A twin that itself contains
    # another gated entry's fix waits for the next cycle.
    def contains_fix(twin, entry):
        return any(other is not entry and git("merge-base", "--is-ancestor", other["commit"], twin["commit"],
                                              check=False).returncode == 0 for other in gated)
    twins = {}
    for entry in list(gated):
        twin = red_twin(entry)
        if contains_fix(twin, entry):
            gated.remove(entry)
            (RUNNING / f"{entry['id']}.json").rename(QUEUE / f"{entry['id']}.json")
        else:
            twins[entry["id"]] = twin
    passed, failed = build_and_test(probes, log) if probes else ([], [])
    for entry in passed:
        entry["status"] = "probe passed"
    # Measure now, while the tree is still the probe tree: the RED twins below rebuild it
    # without the probes, so a later parity executable would lack their changes.
    measure_then_write(passed, log)
    twin_passed, twin_failed = build_and_test(list(twins.values()), log) if twins else ([], [])
    for entry in twin_passed:
        entry["status"] = "probe passed"
    for entry in failed + twin_failed:
        entry["status"] = "rejected"
    passed, failed = passed + twin_passed, failed + twin_failed
    write_results([e for e in failed if not e["id"].endswith("-red")])
    outcome = {e["id"]: e for e in passed + failed}
    proven = []
    for entry in gated:
        twin = outcome.get(twins[entry["id"]]["id"])
        if twin is None:  # deferred with the probe tree; retry the pair next cycle
            (RUNNING / f"{entry['id']}.json").rename(QUEUE / f"{entry['id']}.json")
            continue
        entry["red_log"] = twin.get("test_log")
        if twin["status"] == "rejected" and "tests failed" in twin.get("reason", ""):
            proven.append(entry)
            continue
        entry["status"] = "rejected"
        entry["reason"] = ("RED not observed: tests pass without the fix" if twin["status"] != "rejected"
                           else f"RED commit does not build: {twin.get('reason', '')[:500]}")
        write_results([entry])
    return proven


def mark_phase(log, phase, entries):
    """Record which entries the running cycle is on, for `where`."""
    (HOME / "current.json").write_text(json.dumps(
        {"cycle": log.name, "phase": phase, "since": time.time(), "entries": [e["id"] for e in entries]}))


def cycle():
    entries = take_queue()
    log = LOGS / f"cycle-{time.strftime('%Y%m%dT%H%M%S')}.log"
    wait_for_capacity()
    probes = [e for e in entries if not e["land"]]
    gated = [e for e in entries if e["land"] and e.get("red")]
    plain = [e for e in entries if e["land"] and not e.get("red")]
    # Proven RED-gated entries land in the same build as plain ones: landing plain entries
    # first moved master under the gated ones, which then conflicted with this cycle's own landings.
    proven = []
    if probes or gated:
        mark_phase(log, "1/2 probes and RED twins", probes + gated)
        proven = run_probes(probes, gated, log)
    mark_phase(log, "2/2 landings", plain + proven)
    land(plain + proven, log)
    (HOME / "current.json").unlink(missing_ok=True)


def requeue():
    """Entries left in running/ go back to the queue."""
    for path in RUNNING.glob("*.json"):
        path.rename(QUEUE / path.name)


def run(_args):
    for directory in (QUEUE, RUNNING, RESULTS, LOGS):
        directory.mkdir(parents=True, exist_ok=True)
    requeue()
    script = Path(__file__).resolve()
    loaded = script.stat().st_mtime
    while True:
        # deploy_merge_train.py replaces this file; pick it up between cycles.
        if script.stat().st_mtime != loaded:
            # Start no new cycle (it would start more measurements); reload once they drain.
            if MEASURING:
                time.sleep(30)
                continue
            print("merge_train.py changed, reloading", flush=True)
            os.execv(sys.executable, [sys.executable, str(script), "run"])
        queued = list(QUEUE.glob("*.json"))
        if queued and time.time() - min(p.stat().st_mtime for p in queued) >= GATHER_SECONDS:
            try:
                cycle()
            except MasterBroken as error:
                # Not the entries' fault: keep them queued until master or the shared checkout is fixed.
                print(error, flush=True)
                requeue()
                time.sleep(300)
        else:
            time.sleep(30)


def where(args):
    """Say where one entry is: queued, in which phase of the running cycle, or done."""
    for directory, label in ((RESULTS, "done"), (QUEUE, "queued for the next cycle")):
        path = directory / f"{args.id}.json"
        if path.exists():
            entry = json.loads(path.read_text())
            print(f"{label}: {entry.get('status', '')} {entry.get('reason', '')[:300]}".rstrip())
            return
    if (RUNNING / f"{args.id}.json").exists():
        current = HOME / "current.json"
        try:
            info = json.loads(current.read_text())
        except (FileNotFoundError, json.JSONDecodeError):
            # Between cycles: a landed entry's measurement may still be running in the background.
            print("running, between cycles (measuring, or waiting for the next cycle to start); keep waiting")
            return
        here = args.id in info.get("entries", [])
        minutes = round((time.time() - info.get("since", time.time())) / 60, 1)
        print(f"in cycle {info.get('cycle')}: phase {info.get('phase')} ({minutes} min)"
              + ("; this entry is in the current phase" if here else "; this entry waits for a later phase of this cycle"))
        return
    print("unknown id")


def withdraw(args):
    """Drop a queued entry (an owner's obsolete or superseded submission); running entries finish."""
    queued = QUEUE / f"{args.id}.json"
    try:
        entry = json.loads(queued.read_text())
        queued.unlink()
    except FileNotFoundError:
        raise SystemExit(f"{args.id} is not queued (running entries cannot be withdrawn; see `where`)")
    entry["status"], entry["reason"] = "rejected", "withdrawn by owner"
    write_results([entry])
    print(f"withdrew {args.id}")


def status(_args):
    """Print the train's throughput metrics as JSON (read-only)."""
    now = time.time()

    def ages(directory):
        times = [p.stat().st_mtime for p in directory.glob("*.json")]
        return {"count": len(times), "oldest_minutes": round((now - min(times)) / 60, 1) if times else None}

    cycles = []
    logs = sorted(LOGS.glob("cycle-*.log"), key=lambda p: p.stat().st_mtime)
    for start, log in enumerate(logs[-4:]):
        text = log.read_text(errors="replace")
        begun = time.mktime(time.strptime(log.name[6:21], "%Y%m%dT%H%M%S"))
        cycles.append({"log": log.name, "minutes": round((log.stat().st_mtime - begun) / 60, 1),
                       "checks": text.count("$ cargo check"), "push_races": text.count("push raced")})
    results = sorted(RESULTS.glob("*.json"), key=lambda p: p.stat().st_mtime)[-30:]
    outcome = {}
    for path in results:
        entry = json.loads(path.read_text())
        state = entry["status"].split(" ")[0]
        if state == "rejected":
            reason = entry.get("reason", "")
            state = next((name for key, name in (("superseded", "rejected: superseded"),
                                                  ("merge conflict", "rejected: merge conflict"),
                                                  ("check or fmt", "rejected: check/fmt"),
                                                  ("tests failed", "rejected: tests failed"),
                                                  ("RED", "rejected: RED gate")) if key in reason),
                         "rejected: other")
            if state == "rejected: tests failed" and not entry["land"]:
                state = "probe RED (expected)"
        outcome[state] = outcome.get(state, 0) + 1
    git("fetch", "-q", "origin", check=False)
    landed = git("log", "-1", "--format=%ct %h %s", "--invert-grep", "--grep=^merge train",
                 "--grep=^AGENTS", "origin/master", check=False).stdout.split(" ", 2)
    report = {"unit": subprocess.run(["systemctl", "--user", "is-active", "merge-train"], text=True,
                                     capture_output=True).stdout.strip(),
              "queue": ages(QUEUE), "running": ages(RUNNING),
              "cycles": cycles[:-1], "current_cycle": cycles[-1] if cycles else None,
              "last_30_results": outcome,
              "last_landing": {"minutes_ago": round((now - int(landed[0])) / 60, 1), "commit": landed[1],
                               "subject": landed[2].strip()} if len(landed) == 3 else None}
    print(json.dumps(report, indent=2))


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
    s.add_argument("--red", help="commit with the tests but without the fix; its tests must fail "
                                 "in the same cycle before the entry lands (replaces a separate RED probe)")
    s.set_defaults(func=submit)
    w = sub.add_parser("wait")
    w.add_argument("id")
    w.set_defaults(func=wait)
    sub.add_parser("run").set_defaults(func=run)
    wh = sub.add_parser("where", help="where one entry is")
    wh.add_argument("id")
    wh.set_defaults(func=where)
    wd = sub.add_parser("withdraw", help="drop one queued entry")
    wd.add_argument("id")
    wd.set_defaults(func=withdraw)
    sub.add_parser("status", help="throughput metrics for the periodic bottleneck check").set_defaults(func=status)
    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
