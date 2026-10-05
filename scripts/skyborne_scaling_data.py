#!/usr/bin/env python3
"""Import nine pinned Forever scaling CSVs into an existing owned scratch world.db.

No downloads, Retail writes, invented fields or cross-source scaling substitutes.
"""

import argparse
from contextlib import closing
import csv
import hashlib
import io
import json
import math
from pathlib import Path
import re
import sqlite3

from skyborne_data import BUILD, PROVENANCE_DDL

SCRIPTS = Path(__file__).resolve().parent
TABLES = {
    "ItemArmorQuality": "skyborne_item_armor_quality",
    "ItemArmorTotal": "skyborne_item_armor_total",
    "ItemArmorShield": "skyborne_item_armor_shield",
    "ArmorLocation": "skyborne_armor_location",
    "ItemDamageOneHand": "skyborne_item_damage_one_hand",
    "ItemDamageOneHandCaster": "skyborne_item_damage_one_hand_caster",
    "ItemDamageTwoHand": "skyborne_item_damage_two_hand",
    "ItemDamageTwoHandCaster": "skyborne_item_damage_two_hand_caster",
    "RandPropPoints": "skyborne_rand_prop_points",
}


def source_columns(name, pin, schema):
    """Reuse Retail names/types only for fields actually authored by this source."""
    table = TABLES[name].removeprefix("skyborne_")
    body = schema.split(f"CREATE TABLE {table} (", 1)[1].split("\n);", 1)[0]
    types = dict(re.findall(r"^\s*(\w+)\s+(INTEGER|REAL)\b", body, re.MULTILINE))
    return [(column, types[column]) for column in pin["columns"]]


def decode_rows(name, data, columns):
    reader = csv.reader(io.StringIO(data.decode("utf-8"), newline=""))
    if next(reader, None) != [column for column, _ in columns]:
        raise ValueError(f"{name}: CSV columns differ from pinned source layout")
    records = []
    ids = set()
    for ordinal, row in enumerate(reader, 2):
        if len(row) != len(columns):
            raise ValueError(f"{name} line {ordinal}: incorrect field count")
        try:
            values = tuple(
                int(value) if kind == "INTEGER" else float(value)
                for value, (_, kind) in zip(row, columns)
            )
        except ValueError as error:
            raise ValueError(f"{name} line {ordinal}: invalid numeric value") from error
        if not all(math.isfinite(value) for value in values):
            raise ValueError(f"{name} line {ordinal}: nonfinite numeric value")
        row_id = values[0]
        if row_id <= 0 or row_id in ids:
            raise ValueError(
                f"{name} line {ordinal}: invalid or duplicate key {row_id}"
            )
        ids.add(row_id)
        records.append(values)
    return records


def load_catalog(catalog, required_levels):
    """Read and validate every source before opening any destination transaction."""
    manifest = json.loads((catalog / "manifest.json").read_text())
    if manifest["build"] != BUILD or manifest["product"] != "wow_classic_beta":
        raise ValueError(f"catalog: expected wow_classic_beta build {BUILD}")
    pins = json.loads((SCRIPTS / "reference/forever-scaling-70205.json").read_text())
    schema = (SCRIPTS / "db2_schema.sql").read_text()
    levels = {1, 2, *required_levels, *manifest["authored_item_levels"]}
    if any(not isinstance(level, int) or level <= 0 for level in levels):
        raise ValueError("catalog: required levels must be positive integers")
    tables = {}
    for name in TABLES:
        pin = pins[name]
        expected_source = {key: value for key, value in pin.items() if key != "columns"}
        if manifest["sources"][name] != expected_source:
            raise ValueError(
                f"{name}: manifest source differs from pinned FDID/layout/hashes"
            )
        output = manifest["outputs"][f"{name}.csv"]
        data = (catalog / f"{name}.csv").read_bytes()
        if hashlib.sha256(data).hexdigest() != output["sha256"]:
            raise ValueError(f"{name}: CSV SHA-256 mismatch")
        if output["columns"] != pin["columns"]:
            raise ValueError(
                f"{name}: manifest columns differ from pinned source layout"
            )
        columns = source_columns(name, pin, schema)
        rows = decode_rows(name, data, columns)
        ids = [row[0] for row in rows]
        if len(rows) != output["rows"] or ids != output["row_ids"]:
            raise ValueError(f"{name}: CSV row count/IDs differ from manifest")
        if name != "ArmorLocation":
            missing = levels - set(ids)
            if missing:
                raise ValueError(f"{name}: missing required levels {sorted(missing)}")
        tables[name] = (columns, rows, pin)
    return tables


