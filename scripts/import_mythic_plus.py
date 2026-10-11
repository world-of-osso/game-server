#!/usr/bin/env python3
"""Import local Retail 12.1.0.69933 challenge DB2s into an existing SQLite COPY.

No downloads, CASC extraction, world population mutation, scoring formula or
season/affix rotation is performed here. Inputs are FileDataID-named WDC5 files.
Layouts/column order: WoWDBDefs and TrinityCore a352b1fa DB2Structure.h.
"""

import argparse
import hashlib
import json
import sqlite3
import sys
import warnings
from contextlib import closing
from pathlib import Path

import db2_casc

BUILD = "12.1.0.69933"
CANONICAL = Path("/home/osso/Projects/world-of-osso/game-server/data/world.db")
# (SQL name, layout hash, columns, string indexes, redundant inline relation).
# Arrays are expanded, preserving every authored threshold/reward slot.
TABLES = {
    801709: (
        "map_challenge_mode",
        0xAAC13A73,
        [
            "Name_lang",
            "ID",
            "MapID",
            "Flags",
            "MaxMedals",
            "ExpansionLevel",
            "RequiredWorldStateID",
        ]
        + [f"CriteriaCount_{i}" for i in range(5)]
        + [f"FirstRewardQuestID_{i}" for i in range(6)]
        + [f"RewardQuestID_{i}" for i in range(6)],
        {0},
        "MapID",
    ),
    1344761: (
        "keystone_affix",
        0x166FC575,
        ["Name_lang", "Description_lang", "ID", "FiledataID"],
        {0, 1},
        None,
    ),
    2965645: (
        "challenge_mode_item_bonus_override",
        0xC75A0165,
        [
            "ID",
            "ItemBonusTreeGroupID",
            "DstItemBonusTreeID",
            "Value",
            "RequiredTimeEventPassed",
            "RequiredTimeEventNotPassed",
            "SrcItemBonusTreeID",
        ],
        set(),
        None,
    ),
    2400282: (
        "mythic_plus_season",
        0xDC94262F,
        [
            "ID",
            "MilestoneSeason",
            "StartTimeEvent",
            "ExpansionLevel",
            "HeroicLFGDungeonMinGear",
        ],
        set(),
        None,
    ),
    1139062: (
        "scenario",
        0xB7C9ECF5,
        [
            "ID",
            "Name_lang",
            "AreaTableID",
            "Type",
            "Flags",
            "UiTextureKitID",
            "UiScenarioDisplayInfoID",
        ],
        {1},
        None,
    ),
    1132761: (
        "scenario_step",
        0x4F644CD3,
        [
            "Description_lang",
            "Title_lang",
            "ID",
            "ScenarioID",
            "CriteriatreeID",
            "RewardQuestID",
            "RelatedStep",
            "Supersedes",
            "OrderIndex",
            "Flags",
            "VisibilityPlayerConditionID",
            "WidgetSetID",
        ],
        {0, 1},
        "ScenarioID",
    ),
}
WIDTHS = {
    "MapID": 16,
    "AreaTableID": 16,
    "ScenarioID": 16,
    "Supersedes": 16,
    "WidgetSetID": 16,
    "OrderIndex": 8,
}
UNSIGNED = {
    "ID",
    "MapID",
    "AreaTableID",
    "ScenarioID",
    "CriteriatreeID",
    "Supersedes",
    "OrderIndex",
    "WidgetSetID",
    "ExpansionLevel",
    "UiTextureKitID",
    "UiScenarioDisplayInfoID",
    "VisibilityPlayerConditionID",
    "SrcItemBonusTreeID",
}


def normalize_value(column, raw):
    if isinstance(raw, str):
        return raw
    width = 16 if column.startswith("CriteriaCount_") else WIDTHS.get(column, 32)
    value = raw & ((1 << width) - 1)
    if column not in UNSIGNED and value & (1 << (width - 1)):
        value -= 1 << width
    return value


