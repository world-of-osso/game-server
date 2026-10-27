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
from contextlib import contextmanager
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import subprocess
import sys
import threading
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
ISOLATED_PER_CYCLE = 3
# Global concurrent spec measurements; each spec can run both target counts in parallel.
MEASURE_PARALLEL = 8
MEASURE_SEEDS = "70448,1,2,3,4,5,6,7,8,9"
ASSISTED_DB2 = "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/dps-parity-2026-10-21/native-player-inputs/assisted-70448/"
RUNNER = "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER"


def git(*args, check=True):
    return subprocess.run(["git", *args], cwd=TREE, text=True, capture_output=True, check=check)


TIMING_LOCK = threading.Lock()


def tree_id():
    return git("rev-parse", "HEAD^{tree}").stdout.strip()


def timing_event(log, phase, event, tree, **details):
    record = {"phase": phase, "event": event, "monotonic": time.monotonic(),
              "tree": tree, **details}
    with TIMING_LOCK, log.open("a") as out:
        out.write(f"TIMING {json.dumps(record, sort_keys=True)}\n")


@contextmanager
def timed(log, phase, tree, **details):
    details = {"span": uuid.uuid4().hex, **details}
    timing_event(log, phase, "start", tree, **details)
    try:
        yield
    finally:
        timing_event(log, phase, "end", tree, **details)


def base_env():
    return os.environ | {"CARGO_TARGET_DIR": str(TARGET), "DPS_PARITY_ASSISTED_DB2": ASSISTED_DB2}


def test_env(entry, test):
    """Native parity tests read the entry's first --measure inputs and run in the
    read-only startup-DB view of dps_parity_native.py; --env overrides both."""
    env = {}
    if entry["measure"]:
        # A leading `@SPEC` token runs this test against that spec's --measure inputs, so one
        # entry can gate other specs' fixtures (each needs its own profile/DB/actor).
        chosen = test_spec(test)
        m = next((m for m in entry["measure"] if m["spec"] == chosen), entry["measure"][0])
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


MODULE_LINE = re.compile(r'\bmod\s+(\w+)\s*;')
INLINE_MODULE = re.compile(r'\bmod\s+(\w+)\s*\{|[{}]')
PATH_ATTRIBUTE = re.compile(r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]')
RAW_LITERAL = re.compile(r'(?:br|r)(\#*)"')
CHAR_LITERAL = re.compile(r"'(?:\\\\.|[^'\\\\])'")


def mask_rust_noncode(source):
    """Blank comments and literals while preserving positions and newlines."""
    output = list(source)
    index = 0
    block_depth = 0
    end_literal = None
    while index < len(source):
        start = index
        if block_depth:
            if source.startswith("/*", index):
                block_depth += 1
                index += 2
            elif source.startswith("*/", index):
                block_depth -= 1
                index += 2
            else:
                index += 1
        elif end_literal is not None:
            if source.startswith(end_literal, index):
                index += len(end_literal)
                end_literal = None
            elif end_literal == '"' and source[index] == "\\":
                index += min(2, len(source) - index)
            else:
                index += 1
        elif source.startswith("//", index):
            newline = source.find("\n", index)
            index = len(source) if newline < 0 else newline
        elif source.startswith("/*", index):
            block_depth = 1
            index += 2
        elif (raw := RAW_LITERAL.match(source, index)):
            end_literal = '"' + raw.group(1)
            index += len(raw.group())
        elif source[index] == '"':
            end_literal = '"'
            index += 1
        elif (char := CHAR_LITERAL.match(source, index)):
            index += len(char.group())
        else:
            index += 1
            continue
        for position in range(start, index):
            if output[position] != "\n":
                output[position] = " "
    return "".join(output)


def added_module_lines(commit):
    """Return (path, line, column, name) for declarations added since master."""
    revision = f"origin/master...{commit}"
    paths = git("diff", "--name-only", "-z", "--diff-filter=ACMRT", revision,
                "--", "*.rs", "*.in").stdout.split("\0")
    found = []
    for path in filter(None, paths):
        source = git("show", f"{commit}:{path}").stdout
        code_lines = mask_rust_noncode(source).splitlines()
        diff = git("diff", "--no-ext-diff", "--no-renames", "--unified=0", revision,
                   "--", path).stdout
        line_number = 0
        for line in diff.splitlines():
            hunk = re.match(r"@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@", line)
            if hunk:
                line_number = int(hunk.group(1))
            elif line.startswith("+") and not line.startswith("+++"):
                for declaration in MODULE_LINE.finditer(code_lines[line_number - 1]):
                    found.append((path, line_number, declaration.start(), declaration.group(1)))
                line_number += 1
            elif line.startswith(" "):
                line_number += 1
    return found


