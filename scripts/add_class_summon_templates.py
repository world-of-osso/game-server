#!/usr/bin/env python3
"""Apply native class/spec/talent summon-template closure to a prepared database copy.

Adds the exact missing TDB templates, models and addons selected by the normal
content importer. Never creates world spawns or fabricates creature statistics.
"""
import argparse
from contextlib import closing
import sqlite3
from pathlib import Path

import retail_content


def add_class_summon_templates(conn):
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function("health_expansion", 1, retail_content.health_expansion, deterministic=True)
    before = {row[0] for row in conn.execute("SELECT entry FROM content_creature_template")}
    retail_content.insert_tdb_templates(conn, retail_content.CLASS_SUMMON_ENTRIES)
    after = {row[0] for row in conn.execute("SELECT entry FROM content_creature_template")}
    added = sorted(after - before)
    conn.execute("CREATE TEMP TABLE added_class_summon(entry INTEGER PRIMARY KEY)")
    conn.executemany("INSERT INTO added_class_summon VALUES (?)", ((entry,) for entry in added))
    conn.execute("""INSERT INTO content_creature_template_model
        SELECT CreatureID,Idx,CreatureDisplayID,DisplayScale,Probability,VerifiedBuild
        FROM tdb_creature_template_model WHERE CreatureID IN (SELECT entry FROM added_class_summon)""")
    conn.execute(f"""INSERT INTO content_creature_template_addon
        SELECT a.entry,{retail_content.ADDON_COLUMNS} FROM tdb_creature_template_addon a
        WHERE a.entry IN (SELECT entry FROM added_class_summon)""")
    return added


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    if args.db.name == "world.db" or not args.db.is_file():
        parser.error("provide an existing prepared copy, not world.db")
    with closing(sqlite3.connect(args.db)) as conn, conn:
        added = add_class_summon_templates(conn)
    print(f"added {len(added)} authentic class summon templates; world spawns unchanged")


if __name__ == "__main__":
    main()
