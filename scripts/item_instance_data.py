#!/usr/bin/env python3
"""Import item-instance DB2 tables; canonical publication belongs to the lead.

Usage: python3 scripts/item_instance_data.py --db2-dir PATH/12.1.5.70448 --db COPY.db
ItemSparse/RandPropPoints/CurvePoint remain owned by the ordinary DB2 importer.
ScalingStatDistribution was removed from this build; ItemScalingConfig and
ItemOffsetCurve supply Midnight's native scaling instead.
"""
import argparse
import csv
import hashlib
from pathlib import Path
import re
import sqlite3

TABLES = {
    "ItemBonus": "item_bonus",
    "ItemBonusListLevelDelta": "item_bonus_list_level_delta",
    "ItemScalingConfig": "item_scaling_config",
    "ItemOffsetCurve": "item_offset_curve",
    "SpellItemEnchantment": "spell_item_enchantment",
    "GemProperties": "gem_properties",
    "ItemEffect": "item_effect",
    "ItemXItemEffect": "item_x_item_effect",
}


def quote_identifier(name):
    if not re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name):
        raise ValueError(f"Invalid SQL identifier: {name!r}")
    return f'"{name}"'


def import_table(conn, path, table):
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle)
        header = next(reader)
        rows = list(reader)
    columns = [f'{quote_identifier(column)} {"TEXT" if column.endswith("_lang") else "NUMERIC"}'
               + (" PRIMARY KEY" if column == "ID" else "") for column in header]
    name = quote_identifier(table)
    placeholders = ",".join("?" for _ in header)
    conn.execute("SAVEPOINT item_instance_import")
    try:
        conn.execute(f"DROP TABLE IF EXISTS {name}")
        conn.execute(f'CREATE TABLE {name} ({",".join(columns)})')
        conn.executemany(f"INSERT INTO {name} VALUES ({placeholders})", rows)
    except BaseException:
        conn.execute("ROLLBACK TO item_instance_import")
        conn.execute("RELEASE item_instance_import")
        raise
    conn.execute("RELEASE item_instance_import")
    return len(rows)


def import_catalog(conn, source):
    paths = [(source / f"{name}.csv", table) for name, table in TABLES.items()]
    for path, _ in paths:
        if not path.is_file():
            raise FileNotFoundError(path)
    conn.execute("SAVEPOINT item_instance_catalog")
    try:
        conn.execute("CREATE TABLE IF NOT EXISTS item_instance_source "
                     "(table_name TEXT PRIMARY KEY, build TEXT NOT NULL, sha256 TEXT NOT NULL)")
        counts = {}
        for path, table in paths:
            counts[table] = import_table(conn, path, table)
            conn.execute("INSERT OR REPLACE INTO item_instance_source VALUES (?,?,?)",
                         (table, source.name, hashlib.sha256(path.read_bytes()).hexdigest()))
        conn.execute("CREATE INDEX item_bonus_parent ON item_bonus(ParentItemBonusListID,OrderIndex,ID)")
        conn.execute("RELEASE item_instance_catalog")
    except BaseException:
        conn.execute("ROLLBACK TO item_instance_catalog")
        conn.execute("RELEASE item_instance_catalog")
        raise
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db2-dir", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    if not args.db.is_file():
        parser.error(f"Database does not exist: {args.db}")
    with sqlite3.connect(args.db) as conn:
        counts = import_catalog(conn, args.db2_dir)
    for table, count in counts.items():
        print(f"{table}: {count}")


if __name__ == "__main__":
    main()
