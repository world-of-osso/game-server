#!/usr/bin/env python3
"""Extract selected maps' ground closure from build-selected local CASC products.

Default selection is spawned maps; --map includes registered maps without spawns.
Each pass discovers WDT MAID, obj0 MODF and WMO GFID dependencies. Extraction
uses the Map row's build provenance and never a CDN. Failed files are recorded.
"""

import argparse
import os
import subprocess
from pathlib import Path

from ground_files import GroundFiles, NO_WDT, add_map_selection, chunks, selected_map_sources

BATCH = 2000


def missing_files(maps, cache):
    files = GroundFiles(cache)
    for map_id, wdt, _product in maps:
        files.start_map(map_id)
        if wdt:
            files.add_map(wdt)
        else:
            files.missing.append((map_id, NO_WDT))
    print(f"{len(maps)} selected maps: {len(files.present)} cached, {len(files.missing)} missing", flush=True)
    return files.missing


def normalize_extracted_file(cache, relative):
    """Unlisted Forever ADT/WMO files are sniffed as .dat; validate before naming."""
    destination = cache / relative
    if destination.is_file():
        return True
    sniffed = destination.with_suffix(".dat")
    if not sniffed.is_file():
        return False
    found = chunks(sniffed.read_bytes())
    required = {".wdt": b"MPHD", ".adt": b"MVER", ".wmo": b"MVER"}[destination.suffix]
    if required not in found:
        raise ValueError(f"{sniffed}: not a {destination.suffix} chunk file")
    if destination.suffix == ".wmo" and not ({b"MOHD", b"MOGP"} & found.keys()):
        raise ValueError(f"{sniffed}: no WMO root/group header")
    sniffed.rename(destination)
    return True


def extract(paths, cache, engine, product, executable):
    """Extract one product/directory batch and normalize server cache names."""
    out_dir = cache / Path(paths[0]).parent
    out_dir.mkdir(parents=True, exist_ok=True)
    environment = os.environ | {"WOW_PRODUCT": product}
    for start in range(0, len(paths), BATCH):
        batch = paths[start : start + BATCH]
        result = subprocess.run(
            [str(executable), *(Path(path).stem for path in batch), "-o", str(out_dir)],
            cwd=engine, env=environment, capture_output=True, text=True,
        )
        print(result.stdout, end="", flush=True)
        print(result.stderr, end="", flush=True)
        present = sum(normalize_extracted_file(cache, path) for path in batch)
        print(f"{product} {out_dir.name}: {present}/{len(batch)} cached; exit {result.returncode}", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--engine", type=Path, required=True, help="checkout with local data/casc")
    parser.add_argument("--casc-local", type=Path, default=Path("casc-local"))
    parser.add_argument("--failed", type=Path, required=True)
    add_map_selection(parser)
    args = parser.parse_args()
    maps = selected_map_sources(args.world_db, args.map_ids, args.registered)
    products = {map_id: product for map_id, _wdt, product in maps}
    failed = set()
    if args.failed.is_file():
        for line in args.failed.read_text().splitlines():
            map_id, path = line.split("\t")
            failed.add((int(map_id), path))
    while True:
        todo = [(map_id, path) for map_id, path in missing_files(maps, args.cache)
                if (map_id, path) not in failed]
        if not todo:
            break
        for product in sorted(set(products.values())):
            for directory in ("terrain", "models"):
                paths = sorted({path for map_id, path in todo
                                if products[map_id] == product and path.startswith(directory + "/")})
                if paths:
                    extract(paths, args.cache, args.engine, product, args.casc_local)
        failed.update((map_id, path) for map_id, path in todo if not (args.cache / path).is_file())
        args.failed.write_text("".join(f"{map_id}\t{path}\n" for map_id, path in sorted(failed)))
        print(f"{len(failed)} failed so far", flush=True)
    return int(any(map_id in products for map_id, _path in failed))


if __name__ == "__main__":
    raise SystemExit(main())