def module_directory(path):
    source = Path(path)
    if source.suffix == ".in" or source.name in ("mod.rs", "lib.rs", "main.rs") \
            or source.parent.name in ("bin", "examples", "tests", "benches"):
        return source.parent
    return source.parent / source.stem


def module_paths(path, source, number, column, name):
    """Resolve an external declaration in a file or its inline module."""
    lines = source.splitlines()
    depth = 0
    modules = []
    code_lines = mask_rust_noncode(source).splitlines()
    preceding = code_lines[:number - 1] + [code_lines[number - 1][:column]]
    for line in preceding:
        for token in INLINE_MODULE.finditer(line):
            if token.group(1):
                depth += 1
                modules.append((depth, token.group(1)))
            elif token.group() == "{":
                depth += 1
            else:
                if modules and modules[-1][0] == depth:
                    modules.pop()
                depth -= 1
    nested = Path(*(name for _, name in modules))
    prefix = lines[number - 1][:column]
    attrs = [re.split(r'[;{}]', prefix)[-1]]
    for line in reversed(lines[:number - 1]):
        stripped = line.strip()
        if stripped.startswith("#[") or stripped.startswith("//") or not stripped:
            attrs.append(line)
        else:
            break
    explicit = next((match.group(1) for line in attrs if (match := PATH_ATTRIBUTE.search(line))), None)
    if explicit is not None:
        parent = module_directory(path) if modules else Path(path).parent
        return [os.path.normpath(str(parent / nested / explicit))]
    directory = module_directory(path) / nested
    return [str(directory / f"{name}.rs"), str(directory / name / "mod.rs")]


def missing_added_modules(commit):
    sources = {}
    missing = []
    for path, number, column, name in added_module_lines(commit):
        if path not in sources:
            sources[path] = git("show", f"{commit}:{path}").stdout
        candidates = module_paths(path, sources[path], number, column, name)
        if not any(git("cat-file", "-e", f"{commit}:{candidate}", check=False).returncode == 0
                   for candidate in candidates):
            missing.append(f"{path}:{number}: mod {name}; requires committed " + " or ".join(candidates))
    return missing


def added_module_names(commit):
    return {name for _, _, _, name in added_module_lines(commit)}


def missing_module_names(log, start):
    with log.open() as source:
        source.seek(start)
        text = source.read()
    return set(re.findall(r'error\[E0583\]: file not found for module [`\']([^`\']+)[`\']', text))


def submit(args):
    commit = subprocess.run(["git", "rev-parse", "--verify", f"{args.commit}^{{commit}}"],
                            text=True, capture_output=True, check=True).stdout.strip()
    missing = missing_added_modules(commit)
    if missing:
        raise SystemExit("missing committed Rust module files:\n" + "\n".join(missing))
    entry = {"id": uuid.uuid4().hex[:10], "owner": args.owner, "commit": commit,
             "land": args.land, "red": args.red and subprocess.run(
                 ["git", "rev-parse", "--verify", f"{args.red}^{{commit}}"], text=True,
                 capture_output=True, check=True).stdout.strip(), "env": dict(pair.split("=", 1) for pair in args.env), "tests": [shlex.split(test) for test in args.test],
             "measure": [parse_measure(m) for m in args.measure], "submitted": time.time()}
    # Native parity tests read DPS_PARITY_* inputs that only --measure (or --env) supplies; without
    # them they fail at fixture setup a cycle later (Shadow Collapse/Ire probes, 2026-10-26).
    if any("server-simulation" in test and "--ignored" in test for test in entry["tests"]) \
            and not entry["measure"] and not any(k.startswith("DPS_PARITY_") for k in entry["env"]):
        sys.exit("native server-simulation --ignored tests need --measure (or --env DPS_PARITY_*) inputs")
    specs = {m["spec"] for m in entry["measure"]}
    unknown = [t[0] for t in entry["tests"] if test_spec(t) is not None and test_spec(t) not in specs]
    if unknown:
        sys.exit(f"{', '.join(unknown)}: each @SPEC --test needs a --measure spec=SPEC supplying its inputs")
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


