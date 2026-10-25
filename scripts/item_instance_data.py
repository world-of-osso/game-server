#!/usr/bin/env python3
"""Import item-instance DB2 tables; canonical publication belongs to the lead.

Full replacement import: python3 scripts/item_instance_data.py --db2-dir PATH/12.1.5.70448 --db COPY.db
Additive canonical publication: add --publish-producer-catalogs. Only four absent
producer catalogs/source stamps are created; identical repeats write nothing,
and existing table/provenance drift is refused. Existing eight catalogs are untouched.
ItemSparse/RandPropPoints/CurvePoint remain owned by the ordinary DB2 importer.
ScalingStatDistribution was removed from this build; ItemScalingConfig and
ItemOffsetCurve supply Midnight's native scaling instead.
"""
import argparse
from contextlib import closing
import csv
import hashlib
from pathlib import Path
import re
import sqlite3

PRODUCER_TABLES = {
    "ItemSet": "item_set",
    "ItemSetSpell": "item_set_spell",
    "SpellDescriptionVariables": "spell_description_variables",
    "SpellXDescriptionVariables": "spell_x_description_variables",
}
SOURCE_SCHEMA = ("CREATE TABLE IF NOT EXISTS item_instance_source "
                 "(table_name TEXT PRIMARY KEY, build TEXT NOT NULL, sha256 TEXT NOT NULL)")

TABLES = {
    "ItemBonus": "item_bonus",
    "ItemBonusListLevelDelta": "item_bonus_list_level_delta",
    "ItemScalingConfig": "item_scaling_config",
    "ItemOffsetCurve": "item_offset_curve",
    "SpellItemEnchantment": "spell_item_enchantment",
    "GemProperties": "gem_properties",
    "ItemEffect": "item_effect",
    "ItemXItemEffect": "item_x_item_effect",
    **PRODUCER_TABLES,
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
        conn.execute(SOURCE_SCHEMA)
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


def table_exists(conn, table):
    return conn.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?",
                        (table,)).fetchone() is not None


def validate_existing_catalog(conn, native, table):
    if not table_exists(conn, table):
        return False
    name = quote_identifier(table)
    columns = [row[1] for row in native.execute(f"PRAGMA table_info({name})")]
    existing = {row[1] for row in conn.execute(f"PRAGMA table_info({name})")}
    if not set(columns).issubset(existing):
        raise ValueError(f"Existing {table} lacks native columns; additive publication refuses")
    fields = ",".join(quote_identifier(column) for column in columns)
    query = f"SELECT {fields} FROM {name} ORDER BY ID"
    if conn.execute(query).fetchall() != native.execute(query).fetchall():
        raise ValueError(f"Existing {table} differs from native source; additive publication refuses")
    return True


def read_publication_plan(conn, native, source):
    stamps = {}
    if table_exists(conn, "item_instance_source"):
        stamps = {table: (build, digest) for table, build, digest in conn.execute(
            "SELECT table_name,build,sha256 FROM item_instance_source")}
    plan = []
    for csv_name, table in PRODUCER_TABLES.items():
        path = source / f"{csv_name}.csv"
        stamp = (source.name, hashlib.sha256(path.read_bytes()).hexdigest())
        existing = validate_existing_catalog(conn, native, table)
        recorded = stamps.get(table)
        if recorded is not None and (recorded != stamp or not existing):
            raise ValueError(f"Existing {table} source stamp conflicts; additive publication refuses")
        plan.append((path, table, existing, recorded is None, stamp))
    return plan


def apply_publication_plan(conn, plan):
    conn.execute(SOURCE_SCHEMA)
    for path, table, existing, add_stamp, stamp in plan:
        if not existing:
            import_table(conn, path, table)
        if add_stamp:
            conn.execute("INSERT INTO item_instance_source VALUES (?,?,?)", (table, *stamp))


def publish_producer_catalogs(conn, source):
    # Read/validate all inputs without touching the destination database.
    with closing(sqlite3.connect(":memory:")) as native:
        counts = {table: import_table(native, source / f"{csv_name}.csv", table)
                  for csv_name, table in PRODUCER_TABLES.items()}
        conn.execute("SAVEPOINT producer_catalog_publication")
        try:
            plan = read_publication_plan(conn, native, source)
            apply_publication_plan(conn, plan)
            conn.execute("RELEASE producer_catalog_publication")
        except BaseException:
            conn.execute("ROLLBACK TO producer_catalog_publication")
            conn.execute("RELEASE producer_catalog_publication")
            raise
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db2-dir", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--publish-producer-catalogs", action="store_true",
                        help="Add only four absent producer catalogs; accept identical existing "
                             "rows without writes and refuse drift")
    args = parser.parse_args()
    if not args.db.is_file():
        parser.error(f"Database does not exist: {args.db}")
    with closing(sqlite3.connect(args.db)) as conn, conn:
        operation = publish_producer_catalogs if args.publish_producer_catalogs else import_catalog
        counts = operation(conn, args.db2_dir)
    for table, count in counts.items():
        print(f"{table}: {count}")


if __name__ == "__main__":
    main()
