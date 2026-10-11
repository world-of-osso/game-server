#!/usr/bin/env python3
"""Copy, recover, export and atomically publish one public Armory snapshot."""
import argparse
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import random
import shlex
import shutil
import subprocess
import sys
import tempfile
import time

HOME = Path.home()
REMOTE = "sakuin"
SSH = ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", REMOTE]


def execute(argv, *, destination=None, retry_transport=False):
    print("+ " + shlex.join(map(str, argv)), flush=True)
    for attempt in range(3):
        if destination is None:
            result = subprocess.run(argv, check=False)
        else:
            with destination.open("wb") as output:
                result = subprocess.run(argv, stdout=output, check=False)
        if result.returncode == 0:
            return
        if not retry_transport or result.returncode != 255 or attempt == 2:
            raise subprocess.CalledProcessError(result.returncode, argv)
        time.sleep(2 ** attempt + random.uniform(0, 0.5))


def copy_source(args, run_dir):
    source = run_dir / "download.redb"
    if args.source_redb:
        shutil.copyfile(args.source_redb, source)
    else:
        execute(SSH + ["sudo -n cat /var/lib/game-server/data/game.redb"],
                destination=source, retry_transport=True)
    if source.stat().st_size == 0:
        raise ValueError("empty production redb copy")
    return source


def validate_references(value, snapshot):
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"url", "icon_url"} and child is not None:
                relative = Path(child)
                if relative.is_absolute() or ".." in relative.parts:
                    raise ValueError(f"invalid snapshot reference: {child}")
                if not (snapshot / relative).is_file():
                    raise ValueError(f"missing snapshot reference: {child}")
            validate_references(child, snapshot)
    elif isinstance(value, list):
        for child in value:
            validate_references(child, snapshot)


def validate_snapshot(snapshot):
    index = json.loads((snapshot / "index.json").read_text())
    for name in ["rankings/mythic-plus.json", "rankings/raids.json"]:
        if not (snapshot / name).is_file():
            raise ValueError(f"missing snapshot document: {name}")
    envelope = (1, "world-of-osso", index["as_of"])
    for path in snapshot.rglob("*.json"):
        document = json.loads(path.read_text())
        actual = (document["schema_version"], document["realm"], document["as_of"])
        if actual != envelope:
            raise ValueError(f"inconsistent snapshot envelope: {path}")
        validate_references(document, snapshot)
    print(f"validated snapshot: {snapshot}; characters={len(index['characters'])}; as_of={index['as_of']}", flush=True)


def publish_snapshot(snapshot, name):
    stage = f"osso-data-stage/{name}"
    execute(SSH + [f"mkdir -p ~/{stage}"], retry_transport=True)
    execute(["scp", "-o", "BatchMode=yes", "-o", "ConnectTimeout=20", "-r",
             str(snapshot) + "/.", f"{REMOTE}:{stage}/"], retry_transport=True)
    # GNU mv exchange is one atomic renameat2 operation, not two renames with a gap.
    command = f'''set -eu
stage="$HOME/{stage}"
data=/var/lib/osso-website/www/data
new=/var/lib/osso-website/www/data.new
old=/var/lib/osso-website/www/data.old
sudo -n rm -rf "$new"
sudo -n cp -a "$stage" "$new"
sudo -n chmod -R a+rX "$new"
if [ -e "$data" ]; then
    sudo -n mv -T --exchange --no-copy "$new" "$data"
    sudo -n rm -rf "$old"
    sudo -n mv -T --no-copy "$new" "$old"
else
    sudo -n mv -T --no-copy "$new" "$data"
fi
rm -rf "$stage"
'''
    # Do not retry the non-idempotent exchange: a lost reply may mean it succeeded.
    execute(SSH + [command])
    print("published complete snapshot; previous snapshot retained as data.old", flush=True)


def prune_runs(runs, keep=2):
    old = sorted(path for path in runs.glob("run-*") if path.is_dir())
    for path in old[:-keep]:
        shutil.rmtree(path)


def refresh(args):
    os.umask(0o077)
    args.state_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
    with (args.state_dir / "refresh.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        runs = args.state_dir / "runs"
        runs.mkdir(exist_ok=True)
        prune_runs(runs)
        stamp = datetime.now(timezone.utc).strftime("run-%Y%m%dT%H%M%S%fZ-")
        run_dir = Path(tempfile.mkdtemp(prefix=stamp, dir=runs))
        source = copy_source(args, run_dir)
        repaired = run_dir / "repaired.redb"
        execute([str(args.exporter), "--repair-copy", str(source), str(repaired)])
        snapshot = run_dir / "snapshot"
        execute([str(args.exporter), "--redb", str(repaired), "--world-db", str(args.world_db),
                 "--opt-outs", str(args.opt_outs), "--textures", str(args.textures),
                 "--output", str(snapshot), "--realm", "world-of-osso",
                 "--realm-name", "World of Osso"])
        validate_snapshot(snapshot)
        if args.dry_run:
            print(f"DRY RUN: local snapshot {snapshot}; no staging or publishing", flush=True)
        else:
            publish_snapshot(snapshot, run_dir.name)


def parse_args():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="copy and export locally; never upload")
    parser.add_argument("--source-redb", type=Path, help="use a local input instead of production SSH")
    parser.add_argument("--exporter", type=Path, default=HOME / ".local/lib/game-server-site-refresh/game-server-site-export")
    parser.add_argument("--state-dir", type=Path, default=HOME / ".local/state/game-server-site-refresh")
    parser.add_argument("--opt-outs", type=Path, default=HOME / ".config/game-server-site-refresh/opt-outs.json")
    parser.add_argument("--world-db", type=Path, default=Path("/home/osso/Projects/world-of-osso/game-server/data/world.db"))
    parser.add_argument("--textures", type=Path, default=Path("/home/osso/Projects/world-of-osso/game-engine/data/textures"))
    return parser.parse_args()


if __name__ == "__main__":
    try:
        refresh(parse_args())
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"site refresh failed: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