# Registries merged with `merge=union`: a line one side moved and the other side kept survives
# twice (E0428 "defined multiple times" cost 19 checks on 2026-10-26). A repeated entry line is
# always an error there, so the first occurrence is kept.
UNION_REGISTRIES = {
    "crates/server-spell-scripts/src/modules.rs": re.compile(r'^\s+\w+: \(\d+, "[^"]+"\),$'),
    "crates/server-simulation/src/dungeon_simulator/dps_parity/module_list.in": re.compile(r"^mod \w+;$"),
    "crates/server-combat/src/auras/stacked_next_spell_uses.in": re.compile(r"^\s+\d+,$"),
}


def drop_union_duplicates():
    changed = False
    for name, entry_line in UNION_REGISTRIES.items():
        path = TREE / name
        if not path.exists():
            continue
        seen, kept = set(), []
        for line in path.read_text().splitlines(keepends=True):
            key = line.rstrip("\n")
            if entry_line.match(key):
                if key in seen:
                    continue
                seen.add(key)
            kept.append(line)
        text = "".join(kept)
        if text != path.read_text():
            path.write_text(text)
            changed = True
    if changed:
        git("commit", "-qa", "--amend", "--no-edit")


def merge(entries, log):
    """Merge entries in order; return those that merged and those that conflicted."""
    merged, conflicted = [], []
    for entry in entries:
        with timed(log, "merge", tree_id(), entry=entry["id"], commit=entry["commit"]):
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
            drop_union_duplicates()
            merged.append(entry)
    with log.open("a") as out:
        out.write(f"merged {[e['id'] for e in merged]} conflicted {[e['id'] for e in conflicted]}\n")
    return merged, conflicted


def check(log):
    """Compile check. Formatting is applied, not checked: an unformatted entry would
    otherwise cost a bisect, and formatter output is never a behavior change."""
    tree = tree_id()
    with timed(log, "check", tree, scope="workspace-all-targets"):
        result = cargo(log, "check", "--locked", "--workspace", "--all-targets")
    if result != 0:
        return False
    # The workspace check unifies test-support features into every crate; the deployed server
    # binary builds without them (an ungated simc_apl_native call broke it on 2026-10-26).
    with timed(log, "check", tree, scope="server-bins"):
        result = cargo(log, "check", "--locked", "-p", "server", "--bins")
    if result != 0:
        return False
    with timed(log, "fmt", tree):
        if cargo(log, "fmt") != 0:
            return False
        # Cargo does not discover include!() or macro-generated module declarations.
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


def failing_paths(log, start):
    """Source files named by errors in the log written since byte offset `start`."""
    with log.open() as f:
        f.seek(start)
        text = f.read()
    # include!()d sources are reported through `..` hops (crates/a/src/../../b/...).
    return {os.path.normpath(p) for p in
            re.findall(r"(?:-->|Diff in) /?(?:\S*?/game-server-land/)?(crates/\S+?\.(?:rs|in))", text)}


def touches(entry, paths):
    changed = git("diff", "--name-only", f"origin/master...{entry['commit']}", check=False).stdout.split()
    return bool(paths.intersection(changed))


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

    checked_from = log.stat().st_size if log.exists() else 0
    if check(log):
        return merged, rejected
    paths = failing_paths(log, checked_from)
    # A broken master rejects everything; report that instead of blaming entries.
    reset_tree()
    if not check(log):
        raise MasterBroken(f"origin/master itself fails check; see {log}")
    # Entries touching the files the compiler or rustfmt blamed are the likely culprits: admit
    # the rest in one check first, so one broken entry costs ~3 checks instead of a full bisect.
    modules = missing_module_names(log, checked_from)
    suspects = [e for e in merged if added_module_names(e["commit"]) & modules]
    if not suspects:
        suspects = [e for e in merged if touches(e, paths)]
    if suspects and len(suspects) < len(merged):
        admit([e for e in merged if e not in suspects])
        admit(suspects)
    else:
        admit(merged)
    reset_tree()
    merge(good, log)
    return good, rejected


