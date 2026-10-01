#!/usr/bin/env python3
"""Import seven pinned Midnight reference tables into an explicit staging SQLite DB."""

import argparse
import csv
from contextlib import closing
from dataclasses import dataclass
import hashlib
import io
import json
import math
from pathlib import Path
import re
import sqlite3
import struct

import db2_casc

BUILD = "12.1.0.69933"
SCHEMA = Path(__file__).with_name("crafting_schema.sql")


@dataclass(frozen=True)
class Table:
    name: str
    filename: str
    layout: int | None
    count: int
    sha256: str


# Content pins bind both CSVs (which carry no build header) and DB2s to the
# genuine local-CASC/Wago snapshot. Field order/types are in crafting_schema.sql.
TABLES = (
    Table(
        "crafting_data",
        "4545611.db2",
        0xC21C854F,
        2785,
        "4321a0f044fbb13359f117e25e34b798677e779d9e79b396d5b32e77ccab35d8",
    ),
    Table(
        "crafting_data_item_quality",
        "4545612.db2",
        0xDE11A051,
        1687,
        "7c7faf62485e0b3e235583c8964bd43d80cc6f96e37b82d63b6abf924eb43e0d",
    ),
    Table(
        "modified_crafting_spell_slot",
        "3390956.db2",
        0x00739B08,
        12769,
        "3883b95c56d2804575562e34d675399bc05a1e974caace5233bdb9609fccab73",
    ),
    Table(
        "modified_crafting_reagent_slot",
        "ModifiedCraftingReagentSlot.csv",
        None,
        393,
        "4a7c96204efb79305f408fe5f65ec81b2a083a5a3545dfaa1acb8ec6d6c5d281",
    ),
    Table(
        "modified_crafting_category",
        "ModifiedCraftingCategory.csv",
        None,
        598,
        "0d00e65bfc0644460ef618a5d856a3517b9e8c38465ae36cb67b0a45f378d1ce",
    ),
    Table(
        "modified_crafting_reagent_item",
        "ModifiedCraftingReagentItem.csv",
        None,
        623,
        "3dc5da789ca3e3f02d3169c1c1083f9bdd41b43f0f4e4e810e0240c09435bfa9",
    ),
    Table(
        "mcr_slot_x_mcr_category",
        "MCRSlotXMCRCategory.csv",
        None,
        910,
        "c0271c733c6da5ecf0b552d1a5dc6306a1ba58ed158e58ced98765d1aadfef0d",
    ),
)


def read_schema():
    """Return the seven CREATE statements and their ordered typed columns."""
    schema = {}
    for statement in SCHEMA.read_text().split(";"):
        if not statement.strip():
            continue
        match = re.fullmatch(r"\s*CREATE TABLE (\w+) \((.*)\)\s*", statement, re.DOTALL)
        if match is None:
            raise ValueError("unexpected crafting schema statement")
        name, body = match.groups()
        columns = []
        for definition in body.split(","):
            column = re.fullmatch(
                r"\s*(\w+) (INTEGER|REAL|TEXT) (PRIMARY KEY|NOT NULL)\s*", definition
            )
            if column is None:
                raise ValueError(f"{name}: invalid column definition")
            columns.append(column.groups()[:2])
        if name in schema:
            raise ValueError(f"duplicate schema table {name}")
        schema[name] = (statement, columns)
    if set(schema) != {t.name for t in TABLES}:
        raise ValueError("crafting schema must define exactly the seven pinned tables")
    return schema


def require_count(table, actual):
    if actual != table.count:
        raise ValueError(f"{table.name}: count {actual}, expected {table.count}")


def decode_db2(table, data, columns):
    header = db2_casc.read_header(data)
    if header["layout_hash"] != table.layout:
        raise ValueError(
            f"{table.name}: layout {header['layout_hash']:08X}, expected {table.layout:08X}"
        )
    require_count(table, header["record_count"])
    _, decoded = db2_casc.decode(data)
    require_count(table, len(decoded))
    rows = []
    for record_id, values in sorted(decoded.items()):
        expected = len(columns) + (table.name == "modified_crafting_spell_slot")
        if len(values) != expected:
            raise ValueError(
                f"{table.name} ID {record_id}: width {len(values)}, expected {expected}"
            )
        if table.name == "modified_crafting_spell_slot":
            # DBD 00739B08 declares inline $relation$SpellID at column 1.
            # The decoder appends the same parent from the relationship map.
            if values[-1] != values[1]:
                raise ValueError(
                    f"{table.name} ID {record_id}: relationship parent {values[-1]} != SpellID {values[1]}"
                )
            values = values[:-1]
        rows.append(
            [db2_casc.typed(raw, kind) for raw, (_, kind) in zip(values, columns)]
        )
    return rows


