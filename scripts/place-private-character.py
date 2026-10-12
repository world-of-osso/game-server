#!/usr/bin/env python3
"""Place a private fb_* character using regression-tested Retail destination data."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess


def call_admin(executable, *arguments):
    result = subprocess.run(
        [executable, *arguments], capture_output=True, text=True, check=False
    )
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return result.stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scenario", choices=("cooldown-assist", "smartai-erland"))
    parser.add_argument("username")
    parser.add_argument("character")
    parser.add_argument("--admin", required=True, help="Installed game-server-admin executable")
    arguments = parser.parse_args()
    socket = os.environ.get("GAME_SERVER_ADMIN_SOCKET", "")
    match = re.fullmatch(r"/tmp/game-server-admin-(\d+)\.sock", socket)
    if not match or int(match[1]) == 5000:
        parser.error("GAME_SERVER_ADMIN_SOCKET must explicitly select a private port, never 5000")
    if not arguments.username.startswith("fb_"):
        parser.error("private proof requires an fb_* account")
    record = call_admin(arguments.admin, "find-character", arguments.character)
    if record.split()[0] != "username=" + arguments.username:
        parser.error("character does not belong to the specified private account")
    path = Path(__file__).with_name("private-live-positions.json")
    destination = json.loads(path.read_text())["positions"][arguments.scenario]
    coordinates = [str(destination[key]) for key in ("map", "x", "y", "z")]
    print(call_admin(arguments.admin, "teleport", arguments.character, *coordinates))
    print(json.dumps({"scenario": arguments.scenario, **destination}))


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, ValueError, KeyError, IndexError) as error:
        raise SystemExit(str(error)) from error