TEST_PARALLEL = 8


def test_executables(entry, test, entry_log, cycle_log, tree):
    """Build (under Cargo's lock) and list the test executables for one --test.
    Returns [(executable, cwd)] or None when the build fails."""
    cargo_args, _, _ = split_test(test)
    with timed(cycle_log, "no-run-build", tree, entry=entry["id"], args=cargo_args):
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


def test_spec(test):
    """The spec named by a leading `@SPEC` token of a --test, if any."""
    return test[0][1:] if test and test[0].startswith("@") else None


def split_test(test):
    """Split `--test` args into (cargo args, "--", test binary args). Cargo forwards its
    positional TESTNAME filter to the binaries, so it moves to the binary args."""
    if test_spec(test) is not None:
        test = test[1:]
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


def run_executable(entry, test, executable, cwd, entry_log, cycle_log, tree):
    """Run one test binary outside Cargo, so entries run in parallel without its build lock."""
    env = base_env() | test_env(entry, test)
    runner = shlex.split(env.pop(RUNNER, ""))
    config_env, config_runner = config_overrides(split_test(test)[0])
    env |= config_env
    runner = config_runner or runner
    _, _, test_args = split_test(test)
    command = [*runner, executable, *test_args]
    with timed(cycle_log, "test-execution", tree, entry=entry["id"], args=test_args,
               executable=executable):
        out = subprocess.run(command, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                             stderr=subprocess.STDOUT, check=False)
    with entry_log.open("a") as f:
        f.write(f"\n$ {shlex.join(command)}\n{out.stdout}")
    ran = sum(int(m) for m in re.findall(r"test result: \w+\. (\d+) passed; (?:\d+) failed", out.stdout))
    ran += sum(int(m) for m in re.findall(r"test result: \w+\. \d+ passed; (\d+) failed", out.stdout))
    # libtest interleaves panic output with "test X ... FAILED"; the closing summary lists names.
    failed = {name for block in re.findall(r"^failures:\n((?:    \S+\n)+)\ntest result:", out.stdout, re.M)
              for name in block.split()}
    return out.returncode, ran, failed, out.stdout


def test_file(name, rev):
    """Path of the file defining test function `name` (last path segment) at `rev`, or None."""
    fn = name.rsplit("::", 1)[-1]
    out = git("grep", "-l", "-w", f"fn {fn}", rev, "--", "crates", check=False).stdout.split()
    return out[0].split(":", 1)[1] if out else None


def foreign_failures_only(entry, failed):
    """A combined tree also contains other entries' tests; a broad filter can select one that
    another entry added or changed and that fails for that owner (2026-10-26: Assassination's
    in-flight Doomblade/Focus tests rejected Feral twice). A failure is foreign only when this
    entry leaves the test unchanged from master and the combined tree changes it. Comparing
    only alone vs combined misread the train's own cargo fmt of a test this entry added as
    another entry's change (2026-10-27: 59741a5cf2 landed a failing test)."""
    alone = git("merge-tree", "--write-tree", "origin/master", entry["commit"], check=False)
    if alone.returncode:
        return False
    alone_tree = alone.stdout.split()[0]

    def body(rev, path, fn):
        text = git("show", f"{rev}:{path}", check=False).stdout.splitlines()
        for i, line in enumerate(text):
            if re.search(rf"\bfn {re.escape(fn)}\b", line):
                indent = line[:len(line) - len(line.lstrip())]
                end = next((j for j in range(i + 1, len(text)) if text[j] == f"{indent}}}"), len(text))
                # The train formats the combined tree, so compare without whitespace/trailing commas.
                return re.sub(r",(?=[)\]}])", "", re.sub(r"\s+", "", "".join(text[i:end + 1])))
        return None

    def foreign(name):
        # Compared per test function, so this entry's own edits in a shared test file stay its own.
        fn = name.rsplit("::", 1)[-1]
        path = test_file(name, "HEAD")
        if path is None:
            return False
        own = body(alone_tree, path, fn)
        return own == body("origin/master", path, fn) and own != body("HEAD", path, fn)

    return bool(failed) and all(foreign(name) for name in failed)


