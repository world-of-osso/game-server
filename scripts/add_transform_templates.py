#!/usr/bin/env python3
"""Add the creature templates spells transform into to an imported world.db, additively.

  add_transform_templates.py --db data/world.db.new

content_policy.py now keeps TrinityCore's template of every entry a TRANSFORM aura names
(retail_content.TRANSFORM_ENTRIES), not only spawned ones. This applies that rule to an
already-imported database without re-running the import: it inserts the missing
content_creature_template rows, their content_creature_template_model rows and their
content_creature_template_addon rows, exactly as the import would, and changes no other row.
Run it on a copy; the caller swaps the copy in.
"""

import argparse
import sqlite3
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import retail_content  # noqa: E402


def add_transform_templates(conn):
    """Insert the missing transform templates with their models and addons; return their entries."""
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function("health_expansion", 1, retail_content.health_expansion, deterministic=True)
    before = {entry for (entry,) in conn.execute("SELECT entry FROM content_creature_template")}
    retail_content.insert_tdb_templates(conn, retail_content.TRANSFORM_ENTRIES)
    added = sorted({entry for (entry,) in conn.execute("SELECT entry FROM content_creature_template")} - before)
    conn.execute("CREATE TEMP TABLE added_entry (entry INTEGER PRIMARY KEY)")
    conn.executemany("INSERT INTO temp.added_entry VALUES (?)", ((entry,) for entry in added))
    conn.execute("""INSERT INTO content_creature_template_model
        SELECT CreatureID, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild
        FROM tdb_creature_template_model WHERE CreatureID IN (SELECT entry FROM temp.added_entry)""")
    conn.execute(f"""INSERT INTO content_creature_template_addon SELECT a.entry, {retail_content.ADDON_COLUMNS}
        FROM tdb_creature_template_addon a WHERE a.entry IN (SELECT entry FROM temp.added_entry)""")
    return added


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", required=True, type=Path)
    args = parser.parse_args()
    with sqlite3.connect(args.db) as conn:
        added = add_transform_templates(conn)
    print(f"added {len(added)} transform templates")


if __name__ == "__main__":
    main()
