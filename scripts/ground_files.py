#!/usr/bin/env python3
"""List server ground files for spawned, explicit, or all registered maps.

crates/server/src/ground.rs reads, from the client asset cache: each map's WDT
(`terrain/<Map.WdtFileDataID>.wdt`); for a global-WMO map (MPHD flag 0x1) the WDT MODF
WMO, else every MAID tile's root and `_obj0` ADT (`terrain/<fdid>.adt`); each `_obj0`
MODF WMO placed by FileDataID (flag 0x8) and its GFID group files (`models/<fdid>.wmo`).

Prints the cached files' paths relative to the cache, one per line (rsync
--files-from); `--missing` prints the uncached ones with their map instead. The summary on stderr counts the files the cache does not hold: those
tiles and WMOs have no ground on the server until they are extracted.
"""

import argparse
import sqlite3
import struct
import sys
from contextlib import closing
from pathlib import Path

MPHD_FLAG_GLOBAL_WMO = 0x1
MAID_ENTRY_SIZE = 32
MODF_ENTRY_SIZE = 64
MODF_FLAG_FILEDATAID = 0x8
# What the server's `GroundMap::missing_files` reports for a map without a WDT.
NO_WDT = "no WDT FileDataID"


def chunks(data):
    """Chunk payloads by tag (tags are stored reversed); the first of each tag wins."""
    found = {}
    offset = 0
    while offset + 8 <= len(data):
        tag = data[offset : offset + 4][::-1]
        size = struct.unpack_from("<I", data, offset + 4)[0]
        found.setdefault(tag, data[offset + 8 : offset + 8 + size])
        offset += 8 + size
    return found


def modf_wmos(modf, by_fdid):
    """WMO FileDataIDs placed by a MODF; `by_fdid` keeps only entries flagged 0x8."""
    wmos = []
    for offset in range(0, len(modf) - MODF_ENTRY_SIZE + 1, MODF_ENTRY_SIZE):
        fdid = struct.unpack_from("<I", modf, offset)[0]
        flags = struct.unpack_from("<H", modf, offset + 56)[0]
        if fdid and (not by_fdid or flags & MODF_FLAG_FILEDATAID):
            wmos.append(fdid)
    return wmos


class GroundFiles:
    def __init__(self, cache):
        self.cache = cache
        self.present = {}
        # (map id, path) of each file a map needs that the cache does not hold: a WMO
        # several maps place is missing on each of them, as the server checks per map.
        self.missing = []
        self.seen = set()
        self.map_id = None

    def start_map(self, map_id):
        self.map_id = map_id
        self.seen = set()

    def read(self, relative):
        """The file's bytes once per map (None if already read or not cached)."""
        if relative in self.seen:
            return None
        self.seen.add(relative)
        path = self.cache / relative
        if not path.is_file():
            self.missing.append((self.map_id, relative))
            return None
        self.present[relative] = None
        return path.read_bytes()

    def add_wmo(self, fdid):
        root = self.read(f"models/{fdid}.wmo")
        if root is None:
            return
        found = chunks(root)
        # The root's MOHD group count of GFID entries (`group_file_data_ids`).
        count = struct.unpack_from("<I", found.get(b"MOHD", bytes(8)), 4)[0]
        gfid = found.get(b"GFID", b"")
        for offset in range(0, min(len(gfid), count * 4) - 3, 4):
            group = struct.unpack_from("<I", gfid, offset)[0]
            if group:
                self.read(f"models/{group}.wmo")

    def add_map(self, wdt_fdid):
        wdt = self.read(f"terrain/{wdt_fdid}.wdt")
        if wdt is None:
            return
        found = chunks(wdt)
        mphd = struct.unpack_from("<I", found.get(b"MPHD", b"\0\0\0\0"), 0)[0]
        if mphd & MPHD_FLAG_GLOBAL_WMO:
            for wmo in modf_wmos(found.get(b"MODF", b""), by_fdid=False)[:1]:
                self.add_wmo(wmo)
            return
        maid = found.get(b"MAID", b"")
        for offset in range(0, len(maid) - MAID_ENTRY_SIZE + 1, MAID_ENTRY_SIZE):
            root, obj0 = struct.unpack_from("<II", maid, offset)
            if root:
                self.read(f"terrain/{root}.adt")
            if obj0:
                placements = self.read(f"terrain/{obj0}.adt")
                if placements is not None:
                    modf = chunks(placements).get(b"MODF", b"")
                    for wmo in modf_wmos(modf, by_fdid=True):
                        self.add_wmo(wmo)


# Local CASC product of the pinned row overlay, keyed by provenance, not map ID.
BUILD_PRODUCTS = {"1.60.1.70205": "wow_classic_beta"}


def selected_map_sources(world_db, map_ids=None, registered=False):
    """Select explicit/registered maps or spawned maps, with their local CASC product."""
    with closing(sqlite3.connect(f"file:{world_db}?mode=ro", uri=True)) as db:
        rows = dict(db.execute("SELECT ID, WdtFileDataID FROM map"))
        selected = (
            sorted(set(map_ids))
            if map_ids
            else sorted(rows)
            if registered
            else [
                row[0]
                for row in db.execute(
                    "SELECT DISTINCT map FROM content_creature ORDER BY map"
                )
            ]
        )
        has_sources = db.execute(
            "SELECT 1 FROM sqlite_master WHERE name = 'db2_row_source'"
        ).fetchone()
        builds = (
            dict(
                db.execute(
                    "SELECT row_id, build FROM db2_row_source WHERE table_name = 'map'"
                )
            )
            if has_sources
            else {}
        )
    result = []
    for map_id in selected:
        if map_ids and map_id not in rows:
            raise ValueError(f"map {map_id} is not registered")
        build = builds.get(map_id)
        if build is not None and build not in BUILD_PRODUCTS:
            raise ValueError(f"map {map_id}: no local CASC product for build {build}")
        result.append(
            (map_id, rows.get(map_id), BUILD_PRODUCTS[build] if build else "wow")
        )
    return result


def add_map_selection(parser):
    selection = parser.add_mutually_exclusive_group()
    selection.add_argument(
        "--map",
        type=int,
        action="append",
        dest="map_ids",
        help="registered map ID; repeat for multiple maps, even without spawns",
    )
    selection.add_argument(
        "--registered", action="store_true", help="all registered maps"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True, help="game-engine/data")
    parser.add_argument(
        "--missing", action="store_true", help="print `map id<TAB>path` of the uncached files instead"
    )
    add_map_selection(parser)
    args = parser.parse_args()
    files = GroundFiles(args.cache)
    maps = selected_map_sources(args.world_db, args.map_ids, args.registered)
    for map_id, wdt, _product in maps:
        files.start_map(map_id)
        if wdt:
            files.add_map(wdt)
        else:
            files.missing.append((map_id, NO_WDT))
    size = sum((args.cache / relative).stat().st_size for relative in files.present)
    if args.missing:
        print("\n".join(f"{map_id}\t{path}" for map_id, path in files.missing))
    else:
        print("\n".join(files.present))
    missing_wdts = sum(1 for _, path in files.missing if path.endswith(".wdt") or path == NO_WDT)
    print(
        f"{len(maps)} selected maps: {len(files.present)} cached files, {size / 1e6:.1f} MB; "
        f"not cached: {missing_wdts} WDTs, {len(files.missing) - missing_wdts} ADT/WMO files",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
