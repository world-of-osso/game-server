#!/usr/bin/env python3
"""Add the pinned ItemSpec tables to an existing world database."""
import argparse
from contextlib import closing
import csv
from pathlib import Path
import re
import sqlite3

import db2

TABLES = ("ItemSpec", "ItemSpecOverride")


def import_item_specs(database: Path, csv_dir: Path) -> dict[str, int]:
    schema = db2.SCHEMA_SQL.read_text()
    prepared = []
    for name in TABLES:
        table = db2.table_name(name)
        match = re.search(rf"CREATE TABLE {table} \(.*?\n\);", schema, re.DOTALL)
        if match is None:
            raise ValueError(f"schema for {table} absent")
        with (csv_dir / f"{name}.csv").open(newline="", encoding="utf-8") as handle:
            reader = csv.DictReader(handle)
            rows = list(reader)
        prepared.append((table, match.group(), reader.fieldnames, rows))
    with closing(sqlite3.connect(database, isolation_level=None)) as conn:
        conn.execute("BEGIN IMMEDIATE")
        try:
            for table, _, _, _ in prepared:
                if conn.execute("SELECT 1 FROM sqlite_master WHERE name=?", (table,)).fetchone():
                    raise ValueError(f"{table} already exists")
            counts = {}
            for table, create, header, rows in prepared:
                conn.execute(create)
                columns = [row[1] for row in conn.execute(f"PRAGMA table_info({table})")]
                if not set(columns).issubset(header):
                    raise ValueError(f"{table} CSV lacks schema columns")
                conn.executemany(
                    f"INSERT INTO {table} VALUES ({','.join('?' for _ in columns)})",
                    ([row[column] for column in columns] for row in rows),
                )
                counts[table] = len(rows)
            conn.execute("CREATE INDEX item_spec_override_item_id ON item_spec_override(ItemID)")
            conn.execute("COMMIT")
        except BaseException:
            conn.execute("ROLLBACK")
            raise
    return counts


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--db", type=Path, required=True)
    parser.add_argument("--csv-root", type=Path, required=True)
    parser.add_argument("--build", required=True)
    args = parser.parse_args()
    if not args.db.is_file():
        parser.error(f"database does not exist: {args.db}")
    counts = import_item_specs(args.db, args.csv_root / args.build)
    for table, count in counts.items():
        print(f"{table}: {count}")


if __name__ == "__main__":
    main()
