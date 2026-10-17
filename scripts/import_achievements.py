#!/usr/bin/env python3
"""Import active installed achievement CSVs into world.db by named columns.

Validate the active build and export receipt before replacing four tables in one
transaction. Readable records follow the active exporter; encrypted gaps are not
filled from historical builds. Raw decoding below serves frozen historical tests.
"""
import argparse
import csv
import hashlib
import json
import sqlite3
from contextlib import closing
from pathlib import Path

import casc_locale
import db2_casc

ACTIVE_DB2 = Path(__file__).resolve().parents[1] / "data/db2/active"
BUILD_INFO = casc_locale.INSTALL / ".build.info"
TABLES = {
    "Achievement": (1260179, 0x6FC5281B, {0, 1, 2}, [
        ("ID", 3), ("Title_lang", 1), ("Description_lang", 0),
        ("Instance_ID", 4), ("Faction", 5), ("Points", 9),
        ("Flags", 10), ("CriteriaTree", 14), ("Category", 7), ("IconFileID", 12),
    ]),
    "Achievement_Category": (1324299, 0x67B2B4BD, {0}, [
        ("ID", 1), ("Name_lang", 0), ("Parent", 2), ("Ui_order", 3),
    ]),
    "Criteria": (1263817, 0x53D03513, set(), [
        ("ID", 0), ("Type", 1), ("Asset", 2), ("ModifierTreeId", 3),
        ("StartEvent", 4), ("StartAsset", 5), ("StartTimer", 6),
        ("FailEvent", 7), ("FailAsset", 8), ("Flags", 9),
        ("EligibilityWorldStateID", 10), ("EligibilityWorldStateValue", 11),
    ]),
    "CriteriaTree": (1263818, 0xFF97AE12, {1}, [
        ("ID", 0), ("Description_lang", 1), ("Parent", 2), ("Amount", 3),
        ("Operator", 4), ("CriteriaID", 5), ("Flags", 6), ("OrderIndex", 7),
    ]),
}
SQL_NAMES = {"Achievement": "achievement", "Achievement_Category": "achievement_category",
             "Criteria": "criteria", "CriteriaTree": "criteria_tree"}


def read_active_build(active_directory, build_info):
    with build_info.open(encoding="utf-8") as stream:
        columns = [column.split("!")[0] for column in next(stream).strip().split("|")]
        records = csv.DictReader(stream, fieldnames=columns, delimiter="|")
        builds = {row["Version"] for row in records
                  if row["Product"] == "wow" and row["Active"] == "1"}
    if len(builds) != 1:
        raise ValueError(f"{build_info}: expected one active wow build, got {sorted(builds)}")
    build = builds.pop()
    active = active_directory.resolve(strict=True)
    if active.name != build:
        raise ValueError(f"{active_directory}: build {active.name}, installed wow build {build}")
    return build


def export_tables(directory):
    tables = {}
    for name, (fdid, layout, text, columns) in TABLES.items():
        data = (directory / f"{fdid}.db2").read_bytes()
        header = db2_casc.read_header(data)
        if header["layout_hash"] != layout:
            raise ValueError(f"{name}: layout {header['layout_hash']:08X}, expected {layout:08X}")
        _, records = db2_casc.decode(data, text)
        rows = [[values[index] for _, index in columns] for _, values in sorted(records.items())]
        if name == "Achievement":
            for row in rows:
                # Points is an 8-bit DB2 value; pallet entries are stored as uint32.
                row[5] &= 0xFF
                row[4] = db2_casc.typed(row[4], "INTEGER")
        path = directory / f"{name}.csv"
        with path.open("w", newline="", encoding="utf-8") as output:
            writer = csv.writer(output)
            writer.writerow([column for column, _ in columns])
            writer.writerows(rows)
        tables[name] = (columns, rows)
        print(f"{name}: {len(rows)} rows, FDID {fdid}, layout {layout:08X}, {path}")
    return tables


def read_active_tables(directory, build):
    receipt = json.loads((directory / 'receipt.json').read_text())
    identity = receipt['identity']
    if identity['build'] != build or identity['product'] != 'wow':
        raise ValueError(f'{directory}: receipt identity does not match active wow {build}')
    tables = {}
    for name, (fdid, layout, _, columns) in TABLES.items():
        source = directory / f'{name}.csv'
        entry = receipt['tables'][name]
        if entry['fdid'] != fdid or entry['layout'] != f'{layout:08X}':
            raise ValueError(f'{name}: unsupported receipt FDID/layout')
        if hashlib.sha256(source.read_bytes()).hexdigest() != entry['csv_sha256']:
            raise ValueError(f'{name}: CSV SHA256 differs from active receipt')
        with source.open(newline='', encoding='utf-8') as stream:
            reader = csv.DictReader(stream)
            rows = [[row[column] if column.endswith('_lang') else int(row[column])
                     for column, _ in columns] for row in reader]
        if len(rows) != entry['rows']:
            raise ValueError(f'{name}: row count differs from active receipt')
        tables[name] = (columns, rows)
    print(json.dumps({'identity': identity, 'inputs': {
        name: receipt['tables'][name] for name in TABLES}}))
    return tables


def import_tables(database, tables, build):
    with closing(sqlite3.connect(database)) as db, db:
        db.execute("BEGIN IMMEDIATE")
        for name, (columns, rows) in tables.items():
            table = SQL_NAMES[name]
            db.execute(f"DROP TABLE IF EXISTS {table}")
            declarations = [f'"{column}" ' + ("TEXT" if column.endswith("_lang") else "INTEGER")
                            + (" PRIMARY KEY" if column == "ID" else " NOT NULL") for column, _ in columns]
            db.execute(f"CREATE TABLE {table} ({', '.join(declarations)})")
            db.executemany(f"INSERT INTO {table} VALUES ({','.join('?' for _ in columns)})", rows)
            db.execute("INSERT OR REPLACE INTO db2_build (table_name, build) VALUES (?, ?)", (table, build))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db2-dir", type=Path, required=True)
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    build = read_active_build(ACTIVE_DB2, BUILD_INFO)
    if args.db2_dir.resolve().name != build:
        parser.error(f"requires active installed build {build}, got {args.db2_dir.resolve().name}")
    import_tables(args.db, read_active_tables(args.db2_dir, build), build)


if __name__ == "__main__":
    main()
