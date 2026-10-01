#!/usr/bin/env python3
"""Extract from local CASC the ground files the server loads for every spawned map.

Repeats until nothing new: lists the uncached files (`ground_files.py --missing`;
each new WDT or `_obj0` ADT names more), extracts them with `casc-local` (local CASC
only, run from a game-engine checkout for its `data/casc` tables) into the cache by
FileDataID, and skips files already there or already failed, so it resumes. Files
`casc-local` cannot extract (encrypted or absent locally) are written to `--failed`
as `map id<TAB>path`.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
BATCH = 2000
EXTRACTED = re.compile(r"^Extracted FDID (\d+) -> ")


def missing_files(world_db, cache):
    listed = subprocess.run(
        [sys.executable, SCRIPTS / "ground_files.py", "--world-db", world_db, "--cache", cache, "--missing"],
        capture_output=True, text=True, check=True,
    )
    print(listed.stderr.strip(), flush=True)
    rows = [line.split("\t") for line in listed.stdout.splitlines() if line]
    return [(int(map_id), path) for map_id, path in rows]


def extract(fdids, out_dir, engine):
    """Writes `fdids` into `out_dir`, reporting progress."""
    done = set()
    for start in range(0, len(fdids), BATCH):
        batch = fdids[start : start + BATCH]
        run = subprocess.run(
            ["casc-local", *map(str, batch), "-o", out_dir],
            cwd=engine, capture_output=True, text=True,
        )
        for line in run.stdout.splitlines() + run.stderr.splitlines():
            match = EXTRACTED.match(line)
            if match:
                done.add(int(match.group(1)))
        print(f"  {out_dir.name}: {len(done)} of {start + len(batch)} extracted", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True, help="game-engine/data")
    parser.add_argument("--engine", type=Path, required=True, help="game-engine checkout (data/casc)")
    parser.add_argument("--failed", type=Path, required=True)
    args = parser.parse_args()
    failed = {}
    if args.failed.is_file():
        for line in args.failed.read_text().splitlines():
            map_id, path = line.split("\t")
            failed[path] = int(map_id)
    while True:
        todo = [(map_id, path) for map_id, path in missing_files(args.world_db, args.cache) if path not in failed]
        if not todo:
            break
        for directory in ("terrain", "models"):
            paths = sorted({path for _, path in todo if path.startswith(directory + "/")})
            fdids = [int(Path(path).stem) for path in paths]
            extract(fdids, args.cache / directory, args.engine)
            # A file written under another sniffed extension is not where the server reads it.
            for map_id, path in todo:
                if path.startswith(directory + "/") and not (args.cache / path).is_file():
                    failed.setdefault(path, map_id)
        args.failed.write_text("".join(f"{map_id}\t{path}\n" for path, map_id in sorted(failed.items())))
        print(f"{len(failed)} failed so far", flush=True)


if __name__ == "__main__":
    main()
