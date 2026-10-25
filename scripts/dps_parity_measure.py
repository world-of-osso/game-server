#!/usr/bin/env python3
"""Build the optimized parity test executable once, then measure every target count in parallel.

Each count runs `dps_parity_native_bis_dummy_records_300s` in its own process through
scripts/dps_parity_native.py (same read-only snapshot view), so reports are those of
the single-count harness. The executable uses Cargo's `test` profile (server crates at
opt-level 2 with debug assertions and overflow checks). Logs and reports land in
--output as <spec>.native.<count>.{log,json}; timings in <spec>.native.timing.json.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
TEST = "dungeon_simulator::dps_parity::dps_parity_native_bis_dummy_records_300s"


def build_executable(jobs):
    """Return the server-simulation test executable Cargo reports for this checkout."""
    command = [sys.executable, str(ROOT / "scripts/build-host.py"), "--build-host", "native",
               "--test", "-p", "server-simulation", "--lib", "--no-run",
               "--message-format=json-render-diagnostics"]
    # Native build-host supplies a checkout-local environment default. Cargo's
    # explicit option preserves a caller-selected shared dependency pool.
    target = os.environ.get("CARGO_TARGET_DIR")
    if target is not None:
        command += ["--target-dir", str(Path(target).resolve())]
    if jobs is not None:
        command += ["-j", str(jobs)]
    result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, text=True, check=False)
    if result.returncode:
        raise SystemExit(f"parity build failed with exit {result.returncode}")
    executables = [
        message["executable"]
        for line in result.stdout.splitlines() if line.startswith("{")
        for message in [json.loads(line)]
        if message.get("reason") == "compiler-artifact" and message.get("executable")
        and message["target"]["name"] == "server_simulation"
    ]
    if len(executables) != 1:
        raise SystemExit(f"expected one server_simulation test executable, found {executables}")
    return executables[0]


def measure_count(executable, args, count):
    environment = os.environ | {
        "DPS_PARITY_WORLD_DB": str(args.world_db),
        "DPS_PARITY_OUTPUT": str(args.output),
        "DPS_PARITY_SPEC": str(args.spec),
        "DPS_PARITY_TARGETS": str(count),
        "DPS_PARITY_ACTOR": args.actor,
    }
    command = [sys.executable, str(ROOT / "scripts/dps_parity_native.py"), executable, TEST,
               "--exact", "--ignored", "--nocapture", "--test-threads=1"]
    report = args.output / f"{args.spec}.native.{count}.json"
    # A failed run must not leave a previous invocation's report in place.
    report.unlink(missing_ok=True)
    started = time.monotonic()
    with (args.output / f"{args.spec}.native.{count}.log").open("w") as log:
        code = subprocess.run(command, cwd=ROOT, env=environment, stdout=log,
                              stderr=subprocess.STDOUT, check=False).returncode
    seconds = time.monotonic() - started
    try:
        tick_loop = json.loads(report.read_text()).get("wall_seconds")
    except (OSError, ValueError):
        tick_loop = None
    return {"exit_code": code, "process_seconds": seconds, "tick_loop_seconds": tick_loop}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--spec", type=int, required=True)
    parser.add_argument("--targets", default="1,3", help="comma-separated counts (1 and/or 3)")
    parser.add_argument("--actor", choices=("follower", "player"), default="follower")
    parser.add_argument("--world-db", type=Path, required=True, help="private prepared world.db")
    parser.add_argument("--output", type=Path, required=True,
                        help="directory holding <spec>.profile.json/selected traits; receives reports")
    parser.add_argument("--jobs", type=int, help="Cargo jobs for the build step")
    args = parser.parse_args()
    counts = [int(count) for count in args.targets.split(",")]
    if not counts or any(count not in (1, 3) for count in counts):
        parser.error("--targets accepts 1 and/or 3")
    args.world_db = args.world_db.resolve(strict=True)
    args.output = args.output.resolve(strict=True)

    started = time.monotonic()
    executable = build_executable(args.jobs)
    built = time.monotonic()
    with ThreadPoolExecutor(max_workers=len(counts)) as pool:
        results = dict(zip(counts, pool.map(lambda count: measure_count(executable, args, count), counts)))
    timing = {"executable": executable, "build_seconds": built - started,
              "measure_seconds": time.monotonic() - built, "total_seconds": time.monotonic() - started,
              "counts": {str(count): result for count, result in results.items()}}
    (args.output / f"{args.spec}.native.timing.json").write_text(json.dumps(timing, indent=2) + "\n")
    for count in counts:
        log = (args.output / f"{args.spec}.native.{count}.log").read_text()
        # --nocapture interleaves the test's stderr summary with libtest's status line.
        lines = [line[line.index("NATIVE_DPS"):] for line in log.splitlines() if "NATIVE_DPS" in line]
        print(*lines or [f"spec{args.spec} targets{count}: FAILED, see {args.output}/{args.spec}.native.{count}.log"])
    print(json.dumps(timing, indent=2))
    return 1 if any(result["exit_code"] for result in results.values()) else 0


if __name__ == "__main__":
    raise SystemExit(main())
