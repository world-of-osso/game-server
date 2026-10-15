#!/usr/bin/env python3
"""Import Vehicle/VehicleSeat server columns from a local DB2 CSV directory into world.db."""
import argparse
import csv
import json
import sqlite3
from contextlib import closing
from pathlib import Path


VEHICLE_MOVEMENT_COLUMNS = [("Flags", "INTEGER"), ("TurnSpeed", "REAL")]
VEHICLE_COLUMNS = [("ID", "INTEGER"), *[(f"SeatID_{i}", "INTEGER") for i in range(8)],
                   *VEHICLE_MOVEMENT_COLUMNS]
SEAT_COLUMNS = [("ID", "INTEGER"), ("Flags", "INTEGER"), ("AttachmentID", "INTEGER"),
                *[(f"AttachmentOffset_{i}", "REAL") for i in range(3)]]
CATALOGS = [("Vehicle", "vehicle", VEHICLE_COLUMNS),
            ("VehicleSeat", "vehicle_seat", SEAT_COLUMNS)]


def read_rows(path, columns):
    with path.open(newline="") as handle:
        return [tuple((float if kind == "REAL" else int)(row[name])
                      for name, kind in columns) for row in csv.DictReader(handle)]


def import_catalogs(directory, database):
    catalogs = [(table, columns, read_rows(directory / f"{source}.csv", columns))
                for source, table, columns in CATALOGS]
    # Require an existing world database; a typo must not create an empty one.
    with closing(sqlite3.connect(database.resolve().as_uri() + "?mode=rw", uri=True)) as conn:
        with conn:
            for table, columns, rows in catalogs:
                schema = ", ".join(f"{name} {kind} " +
                                   ("PRIMARY KEY" if name == "ID" else "NOT NULL")
                                   for name, kind in columns)
                conn.execute(f"CREATE TABLE IF NOT EXISTS {table} ({schema})")
                conn.execute(f"DELETE FROM {table}")
                if table == "vehicle":
                    existing = {row[1] for row in conn.execute("PRAGMA table_info(vehicle)")}
                    for name, kind in VEHICLE_MOVEMENT_COLUMNS:
                        if name not in existing:
                            # The transaction emptied the catalog: no synthetic default is needed.
                            conn.execute(f"ALTER TABLE vehicle ADD COLUMN {name} {kind} NOT NULL")
                names = ", ".join(name for name, _ in columns)
                placeholders = ", ".join("?" for _ in columns)
                conn.executemany(f"INSERT INTO {table} ({names}) VALUES ({placeholders})", rows)
    return {table: len(rows) for table, _, rows in catalogs}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("csv_directory", type=Path)
    parser.add_argument("database", type=Path)
    args = parser.parse_args()
    print(json.dumps(import_catalogs(args.csv_directory, args.database), sort_keys=True))


if __name__ == "__main__":
    main()
