#!/usr/bin/env python3
"""Run isolated seeded simulator tests concurrently and merge their reports.

Build first with scripts/build-host.py --test -p server --bin game-server --no-run.
Pass the test executable Cargo prints; this never starts game-server or clients.
Each seed writes run-SEED.json and seed-SEED.log in the shared output directory.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import json
import os
from pathlib import Path
import subprocess
import sys
import time


def default_jobs(runs):
    """Leave half the estimated idle CPU capacity for other work."""
    cores = len(os.sched_getaffinity(0))
    idle_cores = max(0.0, cores - os.getloadavg()[0])
    return min(runs, max(1, int(idle_cores / 2)))


def run_seed(executable, output, root, seed):
    # A failed worker must not leave a previous invocation's successful receipt.
    (output / f"run-{seed}.json").unlink(missing_ok=True)
    environment = os.environ | {
        "DUNGEON_SIMULATOR_OUTPUT": str(output),
        "DUNGEON_SIMULATOR_RUNS": "1",
        "DUNGEON_SIMULATOR_SEED": str(seed),
    }
    command = [str(executable), "dungeon_simulator::batch::run_stockade_batch",
               "--exact", "--ignored", "--nocapture"]
    with (output / f"seed-{seed}.log").open("w") as log:
        result = subprocess.run(command, cwd=root, env=environment,
                                stdout=log, stderr=subprocess.STDOUT, check=False)
    return seed, result.returncode


def summarize(reports):
    """Same reached-pack denominators and failure distinction as the native batch."""
    packs = {}
    for report in reports:
        failed_pack = report["failed_pack"]
        reached = {pull["pack"] for pull in report["pulls"]}
        if failed_pack is not None:
            reached.add(failed_pack)
        for pack in reached:
            counts = packs.setdefault(str(pack), {"runs_reaching": 0, "wipes": 0, "failures": 0})
            counts["runs_reaching"] += 1
            counts["wipes"] += failed_pack == pack and report["status"] == "wiped"
            counts["failures"] += failed_pack == pack and report["status"] != "cleared"
    for counts in packs.values():
        counts["wipe_rate"] = counts["wipes"] / counts["runs_reaching"]
        counts["failure_rate"] = counts["failures"] / counts["runs_reaching"]
    def count(status):
        return sum(report["status"] == status for report in reports)
    return {
        "runs": len(reports), "seeds": [report["seed"] for report in reports],
        "cleared": count("cleared"), "wiped": count("wiped"),
        "stalled": count("stalled"), "blocked": count("blocked"),
        "timed_out": count("timeout"), "failed_runs": len(reports) - count("cleared"),
        "packs": dict(sorted(packs.items(), key=lambda item: int(item[0]))),
    }


def read_reports(output, seeds):
    reports, errors = [], []
    for seed in seeds:
        path = output / f"run-{seed}.json"
        try:
            report = json.loads(path.read_text())
            if report["seed"] != seed:
                raise ValueError(f"expected seed{seed}, found{report['seed']}")
            reports.append({key: report[key] for key in ("seed", "status", "failed_pack", "pulls")})
        except (OSError, ValueError, KeyError) as error:
            errors.append({"seed": seed, "error": str(error)})
            print(f"Seed{seed} report unavailable at {path}: {error}", file=sys.stderr)
    return reports, errors


def run_batch(executable, output, root, seeds, jobs):
    started = time.monotonic()
    failures = []
    print(f"SIMULATOR seeds={len(seeds)} jobs={jobs} output={output}", flush=True)
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        pending = [pool.submit(run_seed, executable, output, root, seed) for seed in seeds]
        for future in as_completed(pending):
            seed, code = future.result()
            print(f"SEED seed={seed} exit={code} log={output / f'seed-{seed}.log'}", flush=True)
            if code != 0:
                failures.append({"seed": seed, "exit_code": code})
    reports, errors = read_reports(output, seeds)
    summary = summarize(reports)
    summary.update({"requested_runs": len(seeds), "missing_seeds": [error["seed"] for error in errors],
                    "report_errors": errors, "process_failures": sorted(failures, key=lambda row: row["seed"]),
                    "jobs": jobs, "batch_wall_seconds": time.monotonic() - started})
    # Native single-seed summaries can race during execution; replace them only
    # after every worker has exited, with one complete ordered aggregation.
    temporary = output / "summary.json.tmp"
    temporary.write_text(json.dumps(summary, indent=2) + "\n")
    temporary.replace(output / "summary.json")
    print("BATCH " + json.dumps(summary), flush=True)
    return 101 if failures or errors or summary["failed_runs"] else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=1)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--jobs", type=int, help="parallel seed processes; default: half estimated idle cores")
    args = parser.parse_args()
    if args.runs < 1 or args.seed < 0 or args.seed + args.runs - 1 > 2**64 - 1:
        parser.error("--runs must be positive and the seed range must fit u64")
    if args.jobs is not None and args.jobs < 1:
        parser.error("--jobs must be positive")
    executable = args.test_executable.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    jobs = min(args.runs, args.jobs) if args.jobs is not None else default_jobs(args.runs)
    root = Path(__file__).resolve().parent.parent
    return run_batch(executable, output, root, range(args.seed, args.seed + args.runs), jobs)


if __name__ == "__main__":
    raise SystemExit(main())
