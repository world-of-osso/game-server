#!/usr/bin/env python3
"""Characterize the existing shared-process libtest execution boundary."""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import resource
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("filters", nargs="+")
    args = parser.parse_args()
    args.output.mkdir(parents=True)
    artifacts = [json.loads(line) for line in args.artifacts.read_text().splitlines() if line.startswith("{")]
    binary = next(row["executable"] for row in artifacts if row.get("executable") and row["profile"]["test"])
    env = dict(os.environ, RUSTC_BOOTSTRAP="1")
    command = [binary, *args.filters, "-Z", "unstable-options", "--format", "json"]
    discovery = subprocess.run([*command, "--list"], env=env, capture_output=True, text=True, check=True)
    expected = [json.loads(line) for line in discovery.stdout.splitlines() if line.startswith("{")]
    started = time.monotonic()
    result = subprocess.run(command, env=env, capture_output=True, text=True)
    rows = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    actual = Counter(row["name"] for row in rows if row.get("type") == "test" and row.get("event") in ["ok", "failed", "ignored"])
    missing = list((Counter(row["name"] for row in expected if row.get("event") == "discovered") - actual).elements())
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    summary = {"passed": sum(row.get("event") == "ok" for row in rows), "failed": sum(row.get("event") == "failed" for row in rows), "ignored": sum(row.get("event") == "ignored" for row in rows), "missing": missing, "groups": [{"cpu_seconds": usage.ru_utime + usage.ru_stime, "wall_seconds": time.monotonic() - started}]}
    (args.output / "summary.json").write_text(json.dumps(summary, indent=2))
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