def foreign_panics_only(entry, outputs):
    """A shared tree also holds other entries' production code. A failed run whose every panic is
    in a file this entry leaves as on master, but the tree changes, says nothing about this entry
    (2026-10-27: one probe's midnight_stat_procs.rs panic rejected three other owners' probes)."""
    panics = [path for out in outputs for path in re.findall(r"panicked at (\S+?):\d+:\d+", out)]
    alone = git("merge-tree", "--write-tree", "origin/master", entry["commit"], check=False)
    if not panics or alone.returncode:
        return False
    own = set(git("diff", "--name-only", "origin/master", alone.stdout.split()[0]).stdout.split())
    tree = set(git("diff", "--name-only", "origin/master", "HEAD").stdout.split())

    def matches(path, files):
        return any(f == path or f.endswith("/" + path) for f in files)
    return all(matches(path, tree) and not matches(path, own) for path in panics)


def run_tests(entries, log):
    # Build every entry's test binaries first (serialized by Cargo), then run them all in parallel.
    jobs, broken = [], set()
    tree = tree_id()
    for entry in entries:
        entry_log = LOGS / f"{entry['id']}.tests.log"
        entry_log.unlink(missing_ok=True)
        entry["test_log"] = str(entry_log)
        for test in entry["tests"]:
            found = test_executables(entry, test, entry_log, log, tree)
            if found is None:
                broken.add(entry["id"])
            else:
                jobs += [(entry, test, exe, cwd, entry_log, log, tree) for exe, cwd in found]
    with ThreadPoolExecutor(TEST_PARALLEL) as pool:
        codes = list(pool.map(lambda job: ((job[0]["id"], tuple(job[1])), run_executable(*job)), jobs))
    by_id = {e["id"]: e for e in entries}
    failed_names, nonzero, outputs = {}, set(), {}
    for key, (code, _, names, out) in codes:
        if code:
            nonzero.add(key[0])
            failed_names.setdefault(key[0], set()).update(names)
            outputs.setdefault(key[0], []).append(out)
    # Probes and RED twins never pass on this; they retry next cycle in a different tree.
    deferred = {i for i in nonzero - broken if not by_id[i]["land"] and foreign_panics_only(by_id[i], outputs[i])}
    # Only landings: a probe or RED twin must never turn a real failure into a pass.
    foreign = {i for i in nonzero - broken if i in failed_names and by_id[i]["land"]
               and len(failed_names[i]) and foreign_failures_only(by_id[i], failed_names[i])
               # Every nonzero run must be explained by named failures (a panic outside a test is not).
               and all(names for key, (code, _, names, _) in codes if key[0] == i and code)}
    for i in foreign:
        with Path(by_id[i]["test_log"]).open("a") as f:
            f.write(f"\nIgnored failures of tests added by other entries in this tree: {sorted(failed_names[i])}\n")
    failing = broken | (nonzero - foreign)
    # A --test that runs no test at all (a stale filter or package) is a failure, not a pass.
    ran = {}
    for key, (_, count, _, _) in codes:
        ran[key] = ran.get(key, 0) + count
    empty = {e["id"] for e in entries for t in e["tests"]
             if e["id"] not in broken and ran.get((e["id"], tuple(t)), 0) == 0}
    passed, failed = [], []
    for entry in entries:
        if entry["id"] in deferred:
            with Path(entry["test_log"]).open("a") as f:
                f.write("\nDeferred: every panic is in code only other entries in this tree change.\n")
            entry["defer"] = True
            failed.append(entry)
        elif entry["id"] in failing or entry["id"] in empty:
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


PARITY_BUILD_ARGS = ("test", "--locked", "-p", "server-simulation", "--lib", "--no-run",
                     "--message-format=json-render-diagnostics")


