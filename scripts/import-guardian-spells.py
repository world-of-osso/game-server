#!/usr/bin/env python3
"""Import authentic TDB1210 creature_template_spell rows into a prepared DB copy.

python3 scripts/import-guardian-spells.py --archive data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql --db data/world.db.new
Native creature lists win. Two approved missing-row policies supply Dreadstalker
and Antoran Tyrant attacks, not summon dispatch or repeat timers. Policy rows use
VerifiedBuild=0; they are not claimed as verified TDB data.
"""
import argparse
from contextlib import closing
import sqlite3
from pathlib import Path

from mysql_dump import dump_rows

# Lead-approved SimC76f8118 policies; real archive lists replace these wholesale.
PROJECT_BINDINGS = (
    (98035, 271971, "SIMC_DREADSTALKER_ATTACK_BINDING"),
    (250289, 1264093, "SIMC_ANTORAN_TYRANT_BURNING_CLEAVE_BINDING"),
)


def append_missing_project_bindings(conn):
    applied = []
    for creature, spell, policy in PROJECT_BINDINGS:
        if conn.execute(
            "SELECT 1 FROM guardian_spells WHERE CreatureID=? LIMIT 1", (creature,)
        ).fetchone():
            continue
        conn.execute(
            "INSERT INTO guardian_spells VALUES (?,0,?,0)", (creature, spell)
        )
        applied.append(policy)
    return applied


def import_guardian_spells(conn, archive):
    rows = []
    with Path(archive).open(encoding="utf-8") as source:
        for line in source:
            if not line.startswith("INSERT INTO `creature_template_spell`"):
                continue
            for columns, row in dump_rows(line, "creature_template_spell", archive):
                if columns is not None or len(row) != 4 or any(not isinstance(value, int) for value in row):
                    raise ValueError("unexpected creature_template_spell layout")
                creature, index, spell, build = row
                if creature <= 0 or index < 0 or spell <= 0:
                    raise ValueError(f"invalid creature spell row: {row}")
                rows.append((creature, index, spell, build))
    if not rows:
        raise ValueError("archive contains no creature_template_spell rows")
    conn.execute("SAVEPOINT guardian_import")
    try:
        conn.execute("CREATE TABLE IF NOT EXISTS guardian_spells (CreatureID INTEGER NOT NULL, SpellIndex INTEGER NOT NULL, Spell INTEGER NOT NULL, VerifiedBuild INTEGER NOT NULL, PRIMARY KEY (CreatureID, SpellIndex))")
        conn.execute("DELETE FROM guardian_spells")
        conn.executemany("INSERT INTO guardian_spells VALUES (?, ?, ?, ?)", rows)
        applied = append_missing_project_bindings(conn)
        conn.execute("RELEASE guardian_import")
    except BaseException:
        conn.execute("ROLLBACK TO guardian_import")
        conn.execute("RELEASE guardian_import")
        raise
    return len(rows), applied


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    if args.db.name == "world.db":
        parser.error("prepare a database copy, not the live world.db")
    if not args.db.is_file():
        parser.error("database copy does not exist")
    with closing(sqlite3.connect(args.db)) as conn:
        native_count, policies = import_guardian_spells(conn, args.archive)
    print(
        f"guardian_spells: {native_count} authentic archive rows + "
        f"{len(policies)} explicit project policies"
    )
    for policy in policies:
        print(f"guardian_policy: {policy}")


if __name__ == "__main__":
    main()
