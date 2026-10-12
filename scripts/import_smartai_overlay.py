#!/usr/bin/env python3
"""Enable authored TDB SmartAI on matching content templates in a private copy.

Only AIName/ScriptName change. Legacy content stats, spawns and quest relations
remain intact. TrinityCore a352b1fa selects SmartAI by AIName; a legacy
ScriptName must not override that selection. Source-9 lists alone aren't an AI.
"""
import argparse
import sqlite3
from pathlib import Path

from mysql_dump import dump_rows

GROUP_SCHEMA = """
    CREATE TABLE IF NOT EXISTS tdb_creature_summon_groups (
        summonerId INTEGER, summonerType INTEGER, groupId INTEGER, entry INTEGER,
        position_x REAL, position_y REAL, position_z REAL, orientation REAL,
        summonType INTEGER, summonTime INTEGER, Comment TEXT
    )
"""


def read_summon_groups(path):
    rows = []
    with Path(path).open() as source:
        for line in source:
            if line.startswith("INSERT INTO `creature_summon_groups` "):
                rows.extend(row for _, row in dump_rows(line, "creature_summon_groups", path))
    if not rows or any(len(row) != 11 for row in rows):
        raise ValueError(f"{path}: expected pinned 11-column creature_summon_groups rows")
    return rows

CANDIDATES = """
    SELECT c.entry FROM content_creature_template c
    JOIN tdb_creature_template t USING(entry)
    WHERE t.AIName='SmartAI'
      AND EXISTS (SELECT 1 FROM tdb_smart_scripts s WHERE s.entryorguid=c.entry AND s.source_type=0)
      AND (COALESCE(c.AIName,'')<>'SmartAI' OR COALESCE(c.ScriptName,'')<>'')
    ORDER BY c.entry
"""


def import_overlay(path, tdb_world_sql=None):
    path = Path(path).resolve(strict=True)
    canonical = Path(__file__).resolve().parents[1] / "data/world.db"
    if path == canonical.resolve():
        raise ValueError("Refusing canonical world.db; pass a private SQLite copy")
    groups = read_summon_groups(tdb_world_sql) if tdb_world_sql is not None else None
    with sqlite3.connect(path) as conn:
        entries = [row[0] for row in conn.execute(CANDIDATES)]
        conn.executemany("UPDATE content_creature_template SET AIName='SmartAI', ScriptName='' WHERE entry=?",
                         ((entry,) for entry in entries))
        if groups is not None:
            conn.execute(GROUP_SCHEMA)
            conn.execute("DELETE FROM tdb_creature_summon_groups")
            conn.executemany("INSERT INTO tdb_creature_summon_groups VALUES (?,?,?,?,?,?,?,?,?,?,?)", groups)
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True, help="existing private world.db copy")
    parser.add_argument("--tdb-world-sql", type=Path, help="local pinned TDB world dump; imports authored creature summon groups")
    args = parser.parse_args()
    entries = import_overlay(args.db, args.tdb_world_sql)
    print(f"Enabled TDB SmartAI for {len(entries)} templates")
    print("Entries: " + ",".join(map(str, entries)))


if __name__ == "__main__":
    main()
