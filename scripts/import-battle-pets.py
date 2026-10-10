#!/usr/bin/env python3
"""Import a LOCAL CASC export_pet_catalog.py JSON into the deployed world.db."""
import argparse
import json
from pathlib import Path
import sqlite3


def import_catalog(source: Path, database: Path) -> int:
    entries = json.loads(source.read_text())
    rows = [(entry["species_id"], entry["creature_id"], entry["name"],
             entry["display_id"], entry["summon_spell_id"]) for entry in entries]
    with sqlite3.connect(database) as conn:
        conn.execute("""CREATE TABLE IF NOT EXISTS battle_pet_species (
            species_id INTEGER PRIMARY KEY,
            creature_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            display_id INTEGER NOT NULL,
            summon_spell_id INTEGER NOT NULL
        )""")
        conn.execute("DELETE FROM battle_pet_species")
        conn.executemany("INSERT INTO battle_pet_species VALUES (?,?,?,?,?)", rows)
    return len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("catalog", type=Path)
    parser.add_argument("database", type=Path)
    args = parser.parse_args()
    count = import_catalog(args.catalog, args.database)
    print(json.dumps({"battle_pet_species": count}))


if __name__ == "__main__":
    main()
