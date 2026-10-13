#!/usr/bin/env python3
"""Import pinned TDB1210 creature_immunities into an existing owned/canonical copy.

Lead owns canonical publication. Reads local TDB only; replaces this table alone.
The full source-row fingerprint prevents partial or different-build imports.
"""
import argparse
from contextlib import closing
import hashlib
import json
from pathlib import Path
import sqlite3

from mysql_dump import dump_rows

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SOURCE = ROOT / "data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql"
TABLE = "creature_immunities"
COLUMNS = ("ID", "SchoolMask", "DispelTypeMask", "MechanicsMask", "Effects", "Auras",
           "ImmuneAoE", "ImmuneChain", "Comment")
EXPECTED_ROWS = 235
EXPECTED_FINGERPRINT = "b3794d194a12d7f7b4122dccb156987d2579394b6d568f11de2e9fc474c71ecb"
DDL = """CREATE TABLE IF NOT EXISTS creature_immunities (
    ID INTEGER PRIMARY KEY,
    SchoolMask INTEGER NOT NULL,
    DispelTypeMask INTEGER NOT NULL,
    MechanicsMask INTEGER NOT NULL,
    Effects TEXT NOT NULL,
    Auras TEXT NOT NULL,
    ImmuneAoE INTEGER NOT NULL,
    ImmuneChain INTEGER NOT NULL,
    Comment TEXT NOT NULL
)"""


def fingerprint(rows):
    encoded = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode()
    return hashlib.sha256(encoded).hexdigest()


def read_source(source):
    rows = []
    columns = []
    in_ddl = False
    with source.open(encoding="utf-8") as stream:
        for line in stream:
            if line.startswith(f"CREATE TABLE `{TABLE}`"):
                in_ddl = True
            elif in_ddl and line.startswith("  `"):
                columns.append(line.split("`", 2)[1])
            elif in_ddl and line.startswith(")"):
                in_ddl = False
            if line.startswith(f"INSERT INTO `{TABLE}`"):
                for names, row in dump_rows(line, TABLE, source):
                    if names is not None and tuple(names) != COLUMNS:
                        raise ValueError(f"{TABLE}: unexpected INSERT columns")
                    rows.append(row)
    if tuple(columns) != COLUMNS or len(rows) != EXPECTED_ROWS:
        raise ValueError(f"{TABLE}: expected authored columns and {EXPECTED_ROWS} rows; got {len(rows)}")
    rows.sort(key=lambda row: row[0])
    if fingerprint(rows) != EXPECTED_FINGERPRINT:
        raise ValueError(f"{TABLE}: source fingerprint differs from pinned TDB1210")
    return rows


def import_rows(db, rows):
    with closing(sqlite3.connect(db.resolve().as_uri() + "?mode=rw", uri=True)) as conn, conn:
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(DDL)
        actual_columns = tuple(row[1] for row in conn.execute(f"PRAGMA table_info({TABLE})"))
        if actual_columns != COLUMNS:
            raise ValueError(f"{TABLE}: existing SQLite columns differ from native schema")
        conn.execute(f"DELETE FROM {TABLE}")
        conn.executemany(f"INSERT INTO {TABLE} VALUES (?,?,?,?,?,?,?,?,?)", rows)
        imported = conn.execute(f"SELECT * FROM {TABLE} ORDER BY ID").fetchall()
        if fingerprint(imported) != EXPECTED_FINGERPRINT:
            raise ValueError(f"{TABLE}: imported row fingerprint differs from source")
    return {"table": TABLE, "rows": len(rows), "fingerprint": EXPECTED_FINGERPRINT}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", required=True, type=Path, help="existing SQLite copy; no new database")
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    args = parser.parse_args()
    rows = read_source(args.source)
    report = import_rows(args.db, rows)
    report["source"] = str(args.source.resolve())
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
