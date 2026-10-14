#!/usr/bin/env python3
"""Fetch build-pinned Retail DB2 CSVs from wago.tools and import them into world.db.

  db2.py fetch   [--build B] [--csv-root DIR]
  db2.py extract [--build B] [--csv-root DIR]
  db2.py check   [--build B] [--csv-root DIR]
  db2.py import  [--build B] [--csv-root DIR] [--db PATH]

`extract` writes the LOCAL_TABLES CSVs from the local CASC install (db2_casc.py)
instead of wago.tools; `fetch` skips them.

CSVs live in <csv-root>/<build>/<Table>.csv. Import replaces every DB2 table
listed in TABLES inside one transaction (non-DB2 tables are untouched), checks
each CSV header against scripts/db2_schema.sql, and records the build per table
in `db2_build`. The pinned local Forever Skyborne overlay is reapplied in that
same transaction (skyborne_data.py); mixed-source rows use `db2_row_source`.
"""

import argparse
import csv
import email.utils
import random
import re
import sqlite3
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

import db2_casc
import skyborne_data

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
SCHEMA_SQL = SCRIPT_DIR / "db2_schema.sql"
DEFAULT_CSV_ROOT = PROJECT_DIR / "../game-engine/data/db2"
# Follow the installed build: data/db2/active names the current export.
DEFAULT_BUILD = (DEFAULT_CSV_ROOT / "active").resolve().name
DEFAULT_DB = PROJECT_DIR / "data/world.db"
WAGO_URL = "https://wago.tools/db2/{table}/csv?build={build}"
MAX_ATTEMPTS = 6

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


def retry_delay(attempt, error):
    retry_after = (
        error.headers.get("Retry-After")
        if isinstance(error, urllib.error.HTTPError)
        else None
    )
    if retry_after:
        if retry_after.isdigit():
            return float(retry_after)
        when = email.utils.parsedate_to_datetime(retry_after)
        return max(0.0, when.timestamp() - time.time())
    return min(60.0, 2**attempt) * random.uniform(0.5, 1.0)


def is_transient(error):
    if isinstance(error, urllib.error.HTTPError):
        return error.code == 429 or error.code >= 500
    return isinstance(error, (urllib.error.URLError, TimeoutError, ConnectionError))


def download(url):
    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            # wago.tools answers 403 to urllib's default User-Agent.
            request = urllib.request.Request(
                url, headers={"User-Agent": "world-of-osso-db2/1"}
            )
            with urllib.request.urlopen(request, timeout=120) as response:
                if not response.headers.get_content_type() == "text/csv":
                    raise RuntimeError(
                        f"{url}: expected text/csv, got {response.headers.get_content_type()}"
                    )
                return response.read()
        except Exception as error:
            if not is_transient(error) or attempt == MAX_ATTEMPTS:
                raise
            delay = retry_delay(attempt, error)
            print(
                f"  retry {attempt}/{MAX_ATTEMPTS - 1} in {delay:.1f}s: {error}",
                file=sys.stderr,
            )
            time.sleep(delay)


def fetch(build, csv_root):
    out_dir = csv_root / build
    out_dir.mkdir(parents=True, exist_ok=True)
    missing = []
    for db2 in TABLES:
        target = out_dir / f"{db2}.csv"
        if db2 in LOCAL_TABLES:
            continue
        if target.exists():
            print(f"  have {target.name}")
            continue
        try:
            body = download(WAGO_URL.format(table=db2, build=build))
        except urllib.error.HTTPError as error:
            if error.code == 404:
                missing.append(db2)
                continue
            raise
        partial = target.with_suffix(".csv.part")
        partial.write_bytes(body)
        partial.rename(target)
        print(f"  fetched {target.name} ({len(body)} bytes)")
    if missing:
        raise SystemExit(f"wago.tools has no {build} export for: {', '.join(missing)}")


def signed_disenchant_rows(rows):
    """WoWDBDefs5845A4AA: Subclass and ExpansionID are signed8, not uint32."""
    def signed8(value):
        return value - 256 if value >= 128 else value

    return [[row[0], signed8(row[1]), *row[2:6], signed8(row[6]), row[7]] for row in rows]


def extract(build, csv_root):
    """Write each LOCAL_TABLES CSV from local CASC, columns from db2_schema.sql."""
    if build != DEFAULT_BUILD:
        raise SystemExit(f"local CASC holds {DEFAULT_BUILD} only, not {build}")
    out_dir = csv_root / build
    out_dir.mkdir(parents=True, exist_ok=True)
    schema = sqlite3.connect(":memory:")
    schema.executescript(SCHEMA_SQL.read_text())
    for db2, (fdid, layout_hash) in LOCAL_TABLES.items():
        target = out_dir / f"{db2}.csv"
        if target.exists():
            print(f"  have {target.name}")
            continue
        columns = [
            (row[1], row[2])
            for row in schema.execute(f"PRAGMA table_info({table_name(db2)})")
        ]
        header, rows = db2_casc.extract_rows(
            fdid, layout_hash, columns, localized=db2 in LOCALIZED_TABLES,
            inline_arrays={0: 10} if db2 == "OverrideSpellData" else None
        )
        if db2 == "ItemDisenchantLoot":
            rows = signed_disenchant_rows(rows)
        partial = target.with_suffix(".csv.part")
        with partial.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(header)
            writer.writerows(rows)
        partial.rename(target)
        print(f"  extracted {target.name} ({len(rows)} rows, FDID {fdid})")


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
        if header != columns:
            raise SystemExit(
                f"{db2}.csv header does not match {name} in {SCHEMA_SQL.name}:\n"
                f"  csv:    {header}\n  schema: {columns}"
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
            placeholders = ",".join("?" * len(header))
            conn.executemany(f"INSERT INTO {name} VALUES ({placeholders})", rows)
            conn.execute(
                "INSERT INTO db2_build (table_name, build) VALUES (?, ?)", (name, build)
            )
            counts[name] = len(rows)
        skyborne_data.import_rows(conn, skyborne_rows)
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
    parser.add_argument("command", choices=["fetch", "extract", "check", "import"])
    parser.add_argument("--build", default=DEFAULT_BUILD)
    parser.add_argument("--csv-root", type=Path, default=DEFAULT_CSV_ROOT)
    parser.add_argument("--db", type=Path, default=DEFAULT_DB)
    args = parser.parse_args()
    if args.command == "fetch":
        fetch(args.build, args.csv_root)
    elif args.command == "extract":
        extract(args.build, args.csv_root)
    elif args.command == "check":
        check_csvs(args.csv_root / args.build)
    else:
        import_tables(args.build, args.csv_root, args.db)


if __name__ == "__main__":
    main()
