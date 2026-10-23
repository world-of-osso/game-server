#!/usr/bin/env python3
"""Prepare source-bound guardian templates/models on a database copy.

python3 scripts/add_guardian_templates.py --db data/world.db.new
python3 scripts/add_guardian_templates.py --db data/world.db.new --spell 104317 --spell 193331
Explicit spell roots select native Effect28/56 creature IDs; no synthetic rows,
spell lists, stat inheritance or model substitution.
"""
import argparse
import sqlite3
from pathlib import Path

import retail_content

DARKGLARE = 103673
# SimC76f8118 sc_warlock_pets.cpp721/867 binds Wild Imp's Fel Firebolt104318;
# native104317 effect28 supplies creature55659. Other demons remain unbound.
SOURCE_BOUND_GUARDIANS = (DARKGLARE, 55659)


def summon_entries(conn, spell_roots):
    entries = set()
    for spell in spell_roots:
        rows = conn.execute("""SELECT DISTINCT EffectMiscValue_0 FROM spell_effect
            WHERE SpellID=? AND DifficultyID=0 AND Effect IN (28,56) AND EffectMiscValue_0>0""", (spell,)).fetchall()
        if not rows:
            raise ValueError(f"spell {spell} has no native creature summon effect")
        entries.update(row[0] for row in rows)
    return sorted(entries)


def add_guardian_templates(conn, spell_roots=None):
    entries = SOURCE_BOUND_GUARDIANS if spell_roots is None else summon_entries(conn, spell_roots)
    retail_content.create_retail_map_table(conn)
    retail_content.create_difficulty_table(conn)
    conn.create_function("health_expansion", 1, retail_content.health_expansion, deterministic=True)
    return [entry for entry in entries if prepare_template(conn, entry)]


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
    parser.add_argument("--spell", action="append", type=int, help="native summon spell root; repeatable, replaces default guardian selection")
    args = parser.parse_args()
    live = Path(__file__).resolve().parents[1] / "data/world.db"
    if args.db.name == "world.db" or (args.db.exists() and args.db.samefile(live)):
        parser.error("prepare a database copy, not the live world.db")
    if not args.db.is_file():
        parser.error("database copy does not exist")
    with sqlite3.connect(args.db) as conn:
        added = add_guardian_templates(conn, args.spell)
    print(f"added guardian templates: {added}")


if __name__ == "__main__":
    main()