def decode_table(source, fdid, definition):
    name, layout, columns, text_columns, inline_relation = definition
    data = (source / f"{fdid}.db2").read_bytes()
    header = db2_casc.read_header(data)
    if header["layout_hash"] != layout:
        raise ValueError(
            f"{name}: layout {header['layout_hash']:08X}, expected {layout:08X}"
        )
    with warnings.catch_warnings(record=True) as recorded:
        warnings.simplefilter("always")
        _, decoded = db2_casc.decode(data, text_columns)
    gaps = [str(warning.message) for warning in recorded]
    for gap in gaps:
        print(f"{name}: {gap}", file=sys.stderr)
    rows = []
    expected_width = len(columns) + bool(inline_relation)
    for record_id, values in sorted(decoded.items()):
        if len(values) != expected_width:
            raise ValueError(
                f"{name}/{record_id}: {len(values)} fields, expected {expected_width}"
            )
        row = [normalize_value(column, value) for column, value in zip(columns, values)]
        if inline_relation and row[columns.index(inline_relation)] != values[-1]:
            raise ValueError(
                f"{name}/{record_id}: inconsistent {inline_relation} relationship"
            )
        if row[columns.index("ID")] != record_id:
            raise ValueError(f"{name}/{record_id}: inconsistent record ID")
        rows.append(row)
    if not rows:
        raise ValueError(f"{name}: no readable records")
    receipt = {
        "rows": len(rows),
        "fdid": fdid,
        "layout": f"{layout:08X}",
        "sha256": hashlib.sha256(data).hexdigest(),
        "gaps": gaps,
    }
    return rows, receipt


def validate_destination(destination):
    if destination.resolve() == CANONICAL.resolve():
        raise ValueError(
            "refusing canonical world.db; create a separate SQLite backup copy"
        )
    if destination.is_symlink():
        raise ValueError("destination must not be a symlink")
    if not destination.is_file():
        raise ValueError("destination must be an existing SQLite backup copy")
    if destination.stat().st_nlink != 1:
        raise ValueError("destination must not be a hardlink")


def replace_table(conn, name, columns, strings, rows):
    ddl = ", ".join(
        f'"{column}" {"TEXT" if index in strings else "INTEGER"} NOT NULL'
        + (" PRIMARY KEY" if column == "ID" else "")
        for index, column in enumerate(columns)
    )
    conn.execute(f'CREATE TABLE IF NOT EXISTS "{name}" ({ddl})')
    conn.execute(f'DELETE FROM "{name}"')
    placeholders = ",".join("?" for _ in columns)
    column_sql = ",".join(f'"{column}"' for column in columns)
    conn.executemany(
        f'INSERT INTO "{name}" ({column_sql}) VALUES ({placeholders})', rows
    )


def import_catalog(destination, source):
    destination, source = Path(destination), Path(source)
    validate_destination(destination)
    decoded = {
        fdid: decode_table(source, fdid, definition)
        for fdid, definition in TABLES.items()
    }
    report = {
        "build": BUILD,
        "tables": {TABLES[fdid][0]: receipt for fdid, (_, receipt) in decoded.items()},
    }
    with closing(sqlite3.connect(destination)) as conn, conn:
        # Explicit BEGIN keeps SQLite DDL and every replacement in one transaction.
        conn.execute("BEGIN IMMEDIATE")
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mythic_plus_import_source (name TEXT PRIMARY KEY, build TEXT NOT NULL, fdid INTEGER NOT NULL, layout TEXT NOT NULL, sha256 TEXT NOT NULL, gaps TEXT NOT NULL)"
        )
        for fdid, (rows, receipt) in decoded.items():
            name, _, columns, strings, _ = TABLES[fdid]
            replace_table(conn, name, columns, strings, rows)
            conn.execute(
                "INSERT OR REPLACE INTO mythic_plus_import_source VALUES (?, ?, ?, ?, ?, ?)",
                (
                    name,
                    BUILD,
                    fdid,
                    receipt["layout"],
                    receipt["sha256"],
                    json.dumps(receipt["gaps"]),
                ),
            )
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--db", required=True, type=Path, help="existing independent SQLite backup copy"
    )
    parser.add_argument(
        "--source-dir",
        required=True,
        type=Path,
        help="local FileDataID-named DB2s from pinned Mainline build",
    )
    parser.add_argument("--report", required=True, type=Path)
    args = parser.parse_args()
    try:
        report = import_catalog(args.db, args.source_dir)
    except (ValueError, OSError, sqlite3.Error) as error:
        parser.exit(1, f"Mythic+ import failed: {error}\n")
    args.report.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Imported {len(report['tables'])} local Mainline tables into {args.db}")


if __name__ == "__main__":
    main()
