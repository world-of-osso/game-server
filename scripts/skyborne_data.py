#!/usr/bin/env python3
"""Overlay only Skyborne faction templates and Zephras Isle from pinned local Forever DB2s.

Default source files are the exact local CASC snapshots in scripts/reference/.
No network fetch, inferred faction values, or replacement of other Retail rows.
"""

import argparse
import hashlib
import sqlite3
import struct

import zephras_creatures
from contextlib import closing
from pathlib import Path

import db2_casc

SCRIPTS = Path(__file__).resolve().parent
DEFAULT_SOURCE = SCRIPTS / "reference/forever-1.60.1.70205"
BUILD = "1.60.1.70205"
SOURCES = {
    "character_loadout": (
        1344281,
        0x713CE8BB,
        set(range(2373, 2379)),
        "295c28c62daf2925025cb315872420bd5cc4e7d3595593ee616cafa176e47f23",
    ),
    "character_loadout_item": (
        1302846,
        0x0C7A1862,
        None,
        "417f049ff232230bed194c12810ebf85d2efe66e9455b7b67b7973702dab6ce3",
    ),
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
BASE_TABLES = tuple(SOURCES)
SOURCES.update(
    {
        "item": (
            841626,
            0x9A2A4834,
            None,
            "ae2dd49d9ccbb8634f6bc13797bca3eaf4d37687e07e6f90c6fc127e680388ac",
        ),
        "item_sparse": (
            1572924,
            0x6FCC3191,
            None,
            "15fde0ce086da0e642011b1f99b49977b6dfa56e72cf4edc941401dddf139f6d",
        ),
        "item_x_item_effect": (
            3177687,
            0x96F083AD,
            None,
            "2ac01086403c2c6ab5649705dccb585e1c6a03cde6824d7ecb2849be1390b3b4",
        ),
        "item_effect": (
            969941,
            0x4CA77678,
            None,
            "75968bdb475583fa36b17d00057b62d5447d107c18f34e0b2347c68d53807d56",
        ),
        "item_appearance": (
            982462,
            0x481C4281,
            None,
            "b3db3b39fffee4e3231a01ea20bf31f3c38561b33e6f2ec5fc5663b28143d89b",
        ),
        "item_modified_appearance": (
            982457,
            0x03A6C979,
            None,
            "ab4c3376e12c08efa4c9bdcf8b2413b2503e91ea6fa63accbbfe9635e89b8a3d",
        ),
    }
)
ISOLATED_TABLES = frozenset(SOURCES) - set(BASE_TABLES)
# ItemSparse.dbd 6FCC3191: stored-field indices (before array flattening).
SPARSE_ARRAYS = {14: 10, 15: 10, 16: 10, 21: 2, 27: 5, 42: 2, 55: 3}
SPARSE_TYPES = dict.fromkeys(
    (
        5,
        7,
        11,
        12,
        15,
        16,
        17,
        18,
        19,
        21,
        27,
        28,
        29,
        30,
        31,
        32,
        33,
        34,
        51,
        63,
        64,
        65,
        66,
    ),
    "INTEGER",
)
SPARSE_TYPES.update(dict.fromkeys((6, 9, 13, 14, 25, 26), "REAL"))
PROVENANCE_DDL = """CREATE TABLE IF NOT EXISTS db2_row_source (
    table_name TEXT NOT NULL, row_id INTEGER NOT NULL, build TEXT NOT NULL,
    file_data_id INTEGER NOT NULL, layout_hash TEXT NOT NULL, sha256 TEXT NOT NULL,
    PRIMARY KEY (table_name, row_id)
)"""


def source_columns(schema, table):
    columns = [
        (row[1], row[2]) for row in schema.execute(f"PRAGMA table_info({table})")
    ]
    if table == "item":
        columns.insert(6, ("ItemPetFoodID", "INTEGER"))
    if table == "item_sparse":
        columns.append(("AmmunitionType", "INTEGER"))
    if table == "item_modified_appearance":
        columns.append(("RelationItemID", "INTEGER"))
    if table == "character_loadout":
        columns.insert(4, ("Extra", "INTEGER"))
    if table == "character_loadout_item":
        # The stored relation is also returned by the WDC relationship map.
        columns.append(("RelationLoadoutID", "INTEGER"))
    if table == "map":
        # Map.dbd D43AFAC3: Forever adds OceanLiquidTypeID after WdtFileDataID,
        # and NavigationMaxDistance is int32, not Retail's float32.
        index = next(
            i for i, (name, _) in enumerate(columns) if name == "NavigationMaxDistance"
        )
        columns[index] = ("NavigationMaxDistance", "INTEGER")
        columns.insert(index, ("OceanLiquidTypeID", "INTEGER"))
    return columns


def readable_inline_ids(data, header):
    """Pinned ItemModifiedAppearance stores its ID in the first signed bitpacked field."""
    metadata = 204 + header["section_count"] * 40 + header["field_count"] * 4
    bit_offset, width, _, compression, _, _, _ = struct.unpack_from(
        "<HHIIIII", data, metadata
    )
    if header["id_index"] != 0 or compression not in (0, 1, 5):
        raise ValueError("unsupported inline appearance ID encoding")
    ids, _ = zephras_creatures.readable_ids(data, header)
    for index in range(header["section_count"]):
        key, offset, count, *_ = struct.unpack_from("<Q8I", data, 204 + 40 * index)
        if key:
            continue
        for ordinal in range(count):
            start = offset + ordinal * header["record_size"]
            bits = int.from_bytes(data[start : start + header["record_size"]], "little")
            ids.add((bits >> bit_offset) & ((1 << width) - 1))
    return ids


def decode_source(source_dir, table, columns, record_ids=None, readable=False):
    fdid, layout, ids, expected_hash = SOURCES[table]
    path = source_dir / f"{fdid}.db2"
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected_hash:
        raise ValueError(f"{path}: SHA-256 {digest}, expected {expected_hash}")
    header = db2_casc.read_header(data)
    if header["layout_hash"] != layout:
        raise ValueError(f"{path}: expected layout {layout:08X}")
    text_columns = frozenset(i for i, (_, kind) in enumerate(columns) if kind == "TEXT")
    if readable:
        ids = (
            readable_inline_ids(data, header)
            if table == "item_modified_appearance"
            else zephras_creatures.readable_ids(data, header)[0]
        )
    elif record_ids is not None:
        ids = record_ids
    sparse = table == "item_sparse"
    _, rows = db2_casc.decode(
        data,
        text_columns,
        record_ids=ids,
        inline_arrays=SPARSE_ARRAYS if sparse else ({6: 2} if table == "map" else None),
        field_types=SPARSE_TYPES if sparse else None,
    )
    result = []
    for record_id, values in rows.items():
        if len(values) != len(columns):
            raise ValueError(
                f"{path} row {record_id}: {len(values)} fields, expected {len(columns)}"
            )
        result.append(
            {
                name: value if sparse else db2_casc.typed(value, kind)
                for (name, kind), value in zip(columns, values)
            }
        )
    if table == "item_effect":
        # Fixed-record palettes hold uint32; the DBD declares narrow destination values.
        for row in result:
            for name, width, signed in (
                ("LegacySlotIndex", 8, False),
                ("TriggerType", 8, False),
                ("Charges", 16, True),
                ("SpellCategoryID", 16, False),
                ("ChrSpecializationID", 16, False),
            ):
                value = row[name] & ((1 << width) - 1)
                row[name] = (
                    value - (1 << width)
                    if signed and value & (1 << (width - 1))
                    else value
                )
    if table == "character_loadout_item":
        result = [
            row for row in result if row["CharacterLoadoutID"] in range(2373, 2379)
        ]
        for row in result:
            if row.pop("RelationLoadoutID") != row["CharacterLoadoutID"]:
                raise ValueError(f"{path}: loadout relationship mismatch")
    if table == "item_modified_appearance":
        for row in result:
            if row.pop("RelationItemID") != row["ItemID"]:
                raise ValueError(f"{path}: item appearance relationship mismatch")
    return result


def load_item_rows(source_dir, schema, rows):
    ids = {row["ItemID"] for row in rows["character_loadout_item"]}
    for table in ("item", "item_sparse"):
        rows[table] = decode_source(
            source_dir, table, source_columns(schema, table), record_ids=ids
        )
    for table in ("item_modified_appearance", "item_x_item_effect"):
        related = decode_source(
            source_dir, table, source_columns(schema, table), readable=True
        )
        rows[table] = [row for row in related if row["ItemID"] in ids]
    for table, parent, field in (
        ("item_appearance", "item_modified_appearance", "ItemAppearanceID"),
        ("item_effect", "item_x_item_effect", "ItemEffectID"),
    ):
        selected = {row[field] for row in rows[parent] if row[field]}
        rows[table] = decode_source(
            source_dir, table, source_columns(schema, table), record_ids=selected
        )


def load_rows(source_dir):
    with closing(sqlite3.connect(":memory:")) as schema:
        schema.executescript((SCRIPTS / "db2_schema.sql").read_text())
        rows = {
            table: decode_source(source_dir, table, source_columns(schema, table))
            for table in BASE_TABLES
        }
        load_item_rows(source_dir, schema, rows)
        return rows


def validate_loadout_selector(record):
    selector = tuple(
        record[field]
        for field in ("Purpose", "ModID", "Extra", "RaceMasks_0", "RaceMasks_1")
    )
    if selector != (9, 75, 0, 0, 3):
        raise ValueError(
            f"character_loadout {record['ID']}: invalid Forever selector {selector}"
        )


def insert_sourced_row(connection, table, record, columns):
    """Only faction/map have the existing declared selected-row replacement policy."""
    source_table = table.removeprefix("skyborne_")
    fdid, layout, _, digest = SOURCES[source_table]
    provenance = (table, record["ID"], BUILD, fdid, f"{layout:08X}", digest)
    values = tuple(record[column] for column in columns)
    current = connection.execute(
        f"SELECT * FROM {table} WHERE ID=?", (record["ID"],)
    ).fetchone()
    previous = connection.execute(
        "SELECT * FROM db2_row_source WHERE table_name=? AND row_id=?", provenance[:2]
    ).fetchone()
    if current is not None and table not in ("faction_template", "map"):
        if tuple(current) != values or previous != provenance:
            raise ValueError(f"source collision: {table} ID {record['ID']}")
        return
    placeholders = ",".join("?" for _ in columns)
    connection.execute(
        f"INSERT OR REPLACE INTO {table} VALUES ({placeholders})", values
    )
    connection.execute(
        "INSERT OR REPLACE INTO db2_row_source VALUES (?,?,?,?,?,?)", provenance
    )


def validate_item_rows(rows):
    if not ISOLATED_TABLES.intersection(rows):
        return
    missing_tables = ISOLATED_TABLES - rows.keys()
    if missing_tables:
        raise ValueError(f"missing required source tables: {sorted(missing_tables)}")
    references = {row["ItemID"] for row in rows["character_loadout_item"]}
    required = {"item": references, "item_sparse": references}
    for table, parent, field in (
        ("item_effect", "item_x_item_effect", "ItemEffectID"),
        ("item_appearance", "item_modified_appearance", "ItemAppearanceID"),
    ):
        required[table] = {row[field] for row in rows[parent] if row[field]}
    for table, ids in required.items():
        missing = ids - {row["ID"] for row in rows[table]}
        if missing:
            raise ValueError(f"missing required {table} rows: {sorted(missing)}")


def create_isolated_table(connection, table):
    ddl = (SCRIPTS / "db2_schema.sql").read_text()
    body = ddl.split(f"CREATE TABLE {table} (", 1)[1].split("\n);", 1)[0]
    connection.execute(f"CREATE TABLE IF NOT EXISTS skyborne_{table} ({body}\n)")


def import_rows(connection, rows):
    """Atomic overlay; never change a Retail item definition or commit caller changes."""
    validate_item_rows(rows)
    connection.execute("SAVEPOINT skyborne_overlay")
    try:
        connection.execute(PROVENANCE_DDL)
        for source_table, records in rows.items():
            table = source_table
            if source_table in ISOLATED_TABLES:
                create_isolated_table(connection, source_table)
                table = f"skyborne_{source_table}"
            columns = [
                row[1] for row in connection.execute(f"PRAGMA table_info({table})")
            ]
            for record in records:
                if table == "character_loadout":
                    validate_loadout_selector(record)
                insert_sourced_row(connection, table, record, columns)
    except BaseException:
        connection.execute("ROLLBACK TO skyborne_overlay")
        connection.execute("RELEASE skyborne_overlay")
        raise
    connection.execute("RELEASE skyborne_overlay")


def project_items(connection):
    """Rebuild isolated content with the existing item projection, never Retail substitutes."""
    if (
        connection.execute(
            "SELECT 1 FROM sqlite_master WHERE name='skyborne_item'"
        ).fetchone()
        is None
    ):
        has_provenance = connection.execute(
            "SELECT 1 FROM sqlite_master WHERE name='db2_row_source'"
        ).fetchone()
        if (
            has_provenance
            and connection.execute(
                "SELECT 1 FROM db2_row_source WHERE table_name='character_loadout' AND build=? LIMIT 1",
                (BUILD,),
            ).fetchone()
        ):
            raise ValueError("missing required skyborne_item table")
        return
    from content_policy import ITEM_COLUMNS

    references = {
        row[0]
        for row in connection.execute(
            "SELECT li.ItemID FROM character_loadout_item li JOIN db2_row_source p "
            "ON p.table_name='character_loadout_item' AND p.row_id=li.ID WHERE p.build=?",
            (BUILD,),
        )
    }
    for table in ("skyborne_item", "skyborne_item_sparse"):
        present = {row[0] for row in connection.execute(f"SELECT ID FROM {table}")}
        missing = references - present
        if missing:
            raise ValueError(f"missing required {table} rows: {sorted(missing)}")
    projection = ITEM_COLUMNS.replace(
        "item_modified_appearance", "skyborne_item_modified_appearance"
    ).replace("JOIN item_appearance", "JOIN skyborne_item_appearance")
    connection.execute("DROP TABLE IF EXISTS skyborne_content_item")
    connection.execute(
        f"CREATE TABLE skyborne_content_item AS SELECT {projection}, NULL AS SourceEntry FROM skyborne_item_sparse s JOIN skyborne_item i ON i.ID=s.ID"
    )
    connection.execute(
        "CREATE UNIQUE INDEX skyborne_content_item_id ON skyborne_content_item(ID)"
    )
    connection.execute(
        "DELETE FROM db2_row_source WHERE table_name='skyborne_content_item'"
    )
    connection.execute(
        "INSERT INTO db2_row_source SELECT 'skyborne_content_item',row_id,build,file_data_id,layout_hash,sha256 FROM db2_row_source WHERE table_name='skyborne_item_sparse'"
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
        connection.execute("BEGIN IMMEDIATE")
        import_rows(connection, rows)
        project_items(connection)
    print(
        "Forever 1.60.1.70205: imported factions/map, six loadouts and 30 isolated kit items"
    )


if __name__ == "__main__":
    main()
