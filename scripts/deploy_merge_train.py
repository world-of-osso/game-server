#!/usr/bin/env python3
"""Deploy scripts/merge_train.py as the running merge train.

Copies the script to the train home and makes sure the `merge-train` user unit runs it.
A running train re-executes itself between cycles when its file changes, so a deploy
never interrupts a cycle; --restart stops the current cycle instead (its entries are
requeued and rebuilt).
"""
import argparse
import os
from pathlib import Path
import py_compile
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parent.parent
HOME = Path("/home/osso/.worktrees/train")
DEPLOYED = HOME / "merge_train.py"
UNIT = "merge-train"


def systemctl(*args):
    return subprocess.run(["systemctl", "--user", *args], text=True, capture_output=True, check=False)


def is_active():
    return systemctl("is-active", UNIT).stdout.strip() == "active"


def start():
    systemctl("reset-failed", UNIT)
    subprocess.run(["systemd-run", "--user", f"--unit={UNIT}", "--slice=agents.slice",
                    "--property=Restart=on-failure", sys.executable, str(DEPLOYED), "run"],
                   check=True, capture_output=True)


def install(source):
    # Replace atomically: the running train must never read a half-written file.
    staged = DEPLOYED.with_suffix(".staged")
    shutil.copyfile(source, staged)
    os.replace(staged, DEPLOYED)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=ROOT / "scripts/merge_train.py")
    parser.add_argument("--restart", action="store_true", help="stop the current cycle now")
    args = parser.parse_args()
    source = args.source.resolve(strict=True)
    py_compile.compile(str(source), doraise=True)
    HOME.mkdir(parents=True, exist_ok=True)

    if args.restart and is_active():
        systemctl("stop", UNIT)
    install(source)
    if not is_active():
        start()
        time.sleep(2)
        if not is_active():
            raise SystemExit(f"{UNIT} failed to start: journalctl --user -u {UNIT}")
        print(f"started {UNIT} with {source}")
    else:
        print(f"installed {source}; the running train reloads it after the current cycle")


if __name__ == "__main__":
    main()
