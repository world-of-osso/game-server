#!/usr/bin/env python3
"""Check local Retail DB2 CSVs and import them into world.db.

  db2.py check   [--build B] [--csv-root DIR]
  db2.py import  [--build B] [--csv-root DIR] [--db PATH]

CSVs live in <csv-root>/<build>/<Table>.csv. Import replaces every DB2 table
listed in TABLES inside one transaction (non-DB2 tables are untouched), checks
each CSV header against scripts/db2_schema.sql, and updates only its tables'
build receipts in `db2_build`. The pinned local Forever Skyborne overlay is reapplied in that
same transaction (skyborne_data.py), along with the already-authored Zephras
reconstructed factions (zephras_content.py); mixed-source rows use `db2_row_source`.
"""

import argparse
import csv
import hashlib
import json
import os
import re
import sqlite3
import sys
from pathlib import Path

import db2_casc
import skyborne_data
import zephras_content

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
SCHEMA_SQL = SCRIPT_DIR / "db2_schema.sql"
DEFAULT_CSV_ROOT = Path(os.environ.get("DB2_CSV_ROOT", PROJECT_DIR / "../game-engine/data/db2"))
# Follow the installed build: data/db2/active names the current export.
DEFAULT_BUILD = (DEFAULT_CSV_ROOT / "active").resolve().name
DEFAULT_DB = PROJECT_DIR / "data/world.db"

# wago.tools serves no SpellIcon table for Retail 12.x (icons are FileDataIDs in SpellMisc).
TABLES = [
    "SpellName",
    "Spell",
    "SpellMisc",
    "SpellEffect",
    "SummonProperties",
    "OverrideSpellData",
    "SpellPower",
    "SpellCastTimes",
    "SpellRange",
    "SpellCooldowns",
    "SpellDuration",
    "SpellAuraOptions",
    "SpellAuraRestrictions",
    "SpellCategories",
    "SpellCategory",
    "SpellLevels",
    "SpellClassOptions",
    "SpellProcsPerMinute",
    "SpellProcsPerMinuteMod",
    "SpellShapeshift",
    "SpellShapeshiftForm",
    "SpellInterrupts",
    "SpellTargetRestrictions",
    "SpellEquippedItems",
    "SpellRadius",
    "SpellLabel",
    "SpellScaling",
    "SpellEmpower",
    "SpellEmpowerStage",
    "SpellXSpellVisual",
    "SkillLine",
    "SkillLineAbility",
    "ChrClasses",
    "ChrSpecialization",
    "SpecializationSpells",
    "PowerType",
    "ChrClassesXPowerTypes",
    "TraitTree",
    "TraitNode",
    "TraitNodeEntry",
    "TraitDefinition",
    "TraitEdge",
    "TraitCond",
    "TraitCost",
    "TraitCurrency",
    "TraitSubTree",
    "TraitNodeGroup",
    "TraitNodeGroupXTraitNode",
    "TraitNodeXTraitCond",
    "TraitNodeXTraitNodeEntry",
    "TraitTreeLoadout",
    "TraitTreeLoadoutEntry",
    "TraitTreeXTraitCurrency",
    "SkillLineXTraitTree",
    "TraitNodeGroupXTraitCond",
    "TraitNodeGroupXTraitCost",
    "TraitNodeXTraitCost",
    "TraitNodeEntryXTraitCost",
    "TraitNodeEntryXTraitCond",
    "TraitCurrencySource",
    "SpecSetMember",
    "Item",
    "ItemSparse",
    "ItemDisenchantLoot",
    "ItemAppearance",
    "ItemModifiedAppearance",
    "QuestV2",
    "TaxiNodes",
    "TaxiPath",
    "TaxiPathNode",
    "FactionTemplate",
    "AreaTable",
    "RandPropPoints",
    "ItemArmorQuality",
    "ItemArmorShield",
    "ItemArmorTotal",
    "ArmorLocation",
    "ItemDamageOneHand",
    "ItemDamageOneHandCaster",
    "ItemDamageTwoHand",
    "ItemDamageTwoHandCaster",
    "ItemSquishEra",
    "Curve",
    "CurvePoint",
    "DurabilityCosts",
    "DurabilityQuality",
    "GameObjectDisplayInfo",
    "BankTab",
    "CreatureDisplayInfo",
    "CreatureModelData",
    "SpellReagents",
    "SpellTotems",
    "TradeSkillCategory",
    "SkillRaceClassInfo",
    "TotemCategory",
    "ContentTuning",
    "QuestMoneyReward",
    "ExpectedStat",
    "ExpectedStatMod",
    "ContentTuningXExpected",
    "Map",
    "MapDifficulty",
    "AreaTrigger",
    "AreaTriggerActionSet",
    "Difficulty",
    "DungeonEncounter",
    "MapDifficultyXCondition",
    "PlayerCondition",
    "CharacterLoadout",
    "CharacterLoadoutItem",
    "ItemEffect",
    "ItemXItemEffect",
    "Vignette",
    "Mount",
    "MountCapability",
    "MountTypeXCapability",
    "MountXDisplay",
    "QuestInfo",
    "QuestLineXQuest",
    "Campaign",
    "CampaignXQuestLine",
    "CreatureFamily",
]
# Extracted from local CASC by FileDataID (community listfile), pinned to the build's
# WoWDBDefs layout hash: {table: (FileDataID, layout hash)}.
LOCAL_TABLES = {
    "ItemDisenchantLoot": (1134087, 0x5845A4AA),
    "SummonProperties": (1345276, 0xA4CA5ECF),
    "OverrideSpellData": (1002186, 0x297CBDC8),
    "ContentTuning": (1962930, 0xA3E13004),
    "QuestMoneyReward": (1139047, 0x01D6219A),
    "ExpectedStat": (1937326, 0x0FD90F9C),
    "ExpectedStatMod": (1969773, 0x8C41CCCE),
    "ContentTuningXExpected": (2976765, 0x897A4313),
    "Difficulty": (1352127, 0xB810C351),
    "DungeonEncounter": (1347279, 0x35863DC4),
    "MapDifficultyXCondition": (1375435, 0xD514B4CA),
}
# LOCAL_TABLES with localized strings, read from their enUS copy (casc_locale.py).
LOCALIZED_TABLES = {"Difficulty", "DungeonEncounter", "MapDifficultyXCondition"}


