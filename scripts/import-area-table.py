#!/usr/bin/env python3
"""Import AreaTable alone into an existing world.db; leave other tables unchanged.

Example: python3 scripts/import-area-table.py --csv /path/AreaTable.csv
         --db /path/world.db --build <source-build-or-provenance>
"""

import argparse
from pathlib import Path
import re
import sqlite3

import db2


def import_area_table(csv_path, db_path, build):
    header, rows = db2.read_csv(csv_path)
    ddl = re.search(
        r"DROP TABLE IF EXISTS area_table;\nCREATE TABLE area_table \(.*?\n\);",
        db2.SCHEMA_SQL.read_text(), re.S,
    ).group(0)
    schema = sqlite3.connect(":memory:")
    try:
        schema.executescript(ddl)
        expected = [row[1] for row in schema.execute("PRAGMA table_info(area_table)")]
    finally:
        schema.close()
    if header != expected:
        raise ValueError(f"{csv_path}: header does not match area_table: {header}")
    conn = sqlite3.connect(db_path, isolation_level=None)
    try:
        conn.executescript("BEGIN;\n" + ddl)
        conn.executemany(
            f"INSERT INTO area_table VALUES ({','.join('?' for _ in header)})", rows,
        )
        conn.execute(
            "INSERT OR REPLACE INTO db2_build (table_name, build) VALUES (?, ?)",
            ("area_table", build),
        )
        conn.execute("COMMIT")
    except BaseException:
        if conn.in_transaction:
            conn.execute("ROLLBACK")
        raise
    finally:
        conn.close()
    return len(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--csv", required=True, type=Path)
    parser.add_argument("--db", required=True, type=Path)
    parser.add_argument("--build", required=True)
    args = parser.parse_args()
    count = import_area_table(args.csv, args.db, args.build)
    print(f"area_table: {count} rows ({args.build})")


if __name__ == "__main__":
    main()