def parse_number(value, kind):
    if kind == "INTEGER":
        if re.fullmatch(r"-?\d+", value, flags=re.ASCII) is None:
            raise ValueError(f"invalid INTEGER number {value!r}")
        number = int(value)
        if not -(2**31) <= number < 2**32:
            raise ValueError(f"INTEGER number out of DB2 range: {value!r}")
        return number
    try:
        return float(value)
    except ValueError as error:
        raise ValueError(f"invalid REAL number {value!r}") from error


def decode_csv(table, data, columns):
    reader = csv.reader(io.StringIO(data.decode("utf-8"), newline=""), strict=True)
    expected_header = [name for name, _ in columns]
    header = next(reader, None)
    if header != expected_header:
        raise ValueError(
            f"{table.name}: header {header!r}, expected {expected_header!r}"
        )
    rows = []
    for line, values in enumerate(reader, 2):
        if len(values) != len(columns):
            raise ValueError(
                f"{table.name} line {line}: width {len(values)}, expected {len(columns)}"
            )
        try:
            rows.append(
                [
                    value if kind == "TEXT" else parse_number(value, kind)
                    for value, (_, kind) in zip(values, columns)
                ]
            )
        except ValueError as error:
            raise ValueError(f"{table.name} line {line}: {error}") from error
    require_count(table, len(rows))
    return rows


def validate_rows(table, rows, columns):
    id_column = [name for name, _ in columns].index("ID")
    ids = set()
    for row in rows:
        record_id = row[id_column]
        if record_id <= 0 or record_id in ids:
            raise ValueError(f"{table.name}: invalid or duplicate ID {record_id}")
        ids.add(record_id)
        for value, (name, kind) in zip(row, columns):
            if kind == "REAL" and not math.isfinite(value):
                raise ValueError(f"{table.name} ID {record_id}: {name} must be finite")
    return sorted(rows, key=lambda row: row[id_column])


def load_sources(source_dir, schema):
    loaded, report = {}, []
    for table in TABLES:
        _, columns = schema[table.name]
        path = Path(source_dir) / table.filename
        data = path.read_bytes()
        try:
            rows = (
                decode_db2(table, data, columns)
                if table.layout is not None
                else decode_csv(table, data, columns)
            )
            loaded[table.name] = validate_rows(table, rows, columns)
        except (ValueError, struct.error, IndexError, KeyError) as error:
            raise ValueError(f"{path}: {error}") from error
        digest = hashlib.sha256(data).hexdigest()
        if digest != table.sha256:
            raise ValueError(f"{path}: SHA-256 {digest}, expected {table.sha256}")
        report.append(
            {
                "table": table.name,
                "source": table.filename,
                "source_kind": "local-casc" if table.layout is not None else "wago-csv",
                "build": BUILD,
                "layout": f"{table.layout:08X}"
                if table.layout is not None
                else [name for name, _ in columns],
                "rows": len(rows),
                "sha256": digest,
            }
        )
    return loaded, report


def import_data(db, source_dir):
    """Validate all sources, then replace only the seven tables in one transaction.

    Provenance is returned, not written into unrelated metadata tables. The caller
    must supply a staging path; no live/default database is selected implicitly.
    """
    schema = read_schema()
    loaded, report = load_sources(source_dir, schema)
    with closing(sqlite3.connect(db)) as conn, conn:
        conn.execute("BEGIN IMMEDIATE")
        for table in TABLES:
            statement, columns = schema[table.name]
            conn.execute(f'DROP TABLE IF EXISTS "{table.name}"')
            conn.execute(statement)
            placeholders = ",".join("?" for _ in columns)
            conn.executemany(
                f'INSERT INTO "{table.name}" VALUES ({placeholders})',
                loaded[table.name],
            )
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--db", type=Path, required=True, help="explicit staging SQLite database"
    )
    parser.add_argument(
        "--source-dir",
        type=Path,
        required=True,
        help="pinned midnight-20261001 snapshot",
    )
    args = parser.parse_args()
    try:
        report = import_data(args.db, args.source_dir)
    except (OSError, ValueError, sqlite3.Error) as error:
        parser.exit(1, f"crafting import failed: {error}\n")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
