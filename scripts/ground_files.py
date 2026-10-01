#!/usr/bin/env python3
"""List the client files the server's ground loads for the maps creatures spawn on.

crates/server/src/ground.rs reads, from the client asset cache: each map's WDT
(`terrain/<Map.WdtFileDataID>.wdt`); for a global-WMO map (MPHD flag 0x1) the WDT MODF
WMO, else every MAID tile's root and `_obj0` ADT (`terrain/<fdid>.adt`); each `_obj0`
MODF WMO placed by FileDataID (flag 0x8) and its GFID group files (`models/<fdid>.wmo`).

Prints the cached files' paths relative to the cache, one per line (rsync
--files-from). The summary on stderr counts the files the cache does not hold: those
tiles and WMOs have no ground on the server until they are extracted.
"""

import argparse
import sqlite3
import struct
import sys
from pathlib import Path

MPHD_FLAG_GLOBAL_WMO = 0x1
MAID_ENTRY_SIZE = 32
MODF_ENTRY_SIZE = 64
MODF_FLAG_FILEDATAID = 0x8


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
        self.present = []
        self.missing = []
        self.seen = set()

    def read(self, relative):
        """The file's bytes once (None if already listed or not cached)."""
        if relative in self.seen:
            return None
        self.seen.add(relative)
        path = self.cache / relative
        if not path.is_file():
            self.missing.append(relative)
            return None
        self.present.append(relative)
        return path.read_bytes()

    def add_wmo(self, fdid):
        root = self.read(f"models/{fdid}.wmo")
        if root is None:
            return
        gfid = chunks(root).get(b"GFID", b"")
        for offset in range(0, len(gfid) - 3, 4):
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


def spawned_map_wdts(world_db):
    """(map id, WdtFileDataID) of every map with a `content_creature` spawn."""
    connection = sqlite3.connect(f"file:{world_db}?mode=ro", uri=True)
    return connection.execute(
        """SELECT DISTINCT map.ID, map.WdtFileDataID FROM content_creature creature
           JOIN map ON map.ID = creature.map ORDER BY map.ID"""
    ).fetchall()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True, help="game-engine/data")
    args = parser.parse_args()
    files = GroundFiles(args.cache)
    maps = spawned_map_wdts(args.world_db)
    for _, wdt in maps:
        if wdt:
            files.add_map(wdt)
    size = sum((args.cache / relative).stat().st_size for relative in files.present)
    print("\n".join(files.present))
    missing_wdts = sum(1 for path in files.missing if path.endswith(".wdt"))
    print(
        f"{len(maps)} spawned maps: {len(files.present)} cached files, {size / 1e6:.1f} MB; "
        f"not cached: {missing_wdts} WDTs, {len(files.missing) - missing_wdts} ADT/WMO files",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
