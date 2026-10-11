#!/usr/bin/env python3
"""Enable authored TDB SmartAI on matching content templates in a private copy.

Only AIName/ScriptName change. Legacy content stats, spawns and quest relations
remain intact. TrinityCore a352b1fa selects SmartAI by AIName; a legacy
ScriptName must not override that selection. Source-9 lists alone aren't an AI.
"""
import argparse
import sqlite3
from pathlib import Path

CANDIDATES = """
    SELECT c.entry FROM content_creature_template c
    JOIN tdb_creature_template t USING(entry)
    WHERE t.AIName='SmartAI'
      AND EXISTS (SELECT 1 FROM tdb_smart_scripts s WHERE s.entryorguid=c.entry AND s.source_type=0)
      AND (COALESCE(c.AIName,'')<>'SmartAI' OR COALESCE(c.ScriptName,'')<>'')
    ORDER BY c.entry
"""


def import_overlay(path):
    path = Path(path).resolve(strict=True)
    canonical = Path(__file__).resolve().parents[1] / "data/world.db"
    if path == canonical.resolve():
        raise ValueError("Refusing canonical world.db; pass a private SQLite copy")
    with sqlite3.connect(path) as conn:
        entries = [row[0] for row in conn.execute(CANDIDATES)]
        conn.executemany("UPDATE content_creature_template SET AIName='SmartAI', ScriptName='' WHERE entry=?",
                         ((entry,) for entry in entries))
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True, help="existing private world.db copy")
    args = parser.parse_args()
    entries = import_overlay(args.db)
    print(f"Enabled TDB SmartAI for {len(entries)} templates")
    print("Entries: " + ",".join(map(str, entries)))


if __name__ == "__main__":
    main()
