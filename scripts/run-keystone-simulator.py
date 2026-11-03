#!/usr/bin/env python3
"""Run a native headless keystone.

Progress is printed as KEYSTONE_PROGRESS lines. The party field lists
entity:health_percent:mana_percent:alive_or_dead entries (na means unavailable).
Pull decisions are flushed to <output>.pulls.jsonl while the run is active; the full JSON record is written
at exit. The journal is replaced when a new run uses the same output path.
"""
import argparse
import os
from pathlib import Path
import subprocess


def positive_u32(value):
    try:
        number = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("expected an integer") from error
    if not 0 < number <= 2**32 - 1:
        raise argparse.ArgumentTypeError("expected a positive u32")
    return number


def party_specs(value):
    parts = value.split(",")
    if len(parts) != 5:
        raise argparse.ArgumentTypeError("supply tank, healer and three damage spec IDs")
    return [positive_u32(part.strip()) for part in parts]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--test-executable", type=Path, required=True)
    parser.add_argument("--dungeon", type=positive_u32, required=True, help="native map ID")
    parser.add_argument("--key-level", type=positive_u32, required=True)
    parser.add_argument("--party", type=party_specs, required=True, help="tank,healer,damage,damage,leader-damage spec IDs")
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--schedule-epoch", type=int, default=1792454400,
                        help="Unix seconds for the affix schedule (default: fixed simulator epoch)")
    parser.add_argument("--output", type=Path, required=True, help="JSON run-record file")
    args = parser.parse_args()
    if not 0 <= args.seed <= 2**64 - 1:
        parser.error("seed must fit u64")
    executable = args.test_executable.resolve(strict=True)
    output = args.output.resolve()
    environment = os.environ | {
        "DUNGEON_SIMULATOR_MAP": str(args.dungeon),
        "KEYSTONE_SIMULATOR_LEVEL": str(args.key_level),
        "KEYSTONE_SIMULATOR_PARTY": ",".join(map(str, args.party)),
        "KEYSTONE_SIMULATOR_SEED": str(args.seed),
        "KEYSTONE_SIMULATOR_SCHEDULE_EPOCH": str(args.schedule_epoch),
        "KEYSTONE_SIMULATOR_OUTPUT": str(output),
    }
    command = [str(executable), "dungeon_simulator::keystone::tests::run_headless_keystone", "--exact", "--ignored", "--nocapture"]
    root = Path(__file__).resolve().parent.parent
    return subprocess.run(command, cwd=root, env=environment, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