def table_name(db2_name):
    """SpellXSpellVisual -> spell_x_spell_visual, ChrClassesXPowerTypes -> chr_classes_x_power_types."""
    return re.sub(
        r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])", "_", db2_name
    ).lower()


def signed_disenchant_rows(rows):
    """WoWDBDefs5845A4AA: Subclass and ExpansionID are signed8, not uint32."""
    def signed8(value):
        return value - 256 if value >= 128 else value

    return [[row[0], signed8(row[1]), *row[2:6], signed8(row[6]), row[7]] for row in rows]


def read_csv(path):
    with path.open(newline="", encoding="utf-8") as handle:
        rows = csv.reader(handle)
        return next(rows), list(rows)


def check_csvs(csv_dir):
    """Fail unless every table's CSV exists and its header matches db2_schema.sql."""
    missing = [db2 for db2 in TABLES if not (csv_dir / f"{db2}.csv").is_file()]
    if missing:
        raise SystemExit(f"Missing CSVs in {csv_dir}: {', '.join(missing)}")
    schema = sqlite3.connect(":memory:")
    schema.executescript(SCHEMA_SQL.read_text())
    for db2 in TABLES:
        name = table_name(db2)
        with (csv_dir / f"{db2}.csv").open(newline="", encoding="utf-8") as handle:
            header = next(csv.reader(handle))
        columns = [row[1] for row in schema.execute(f"PRAGMA table_info({name})")]
        if not set(columns).issubset(header):
            raise SystemExit(
                f"{db2}.csv header does not match {name} in {SCHEMA_SQL.name}:\n"
                f"  csv:    {header}\n  schema: {columns}"
            )


def reapply_zephras_factions(conn):
    """Rebuild approved derived rows from their existing receipt, never fetch content."""
    if not conn.execute(
        "SELECT 1 FROM sqlite_master WHERE name='zephras_row_source'"
    ).fetchone():
        return
    sources = conn.execute(
        "SELECT row_key,scrape,source_json FROM zephras_row_source "
        "WHERE table_name='faction_template' AND kind='reconstructed' ORDER BY row_key"
    ).fetchall()
    for key, scrape, source in sources:
        faction = zephras_content.faction_row(json.loads(source)["react"])
        if json.loads(key) != {"ID": faction["ID"]}:
            raise ValueError(f"Zephras faction source key mismatch: {key}")
        columns = ",".join(faction)
        placeholders = ",".join("?" for _ in faction)
        conn.execute(
            f"INSERT INTO faction_template ({columns}) VALUES ({placeholders})",
            tuple(faction.values()),
        )
        # Derived reactions are not DB2 bytes: FDID0 and layout 'derived' say so.
        conn.execute(
            "INSERT OR REPLACE INTO db2_row_source VALUES (?,?,?,?,?,?)",
            (
                "faction_template", faction["ID"], scrape, 0, "derived",
                hashlib.sha256(source.encode()).hexdigest(),
            ),
        )


def import_tables(build, csv_root, db_path):
    csv_dir = csv_root / build
    check_csvs(csv_dir)
    skyborne_rows = skyborne_data.load_rows(skyborne_data.DEFAULT_SOURCE)
    conn = sqlite3.connect(db_path, isolation_level=None)
    try:
        conn.executescript("BEGIN;\n" + SCHEMA_SQL.read_text())
        counts = {}
        for db2 in TABLES:
            name = table_name(db2)
            header, rows = read_csv(csv_dir / f"{db2}.csv")
            columns = [row[1] for row in conn.execute(f'PRAGMA table_info({name})')]
            indexes = [header.index(column) for column in columns]
            placeholders = ",".join("?" * len(columns))
            conn.executemany(f"INSERT INTO {name} VALUES ({placeholders})",
                             ([row[index] for index in indexes] for row in rows))
            conn.execute(
                "INSERT INTO db2_build (table_name, build) VALUES (?, ?) "
                "ON CONFLICT(table_name) DO UPDATE SET build=excluded.build", (name, build)
            )
            counts[name] = len(rows)
        skyborne_data.import_rows(conn, skyborne_rows)
        reapply_zephras_factions(conn)
        conn.execute("COMMIT")
    except BaseException:
        if conn.in_transaction:
            conn.execute("ROLLBACK")
        raise
    finally:
        conn.close()
    for name, count in counts.items():
        print(f"  {name}: {count}")


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("command", choices=["check", "import"])
    parser.add_argument("--build")
    parser.add_argument("--csv-root", type=Path, default=DEFAULT_CSV_ROOT)
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    args = parser.parse_args()
    if args.build is None:
        receipt = json.loads((args.csv_root / 'active/receipt.json').read_text())
        args.build = receipt['identity']['build']
    if args.command == "check":
        check_csvs(args.csv_root / args.build)
    else:
        import_tables(args.build, args.csv_root, args.db)


if __name__ == "__main__":
    main()
