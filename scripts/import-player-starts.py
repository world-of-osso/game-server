#!/usr/bin/env python3
"""Import authentic TDB1210 race/class starts into an owned scratch database.

python3 scripts/import-player-starts.py --db data/world.db.new --copy-from data/world.db

Only tdb_playercreateinfo changes. A destination named world.db (also through a
symlink) is refused. --copy-from snapshots an existing database read-only; an
existing scratch destination is backed up to <destination>.player-starts.bak.
The caller owns publishing the scratch database. NPE positions are retained,
but the server currently uses the normal start, not the Exile's Reach intro.
"""

import argparse
import sqlite3
from contextlib import closing
from pathlib import Path

from mysql_dump import dump_rows

DEFAULT_DUMP = Path(__file__).resolve().parents[1] / "data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql"
COLUMNS = (
    "race", "class", "map", "position_x", "position_y", "position_z", "orientation",
    "npe_map", "npe_position_x", "npe_position_y", "npe_position_z", "npe_orientation",
    "npe_transport_guid", "intro_movie_id", "intro_scene_id", "npe_intro_scene_id",
)
DDL = """CREATE TABLE tdb_playercreateinfo (
    race INTEGER NOT NULL, class INTEGER NOT NULL, map INTEGER NOT NULL,
    position_x REAL NOT NULL, position_y REAL NOT NULL, position_z REAL NOT NULL,
    orientation REAL NOT NULL, npe_map INTEGER, npe_position_x REAL,
    npe_position_y REAL, npe_position_z REAL, npe_orientation REAL,
    npe_transport_guid INTEGER, intro_movie_id INTEGER, intro_scene_id INTEGER,
    npe_intro_scene_id INTEGER, PRIMARY KEY (race, class)
)"""


def validate_destination(path):
    if path.name == "world.db" or path.resolve().name == "world.db":
        raise ValueError("refusing live world.db; import into an owned scratch copy")


def read_starts(dump):
    rows = []
    # The complete world dump is large; read only the named table's INSERTs.
    with dump.open(encoding="utf-8") as source:
        for line in source:
            if not line.startswith("INSERT INTO `playercreateinfo` "):
                continue
            for columns, row in dump_rows(line, "playercreateinfo", dump):
                if len(row) != len(COLUMNS) or (columns is not None and tuple(columns) != COLUMNS):
                    raise ValueError(f"{dump}: unexpected playercreateinfo columns")
                rows.append(row)
    if not rows:
        raise ValueError(f"{dump}: no playercreateinfo rows")
    return rows


def import_starts(conn, dump):
    rows = read_starts(dump)
    # A savepoint works within a caller's transaction and restores the old table
    # on a failed load, including duplicate race/class rows.
    conn.execute("SAVEPOINT player_starts")
    try:
        conn.execute("DROP TABLE IF EXISTS tdb_playercreateinfo")
        conn.execute(DDL)
        conn.executemany("INSERT INTO tdb_playercreateinfo VALUES (" + ",".join("?" for _ in COLUMNS) + ")", rows)
        conn.execute("RELEASE player_starts")
    except BaseException:
        conn.execute("ROLLBACK TO player_starts")
        conn.execute("RELEASE player_starts")
        raise
    return len(rows)


def snapshot_database(source, destination):
    if destination.exists():
        raise ValueError(f"snapshot destination already exists: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    with closing(sqlite3.connect(f"{source.resolve().as_uri()}?mode=ro", uri=True)) as original:
        with closing(sqlite3.connect(destination)) as backup:
            original.backup(backup)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--dump", type=Path, default=DEFAULT_DUMP)
    parser.add_argument("--copy-from", type=Path)
    args = parser.parse_args()
    validate_destination(args.db)
    if args.copy_from:
        snapshot_database(args.copy_from, args.db)
    else:
        snapshot_database(args.db, args.db.with_name(args.db.name + ".player-starts.bak"))
    with closing(sqlite3.connect(f"{args.db.resolve().as_uri()}?mode=rw", uri=True)) as conn:
        count = import_starts(conn, args.dump)
    print(f"tdb_playercreateinfo: {count} source rows imported into {args.db}")


if __name__ == "__main__":
    main()
