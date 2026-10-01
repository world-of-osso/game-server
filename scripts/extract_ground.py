#!/usr/bin/env python3
"""Extract the ground files the client cache lacks from local CASC (never the CDN).

Loops `ground_files.py --missing` -> `casc-local` (run from a game-engine checkout, which
holds the CASC resolution tables) until a pass finds nothing new to extract: each
extracted WDT/ADT/WMO names more files. Files already cached are never re-extracted, so
a rerun resumes. Files CASC cannot produce (absent locally, or encrypted with an unknown
TACT key: casc-local zero-fills those, and they are discarded) are written to `--gaps` as
`<map id>\\t<path>\\t<absent|encrypted>`, the known-gap list the server reads at startup.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
BATCH = 500
STOP_BYTES = 20 * 10**9
EXTRACTED = re.compile(r"^Extracted FDID (\d+) -> (.+)$")
ENCRYPTED = re.compile(r"^FDID (\d+) missing TACT keys")
FAILED = re.compile(r"^Failed FDID (\d+): (.*)$")


def missing_files(world_db, cache):
    run = subprocess.run(
        [sys.executable, SCRIPTS / "ground_files.py", "--world-db", world_db, "--cache", cache, "--missing"],
        capture_output=True, text=True, check=True,
    )
    print(run.stderr.strip(), file=sys.stderr)
    return [tuple(line.split("\t")) for line in run.stdout.splitlines() if line]


def extract_batch(casc_local, engine, stage, fdids):
    """casc-local's outcome per FDID: a staged path, "encrypted" or "absent"."""
    run = subprocess.run(
        [casc_local, *map(str, fdids), "-o", stage], cwd=engine, capture_output=True, text=True
    )
    outcome = {}
    for line in run.stderr.splitlines():
        if match := EXTRACTED.match(line):
            outcome.setdefault(int(match[1]), Path(match[2]))
        elif match := ENCRYPTED.match(line):
            outcome[int(match[1])] = "encrypted"
        elif match := FAILED.match(line):
            outcome[int(match[1])] = "absent"
            print(f"FDID {match[1]}: {match[2]}", file=sys.stderr)
    for fdid in fdids:
        if fdid not in outcome:
            raise SystemExit(f"casc-local reported nothing for FDID {fdid}:\n{run.stderr[-2000:]}")
    return outcome


def cache_size(cache):
    return sum(path.stat().st_size for dir in ("terrain", "models") for path in (cache / dir).iterdir())


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True, help="game-engine/data")
    parser.add_argument("--engine", type=Path, required=True, help="game-engine checkout")
    parser.add_argument("--casc-local", type=Path, required=True)
    parser.add_argument("--gaps", type=Path, required=True)
    args = parser.parse_args()
    cache = args.cache.resolve()
    stage = cache / ".ground-extract"
    stage.mkdir(exist_ok=True)
    for leftover in stage.iterdir():  # an interrupted run's batch
        leftover.unlink()
    gaps = {}
    start_size = cache_size(cache)
    while True:
        todo = [(map_id, path) for map_id, path in missing_files(args.world_db, cache) if path not in gaps]
        if not todo:
            break
        for start in range(0, len(todo), BATCH):
            batch = todo[start : start + BATCH]
            by_fdid = {int(Path(path).stem): (map_id, path) for map_id, path in batch}
            outcome = extract_batch(args.casc_local.resolve(), args.engine, stage, list(by_fdid))
            for fdid, result in outcome.items():
                map_id, path = by_fdid[fdid]
                if isinstance(result, Path):
                    result.rename(cache / path)
                else:
                    gaps[path] = (map_id, result)
                    for staged in stage.glob(f"{fdid}.*"):
                        staged.unlink()
            grown = cache_size(cache) - start_size
            print(f"extracted {start + len(batch)}/{len(todo)}, +{grown / 1e9:.2f} GB, {len(gaps)} gaps", file=sys.stderr)
            if grown > STOP_BYTES:
                raise SystemExit(f"stopped: extraction grew the cache by {grown / 1e9:.1f} GB (> 20 GB)")
    stage.rmdir()
    rows = sorted(gaps.items(), key=lambda item: (int(item[1][0]), item[0]))
    args.gaps.write_text("".join(f"{map_id}\t{path}\t{reason}\n" for path, (map_id, reason) in rows))
    print(f"done: +{(cache_size(cache) - start_size) / 1e9:.2f} GB, {len(gaps)} gaps", file=sys.stderr)


if __name__ == "__main__":
    main()
