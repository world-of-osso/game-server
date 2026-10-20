#!/usr/bin/env python3
"""Run the already-built in-process simulator test from its source-data root.

Build first with scripts/build-host.py --test -p server --bin game-server --no-run.
Pass the test executable Cargo prints; this script never starts game-server or clients.
"""
import argparse
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    executable = args.test_executable.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    root = Path(__file__).resolve().parent.parent
    environment = os.environ | {"DUNGEON_SIMULATOR_OUTPUT": str(output)}
    command = [str(executable), "dungeon_simulator::full_stockade_clears_all_three_bosses", "--exact", "--nocapture"]
    result = subprocess.run(command, cwd=root, env=environment, check=False)
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