def parity_executable(log):
    with timed(log, "parity-build", tree_id()):
        out = subprocess.run(
            [str(AGENT_RUN), "merge-train", "cargo", *PARITY_BUILD_ARGS],
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


def measure_spec(entry, measurement, executable, target, log, tree, previous=None):
    if previous is not None:
        previous.result()
    command = [sys.executable, str(TREE / "scripts/dps_parity_measure.py"), "--executable", executable,
               "--spec", measurement["spec"], "--targets", measurement["targets"],
               "--actor", measurement.get("actor", "follower"),
               "--world-db", measurement["world_db"], "--output", measurement["output"]]
    with timed(log, "measure", tree, entry=entry["id"], spec=measurement["spec"]):
        out = subprocess.run(command, cwd=TREE,
                             env=base_env() | {"DPS_PARITY_SEEDS": MEASURE_SEEDS} | entry.get("env", {})
                             | {"CARGO_TARGET_DIR": str(target)},
                             text=True, capture_output=True, check=False)
    return [line for line in out.stdout.splitlines() if "NATIVE_DPS" in line or "FAILED" in line]


def write_results(entries):
    RESULTS.mkdir(parents=True, exist_ok=True)
    for entry in entries:
        # A passing entry whose measurement failed must not read as a clean pass (Frost 64 seed 7,
        # aabedcb472): the status is what `where`/`wait` print.
        failed = [m for m in entry.get("measurements") or [] if "FAILED" in m]
        if failed and "measurement FAILED" not in entry.get("status", ""):
            entry["status"] = f"{entry.get('status', '')}; measurement FAILED ({len(failed)})".lstrip("; ")
        (RESULTS / f"{entry['id']}.json").write_text(json.dumps(entry, indent=2))
        (RUNNING / f"{entry['id']}.json").unlink(missing_ok=True)


# While this file names an owner prefix, cycles take only that owner's entries: a broad change
# (the spell-scripts crate split) gets a cycle in which no other landing moves its seams.
SOLO = HOME / "solo"


def eligible_queue():
    paths = sorted(QUEUE.glob("*.json"))
    if not SOLO.exists():
        return paths
    # One owner prefix or entry id per line (an urgent revert can join the window).
    allowed = tuple(SOLO.read_text().split())
    return [p for p in paths if p.stem in allowed
            or json.loads(p.read_text()).get("owner", "").startswith(allowed)]


def take_queue():
    RUNNING.mkdir(parents=True, exist_ok=True)
    entries = []
    for path in eligible_queue():
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
    requeue_entries(deferred)
    passed, failed = run_tests(tree, log)
    deferred = [e for e in failed if e.pop("defer", False)]
    failed = [e for e in failed if e not in deferred]
    requeue_entries(deferred)
    for entry in failed:
        entry["status"] = "rejected"
    write_results([e for e in failed if not e["id"].endswith("-red")])
    return passed, rejected + failed


def requeue_entries(entries):
    """Back to the queue for the next cycle; a RED twin's entry is requeued by run_probes."""
    for entry in entries:
        entry.pop("reason", None)
        queued = RUNNING / f"{entry['id']}.json"
        if queued.exists():  # RED twins have no queue file
            queued.rename(QUEUE / queued.name)


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
    tree = tree_id()
    lock = threading.Lock()
    pending_entries = [len(measured)]

    def schedule(entry):
        results = [None] * len(entry["measure"])
        remaining = [len(results)]

        def finish(index, future):
            try:
                lines = future.result()
            except Exception as error:  # a failed measurement must still produce a result
                lines = error
            with lock:
                results[index] = lines
                remaining[0] -= 1
                if remaining[0] == 0:
                    failure = next((result for result in results if isinstance(result, Exception)), None)
                    entry["measurements"] = ([f"FAILED: {failure}"] if failure is not None
                                             else [line for result in results for line in result])
                    write_results([entry])
                    pending_entries[0] -= 1
                    if pending_entries[0] == 0:
                        shutil.rmtree(target, ignore_errors=True)
            MEASURING.discard(future)

        previous_by_output = {}
        for index, measurement in enumerate(entry["measure"]):
            # The parity script writes a spec-level timing file as well as target reports.
            key = (str(Path(measurement["output"]).resolve()), measurement["spec"])
            previous = previous_by_output.get(key)
            future = MEASURER.submit(measure_spec, entry, measurement, str(copy), target, log, tree, previous)
            previous_by_output[key] = future
            MEASURING.add(future)
            future.add_done_callback(lambda done, index=index: finish(index, done))

    for entry in measured:
        schedule(entry)


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


def cargo_config_files():
    """External Cargo config also affects the otherwise tree-keyed parity build."""
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    directories = [cargo_home, *(parent / ".cargo" for parent in (TREE, *TREE.parents))]
    return tuple((str(path), path.read_bytes() if path.is_file() else None)
                 for directory in directories for path in (directory / "config", directory / "config.toml"))


def isolated_measure_groups(entries):
    """Group only conflict-free entries resolving to the same parity build inputs."""
    groups = {}
    if entries:
        reset_tree()
    base = git("rev-parse", "origin/master").stdout.strip() if entries else None
    environment = tuple(sorted(base_env().items()))
    configs = cargo_config_files()
    for entry in entries:
        resolved = git("merge-tree", "--write-tree", base, entry["commit"], check=False)
        if resolved.returncode:
            key = ("conflict", entry["id"])
        else:
            tree = resolved.stdout.splitlines()[0]
            build_env = tuple(sorted((key, value) for key, value in entry.get("env", {}).items()
                                     if key.startswith(("CARGO_", "RUST", "LD_"))))
            key = (tree, PARITY_BUILD_ARGS, environment, configs, build_env)
        groups.setdefault(key, []).append(entry)
    return base, list(groups.values())


def build_isolated_group(group, log, base):
    """Check one representative; identical resolved trees have the same verdict."""
    representative, *siblings = group
    passed, failed = build_and_test([representative], log)
    # reset_tree fetches master; a changed base invalidates the grouping proof.
    if siblings and git("rev-parse", "origin/master").stdout.strip() != base:
        for entry in siblings:
            (RUNNING / f"{entry['id']}.json").rename(QUEUE / f"{entry['id']}.json")
        return passed, failed
    if passed:
        return group, []
    if failed:
        for entry in siblings:
            entry["status"] = failed[0]["status"]
            entry["reason"] = failed[0]["reason"]
        write_results(siblings)
        return [], failed + siblings
    for entry in siblings:
        (RUNNING / f"{entry['id']}.json").rename(QUEUE / f"{entry['id']}.json")
    return [], []


def without_entangled_probes(probes):
    """Probes sharing a tree with their owner's other probe, or with a probe on the same commit line,
    are tested with that probe's code too: a RED probe beside its own GREEN passes. Those wait."""
    def related(entry, other):
        same_owner = entry.get("owner") and entry.get("owner") == other.get("owner")
        return same_owner or any(git("merge-base", "--is-ancestor", a["commit"], b["commit"], check=False).returncode == 0
                                 for a, b in ((entry, other), (other, entry)))
    kept = []
    for entry in probes:
        if any(related(entry, other) for other in kept):
            (RUNNING / f"{entry['id']}.json").rename(QUEUE / f"{entry['id']}.json")
        else:
            kept.append(entry)
    return kept


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
    # Probes share one tree, so a shared probe's numbers include the other probes' changes.
    # Measure-only entries (baselines, remeasures) get a tree of their own. Probes with tests stay
    # shared: their verdict is pass/fail, and the landing measurement on master is the real number.
    isolated = [e for e in probes if e["measure"] and not e["tests"]]
    shared = without_entangled_probes([e for e in probes if e not in isolated])
    # Bound distinct isolated builds, not entries sharing a resolved parity-build tree.
    isolated_base, isolated_groups = isolated_measure_groups(isolated)
    for group in isolated_groups[ISOLATED_PER_CYCLE:]:
        for entry in group:
            (RUNNING / f"{entry['id']}.json").rename(QUEUE / f"{entry['id']}.json")
    isolated_groups = isolated_groups[:ISOLATED_PER_CYCLE]
    passed, failed = [], []
    for group in ([shared] if shared else []) + isolated_groups:
        group_passed, group_failed = (build_and_test(group, log) if group is shared
                                      else build_isolated_group(group, log, isolated_base))
        for entry in group_passed:
            entry["status"] = "probe passed"
        # Measure now, while the tree is still this group's: the next build replaces it.
        measure_then_write(group_passed, log)
        passed, failed = passed + group_passed, failed + group_failed
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
        queued = eligible_queue()
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
        entry = json.loads((RUNNING / f"{args.id}.json").read_text())
        landed = git("merge-base", "--is-ancestor", entry["commit"], "origin/master", check=False)
        if entry.get("land") and landed.returncode == 0:
            # Pushed; the result file is written when its background measurement finishes.
            print("landed on origin/master; measurement still running, keep waiting")
            return
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
    s.add_argument("--test", action="append", default=[],
                   help="cargo test arguments, quoted; a leading @SPEC uses that spec's --measure inputs")
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