def required_kit_levels(connection):
    present = connection.execute(
        "SELECT 1 FROM sqlite_master WHERE name='skyborne_item_sparse'"
    ).fetchone()
    if not present:
        return set()
    return {
        row[0]
        for row in connection.execute(
            "SELECT DISTINCT ItemLevel FROM skyborne_item_sparse"
        )
    }


def publish_tables(connection, tables):
    """Replace only isolated scaling rows and their provenance, as one transaction."""
    connection.execute("BEGIN IMMEDIATE")
    try:
        connection.execute(PROVENANCE_DDL)
        for name, (columns, rows, pin) in tables.items():
            table = TABLES[name]
            definitions = [f'"{column}" {kind} NOT NULL' for column, kind in columns]
            definitions[0] += " PRIMARY KEY"
            connection.execute(
                f"CREATE TABLE IF NOT EXISTS {table} ({','.join(definitions)})"
            )
            actual = [
                (row[1], row[2])
                for row in connection.execute(f"PRAGMA table_info({table})")
            ]
            if actual != columns:
                raise ValueError(
                    f"{table}: existing schema differs from source columns/types"
                )
            connection.execute(f"DELETE FROM {table}")
            connection.execute(
                "DELETE FROM db2_row_source WHERE table_name=?", (table,)
            )
            placeholders = ",".join("?" for _ in columns)
            connection.executemany(f"INSERT INTO {table} VALUES ({placeholders})", rows)
            connection.executemany(
                "INSERT INTO db2_row_source VALUES (?,?,?,?,?,?)",
                [
                    (
                        table,
                        row[0],
                        BUILD,
                        pin["fdid"],
                        pin["layout"],
                        pin["raw_sha256"],
                    )
                    for row in rows
                ],
            )
        connection.commit()
    except BaseException:
        connection.rollback()
        raise


def import_catalog(catalog, db_path, required_levels=()):
    tables = load_catalog(catalog, required_levels)
    # URI mode=rw forbids silently provisioning a new destination.
    with closing(
        sqlite3.connect(f"{db_path.resolve().as_uri()}?mode=rw", uri=True)
    ) as connection:
        levels = required_kit_levels(connection)
        for name, (_, rows, _) in tables.items():
            if name == "ArmorLocation":
                continue
            missing = levels - {row[0] for row in rows}
            if missing:
                raise ValueError(f"{name}: missing used kit levels {sorted(missing)}")
        publish_tables(connection, tables)
    return {TABLES[name]: len(rows) for name, (_, rows, _) in tables.items()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--catalog", type=Path, required=True, help="source-proven items/ CSV catalog"
    )
    parser.add_argument(
        "--db", type=Path, required=True, help="existing owned scratch world.db"
    )
    parser.add_argument(
        "--required-level",
        type=int,
        action="append",
        default=[],
        help="additional item level required by consumer (repeatable)",
    )
    args = parser.parse_args()
    try:
        counts = import_catalog(args.catalog, args.db, args.required_level)
    except (OSError, ValueError, KeyError, TypeError, sqlite3.Error) as error:
        parser.exit(1, f"Forever scaling import failed: {error}\n")
    print(
        f"Forever {BUILD}: "
        + ", ".join(f"{table}={count}" for table, count in counts.items())
    )


if __name__ == "__main__":
    main()
