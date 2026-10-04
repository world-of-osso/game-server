#!/usr/bin/env python3
"""Overlay only Skyborne faction templates and Zephras Isle from pinned local Forever DB2s.

Default source files are the exact local CASC snapshots in scripts/reference/.
No network fetch, inferred faction values, or replacement of other Retail rows.
"""

import argparse
import hashlib
import sqlite3
from contextlib import closing
from pathlib import Path

import db2_casc

SCRIPTS = Path(__file__).resolve().parent
DEFAULT_SOURCE = SCRIPTS / "reference/forever-1.60.1.70205"
BUILD = "1.60.1.70205"
SOURCES = {
    "faction_template": (
        1361579,
        0x22B6DC22,
        {3628, 3629},
        "394b3c4f19cd3006c5518f0b5c5addc39273c9035a5ca4b95f974d33874d0d7f",
    ),
    "map": (
        1349477,
        0xD43AFAC3,
        {2991},
        "fe534678ef7bb007cc156b74648b7489c8c07b1f79bd33b010b984e12879ccab",
    ),
}
PROVENANCE_DDL = """CREATE TABLE IF NOT EXISTS db2_row_source (
    table_name TEXT NOT NULL, row_id INTEGER NOT NULL, build TEXT NOT NULL,
    file_data_id INTEGER NOT NULL, layout_hash TEXT NOT NULL, sha256 TEXT NOT NULL,
    PRIMARY KEY (table_name, row_id)
)"""


def source_columns(schema, table):
    columns = [
        (row[1], row[2]) for row in schema.execute(f"PRAGMA table_info({table})")
    ]
    if table == "map":
        # Map.dbd D43AFAC3: Forever adds OceanLiquidTypeID after WdtFileDataID,
        # and NavigationMaxDistance is int32, not Retail's float32.
        index = next(
            i for i, (name, _) in enumerate(columns) if name == "NavigationMaxDistance"
        )
        columns[index] = ("NavigationMaxDistance", "INTEGER")
        columns.insert(index, ("OceanLiquidTypeID", "INTEGER"))
    return columns


def decode_source(source_dir, table, columns):
    fdid, layout, ids, expected_hash = SOURCES[table]
    path = source_dir / f"{fdid}.db2"
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected_hash:
        raise ValueError(f"{path}: SHA-256 {digest}, expected {expected_hash}")
    if db2_casc.read_header(data)["layout_hash"] != layout:
        raise ValueError(f"{path}: expected layout {layout:08X}")
    text_columns = frozenset(i for i, (_, kind) in enumerate(columns) if kind == "TEXT")
    _, rows = db2_casc.decode(
        data,
        text_columns,
        record_ids=ids,
        inline_arrays={6: 2} if table == "map" else None,
    )
    result = []
    for record_id, values in rows.items():
        if len(values) != len(columns):
            raise ValueError(
                f"{path} row {record_id}: {len(values)} fields, expected {len(columns)}"
            )
        result.append(
            {
                name: db2_casc.typed(value, kind)
                for (name, kind), value in zip(columns, values)
            }
        )
    return result


def load_rows(source_dir):
    with closing(sqlite3.connect(":memory:")) as schema:
        schema.executescript((SCRIPTS / "db2_schema.sql").read_text())
        return {
            table: decode_source(source_dir, table, source_columns(schema, table))
            for table in SOURCES
        }


def import_rows(connection, rows):
    """Caller owns the transaction; per-row provenance preserves the Retail table build."""
    connection.execute(PROVENANCE_DDL)
    for table, records in rows.items():
        columns = [row[1] for row in connection.execute(f"PRAGMA table_info({table})")]
        fdid, layout, _, digest = SOURCES[table]
        for record in records:
            values = [record[column] for column in columns]
            placeholders = ",".join("?" for _ in columns)
            connection.execute(
                f"INSERT OR REPLACE INTO {table} VALUES ({placeholders})", values
            )
            connection.execute(
                "INSERT OR REPLACE INTO db2_row_source VALUES (?,?,?,?,?,?)",
                (table, record["ID"], BUILD, fdid, f"{layout:08X}", digest),
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument(
        "--db", type=Path, required=True, help="scratch world.db; no live storage"
    )
    args = parser.parse_args()
    rows = load_rows(args.source_dir)
    # Do not silently create an empty database when the requested scratch file is absent.
    with (
        closing(sqlite3.connect(f"file:{args.db}?mode=rw", uri=True)) as connection,
        connection,
    ):
        import_rows(connection, rows)
    print("Forever 1.60.1.70205: imported faction templates 3628/3629 and map 2991")


if __name__ == "__main__":
    main()
