#!/usr/bin/env python3
"""Import pinned, locally extracted PvP catalogs into an existing world DB.

No TDB/world database writes happen without an explicit destination argument.
All source rows are parsed before a single transaction replaces owned tables.
"""
import argparse
import csv
from pathlib import Path
import sqlite3

CATALOGS = {
    "pvp_currency": ("CurrencyTypes.csv", {
        "id": "ID", "name": "Name_lang", "description": "Description_lang",
        "icon": "InventoryIconFileID", "cap": "MaxQty",
        "weekly_cap": "MaxEarnablePerWeek", "award_condition": "AwardConditionID",
        "world_state_cap": "MaxQtyWorldStateID", "flags": "Flags_0", "flags_b": "Flags_1",
    }),
    "pvp_talent": ("PvpTalent.csv", {
        "id": "ID", "description": "Description_lang", "spec_id": "SpecID",
        "spell_id": "SpellID", "override_spell": "OverridesSpellID", "flags": "Flags",
        "action_spell": "ActionBarSpellID", "category": "PvpTalentCategoryID",
        "level_required": "LevelRequired", "player_condition": "PlayerConditionID",
    }),
    "pvp_talent_category": ("PvpTalentCategory.csv", {"id": "ID", "slot_mask": "TalentSlotMask"}),
    "pvp_talent_slot_unlock": ("PvpTalentSlotUnlock.csv", {
        "id": "ID", "slot_mask": "Slot", "level_required": "LevelRequired",
        "death_knight_level": "DeathKnightLevelRequired", "demon_hunter_level": "DemonHunterLevelRequired",
    }),
    "pvp_honor_reward": ("PrestigeLevelInfo.csv", {
        "id": "ID", "name": "Name_lang", "level": "PrestigeLevel",
        "badge": "BadgeTextureFileDataID", "flags": "Flags", "achievement": "AwardedAchievementID",
    }),
}
TEXT_COLUMNS = {"name", "description"}


def read_catalog(source, table, filename, columns):
    with (source / filename).open(newline="") as handle:
        rows = [tuple(row[field] if column in TEXT_COLUMNS else int(row[field])
                      for column, field in columns.items()) for row in csv.DictReader(handle)]
    if not rows:
        raise ValueError(f"{table}: empty source")
    ids = [row[0] for row in rows]
    if len(ids) != len(set(ids)):
        raise ValueError(f"{table}: duplicate source IDs")
    return rows


def replace_catalog(connection, table, columns, rows):
    definition = ", ".join(f"{column} {'TEXT' if column in TEXT_COLUMNS else 'INTEGER'} NOT NULL"
                           + (" PRIMARY KEY" if column == "id" else "") for column in columns)
    connection.execute(f"CREATE TABLE IF NOT EXISTS {table} ({definition})")
    actual = [row[1] for row in connection.execute(f"PRAGMA table_info({table})")]
    if actual != list(columns):
        raise ValueError(f"{table}: incompatible schema {actual}")
    connection.execute(f"DELETE FROM {table}")
    placeholders = ",".join("?" for _ in columns)
    connection.executemany(f"INSERT INTO {table} VALUES ({placeholders})", rows)


def import_catalogs(source, database):
    source, database = Path(source), Path(database)
    if not database.is_file():
        raise FileNotFoundError(f"existing world database required: {database}")
    parsed = {table: read_catalog(source, table, filename, columns)
              for table, (filename, columns) in CATALOGS.items()}
    with sqlite3.connect(database) as connection:
        connection.execute("BEGIN IMMEDIATE")
        for table, (_, columns) in CATALOGS.items():
            replace_catalog(connection, table, columns, parsed[table])
    return {table: len(rows) for table, rows in parsed.items()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path(__file__).parent / "reference/pvp-12.1.0.69933")
    parser.add_argument("--db", type=Path, required=True)
    args = parser.parse_args()
    print(import_catalogs(args.source, args.db))


if __name__ == "__main__":
    main()
