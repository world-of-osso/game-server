#!/usr/bin/env python3
"""Prepare source-bound Darkglare and Wild Imp templates/models on a database copy.

python3 scripts/add_guardian_templates.py --db data/world.db.new
No synthetic creature, spell list, stat inheritance or model substitution.
"""
import argparse
import sqlite3
from pathlib import Path

import retail_content

DARKGLARE = 103673
# SimC76f8118 sc_warlock_pets.cpp721/867 binds Wild Imp's Fel Firebolt104318;
# native104317 effect28 supplies creature55659. Other demons remain unbound.
SOURCE_BOUND_GUARDIANS = (DARKGLARE, 55659)


def add_guardian_templates(conn):
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function("health_expansion", 1, retail_content.health_expansion, deterministic=True)
    return [entry for entry in SOURCE_BOUND_GUARDIANS if prepare_template(conn, entry)]


def prepare_template(conn, entry):
    present = conn.execute("SELECT 1 FROM content_creature_template WHERE entry=?", (entry,)).fetchone()
    retail_content.insert_tdb_templates(conn, str(entry))
    if not present:
        conn.execute("""INSERT INTO content_creature_template_model
            SELECT CreatureID, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild
            FROM tdb_creature_template_model WHERE CreatureID=?""", (entry,))
    found = conn.execute("""SELECT 1 FROM content_creature_template t
        JOIN content_creature_template_model m ON m.CreatureID=t.entry
        WHERE t.entry=?""", (entry,)).fetchone()
    if not found:
        raise ValueError(f"missing authentic guardian template/model: {entry}")
    return not present


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path)
    args = parser.parse_args()
    live = Path(__file__).resolve().parents[1] / "data/world.db"
    if args.db.name == "world.db" or (args.db.exists() and args.db.samefile(live)):
        parser.error("prepare a database copy, not the live world.db")
    if not args.db.is_file():
        parser.error("database copy does not exist")
    with sqlite3.connect(args.db) as conn:
        added = add_guardian_templates(conn)
    print(f"added guardian templates: {added}")


if __name__ == "__main__":
    main()
