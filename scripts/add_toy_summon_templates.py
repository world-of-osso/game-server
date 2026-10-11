#!/usr/bin/env python3
"""Add authentic TDB templates/models/addons used by linked/scenery summon toys.

Run on a copy: python3 scripts/add_toy_summon_templates.py --db data/world.db.new
Uses the same Retail conversion as content_policy.py; never synthesizes rows.
"""
import argparse
import sqlite3
from pathlib import Path

import retail_content


def add_toy_summon_templates(conn):
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function("health_expansion", 1, retail_content.health_expansion, deterministic=True)
    before = {entry for (entry,) in conn.execute("SELECT entry FROM content_creature_template")}
    retail_content.insert_tdb_templates(conn, retail_content.TOY_SUMMON_ENTRIES)
    added = sorted({entry for (entry,) in conn.execute("SELECT entry FROM content_creature_template")} - before)
    conn.execute("CREATE TEMP TABLE added_toy_entry (entry INTEGER PRIMARY KEY)")
    conn.executemany("INSERT INTO temp.added_toy_entry VALUES (?)", ((entry,) for entry in added))
    conn.execute("""INSERT INTO content_creature_template_model
        SELECT CreatureID, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild
        FROM tdb_creature_template_model WHERE CreatureID IN (SELECT entry FROM temp.added_toy_entry)""")
    conn.execute(f"""INSERT INTO content_creature_template_addon SELECT a.entry, {retail_content.ADDON_COLUMNS}
        FROM tdb_creature_template_addon a WHERE a.entry IN (SELECT entry FROM temp.added_toy_entry)""")
    missing = [entry for entry in retail_content.TOY_SCENERY_SUMMON_ENTRIES
               if not conn.execute("""SELECT 1 FROM content_creature_template t
                   JOIN content_creature_template_model m ON m.CreatureID = t.entry
                   WHERE t.entry = ?""", (entry,)).fetchone()]
    if missing:
        raise ValueError(f"scenery toys missing authentic TDB templates/models: {missing}")
    return added


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path)
    args = parser.parse_args()
    if not args.db.is_file():
        parser.error(f"database not found: {args.db}")
    with sqlite3.connect(args.db) as conn:
        added = add_toy_summon_templates(conn)
    print(f"added {len(added)} toy summon templates: {added}")


if __name__ == "__main__":
    main()
