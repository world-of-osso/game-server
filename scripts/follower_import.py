"""Prepare authentic Retail follower reference SQL; never writes to world.db.

Input: local CASC LFGDungeons (12.1.0.69933, FDID1361033), TDB1210 world dump.
The lead applies the generated SQL. This prepares reference data only: no NPC
role or combat spell assignment is inferred from a name or creature unit_class.
"""

import argparse
import hashlib
import json
import sqlite3
from contextlib import closing
from pathlib import Path

from db2_casc import decode, read_header
from mysql_dump import dump_rows

# WoWDBDefs LFGDungeons.dbd layout for Retail12.1.0.69933.
LAYOUT = 0x34B02DE8
# The pinned Difficulty.csv names205 "Follower" (five players).
FOLLOWER_DIFFICULTY = 205
# Row index includes non-inline ID. Pallet values require declared-width truncation.
FIELDS = (
    ("id", 0, 32),
    ("name", 1, None),
    ("map_id", 10, 16),
    ("difficulty_id", 11, 16),
    ("tanks", 19, 8),
    ("healers", 20, 8),
    ("damage", 21, 8),
    ("min_tanks", 22, 8),
    ("min_healers", 23, 8),
    ("min_damage", 24, 8),
    ("max_premade_tanks", 25, 8),
    ("max_premade_healers", 26, 8),
    ("max_premade_damage", 27, 8),
    ("max_premade_group_size", 31, 8),
    ("content_tuning_id", 32, 32),
    ("flags_0", 33, 32),
    ("flags_1", 34, 32),
    ("required_player_condition_id", 15, 32),
)


def read_follower_dungeons(path):
    data = path.read_bytes()
    header = read_header(data)
    if header["layout_hash"] != LAYOUT:
        raise ValueError(
            f"LFGDungeons layout {header['layout_hash']:08X}, expected {LAYOUT:08X}"
        )
    _, decoded = decode(data, {1, 2})
    result = []
    for row in decoded.values():
        if len(row) != 35:
            raise ValueError(f"LFGDungeons row has {len(row)} fields, expected35")
        values = tuple(
            row[index] if width is None else row[index] & ((1 << width) - 1)
            for _, index, width in FIELDS
        )
        if values[3] == FOLLOWER_DIFFICULTY:
            result.append(values)
    if not result:
        raise ValueError("LFGDungeons contains no follower difficulty entries")
    return sorted(result)


def read_template_spells(path):
    result = []
    with path.open(encoding="utf-8") as source:
        for line in source:
            if not line.startswith("INSERT INTO `creature_template_spell`"):
                continue
            for _, row in dump_rows(line, "creature_template_spell", str(path)):
                if len(row) != 4 or any(not isinstance(value, int) for value in row):
                    raise ValueError(f"invalid creature_template_spell row: {row!r}")
                result.append(row)
    if not result:
        raise ValueError("TDB dump contains no creature_template_spell rows")
    return sorted(result)


def prepare_import(lfg_db2, world_sql, output):
    dungeons = read_follower_dungeons(lfg_db2)
    spells = read_template_spells(world_sql)
    columns = ",".join(
        f"{name} {'TEXT' if width is None else 'INTEGER'} NOT NULL"
        for name, _, width in FIELDS
    )
    with closing(sqlite3.connect(":memory:")) as staging:
        staging.execute(
            f"CREATE TABLE follower_lfg_dungeons ({columns},PRIMARY KEY(id))"
        )
        staging.executemany(
            f"INSERT INTO follower_lfg_dungeons VALUES ({','.join('?' for _ in FIELDS)})",
            dungeons,
        )
        staging.execute(
            "CREATE TABLE tdb_creature_template_spell "
            '("CreatureID" INTEGER NOT NULL,"Index" INTEGER NOT NULL,'
            '"Spell" INTEGER NOT NULL,"VerifiedBuild" INTEGER NOT NULL,'
            'PRIMARY KEY("CreatureID","Index"))'
        )
        staging.executemany(
            "INSERT INTO tdb_creature_template_spell VALUES (?,?,?,?)", spells
        )
        statements = list(staging.iterdump())
    # Replace only these reference tables, in the same transaction as the rows.
    statements[1:1] = [
        "DROP TABLE IF EXISTS follower_lfg_dungeons;",
        "DROP TABLE IF EXISTS tdb_creature_template_spell;",
    ]
    output.write_text("\n".join(statements) + "\n", encoding="utf-8")
    return {
        "lfg_db2_sha256": hashlib.sha256(lfg_db2.read_bytes()).hexdigest(),
        "lfg_layout": f"{LAYOUT:08X}",
        "dungeons": len(dungeons),
        "template_spells": len(spells),
    }


def audit_world(world_db, lfg_db2, world_sql):
    dungeons = read_follower_dungeons(lfg_db2)
    spells = read_template_spells(world_sql)
    spell_counts = {}
    for creature_id, _, spell_id, _ in spells:
        if spell_id:
            spell_counts[creature_id] = spell_counts.get(creature_id, 0) + 1
    with closing(
        sqlite3.connect(world_db.resolve().as_uri() + "?mode=ro", uri=True)
    ) as world:
        escorts = [
            {
                "entry": entry,
                "name": name,
                "ai_name": ai,
                "script_name": script,
                "authored_spell_count": spell_counts.get(entry, 0),
                "smart_script_count": world.execute(
                    "SELECT COUNT(*) FROM tdb_smart_scripts WHERE entryorguid=?",
                    (entry,),
                ).fetchone()[0],
            }
            for entry, name, ai, script in world.execute(
                "SELECT entry,name,AIName,ScriptName FROM tdb_creature_template "
                "WHERE subname='Expedition Escort' ORDER BY entry"
            )
        ]
        catalog = []
        for values in dungeons:
            row = dict(zip((field[0] for field in FIELDS), values))
            row["spawn_counts"] = {
                table: world.execute(
                    f"SELECT COUNT(*) FROM {table} WHERE map=?", (row["map_id"],)
                ).fetchone()[0]
                for table in ("creature", "tdb_creature", "content_creature")
            }
            catalog.append(row)
    return {"dungeons": catalog, "escort_candidates": escorts}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lfg-db2", required=True, type=Path)
    parser.add_argument("--world-sql", required=True, type=Path)
    parser.add_argument(
        "--output", required=True, type=Path, help="SQL for the lead to apply"
    )
    parser.add_argument(
        "--world-db", type=Path, help="Optional read-only spawn/kit audit"
    )
    parser.add_argument("--report", type=Path, help="JSON receipt/audit destination")
    args = parser.parse_args()
    inputs = [args.lfg_db2, args.world_sql]
    if args.world_db:
        inputs.append(args.world_db)
    destinations = [args.output] + ([args.report] if args.report else [])
    resolved = [path.resolve() for path in inputs + destinations]
    if len(set(resolved)) != len(resolved):
        parser.error("input/output paths must be distinct; world.db is read-only")
    report = {}
    if args.world_db:
        report["audit"] = audit_world(args.world_db, args.lfg_db2, args.world_sql)
    report["import"] = prepare_import(args.lfg_db2, args.world_sql, args.output)
    if args.report:
        args.report.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report["import"], sort_keys=True))


if __name__ == "__main__":
    main()
