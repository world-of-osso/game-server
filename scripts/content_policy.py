#!/usr/bin/env python3
"""Derive the Retail-first content_* tables the server reads from AzerothCore + Retail DB2 tables.

  content_policy.py --source-dir DIR [--db data/world.db] [--csv-root DIR] [--overrides CSV] [--report CSV] [--check]

--check only verifies the inputs exist (import-azerothcore.sh preflight).

Needs world.db with the AzerothCore dump tables (mysql_dump.py), the TrinityCore
Retail tdb_* tables (mysql_dump.py --prefix tdb_) and the Retail DB2 tables
(db2.py). Policy (docs/wiki/systems/content-import.md):

- An AzerothCore item/spell id is kept when the Retail row with the same id has
  the same name (case-insensitive, trimmed), or when scripts/content_overrides.csv
  lists it. Item names come from item_template; Wrath spell names from the Wrath
  Classic SpellName CSV (AzerothCore ships none).
- content_item takes every attribute from Retail item_sparse/item/appearance.
- Quests are kept when their id is in quest_v2; unmapped reward items/spells are
  zeroed. Vendor and loot rows need a mapped item.
- Game object templates are kept when their displayId is 0 (no model) or a Retail
  game_object_display_info row; spawns and addons need a kept template.
- content_creature_template_model: a template with any display absent from Retail
  creature_display_info takes the model rows of the same entry in TrinityCore's
  Retail world DB (tdb_creature_template_model) when that entry has the same name (or a
  reviewed creature override, or a Retail spawn of maps 0/1 references it) and every one
  of its displays exists in Retail.
- content_creature: maps 0 and 1 (RETAIL_MAPS) spawn TrinityCore's base-phase
  Retail spawns (PhaseId 0, PhaseGroup 0, terrainSwapMap -1, difficulty 0 in
  spawnDifficulties) at guid + RETAIL_GUID_OFFSET instead of AzerothCore's; other
  maps keep AzerothCore's. content_creature_template adds the TrinityCore template of
  every spawned entry AzerothCore lacks; every template TrinityCore tunes carries its
  ContentTuningID and that ContentTuning's level range (the server scales within it).
  Addons, formations, waypoints and event memberships follow the spawns' source.
- content_pool_template/_members, content_game_event_pool: TrinityCore's spawn pools (PoolMgr) of the
  Retail spawns; AzerothCore's pools are not imported (no AzerothCore-sourced map spawns creatures).

Every dropped row or zeroed reward is written to --report. Runs in one transaction.
"""

import argparse
import csv
import sqlite3
import sys
from pathlib import Path

import derived_merchants
import zephras_content

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))
import db2
import retail_content
import wowhead_loot
from content_common import Report, clone_table
from economy_crafting import crafting_item_ids

PROJECT_DIR = SCRIPT_DIR.parent
WRATH_BUILD = "3.4.0.47659"
DEFAULT_OVERRIDES = SCRIPT_DIR / "content_overrides.csv"
DEFAULT_DERIVED_MERCHANTS = SCRIPT_DIR / "derived-merchants.json"
DEFAULT_REPORT = PROJECT_DIR / "data/reports/content-dropped.csv"

REWARD_ITEMS = [(f"RewardItem{n}", f"RewardAmount{n}") for n in range(1, 5)] + [
    (f"RewardChoiceItemID{n}", f"RewardChoiceItemQuantity{n}") for n in range(1, 7)
]
REWARD_SPELLS = ["RewardSpell", "RewardDisplaySpell"]
QUEST_CHILDREN = {
    "quest_template_addon": "ID",
    "quest_offer_reward": "ID",
    "quest_request_items": "ID",
    "quest_poi": "QuestID",
    "quest_poi_points": "QuestID",
    "creature_queststarter": "quest",
    "creature_questender": "quest",
    "gameobject_queststarter": "quest",
    "gameobject_questender": "quest",
    "areatrigger_involvedrelation": "quest",
}
LOOT_TABLES = [
    "creature_loot_template",
    "gameobject_loot_template",
    "reference_loot_template",
    "item_loot_template",
]
# SkillLineAbilityAcquireMethod AutomaticCharLevel / LearnedOrAutomaticCharLevel (TrinityCore DBCEnums.h):
# the spells a character gains by level. Learned (0) rows are talent/helper spells (Hotbar Slot 01,
# Bloodthirst Heal); NeverLearned (3) is excluded as in spell_info/class_data.rs.
ITEM_COLUMNS = """
    s.ID, s.Display_lang AS Name, s.OverallQualityID AS Quality, s.ItemLevel, s.RequiredLevel,
    s.BuyPrice, s.SellPrice, s.VendorStackCount, s.Stackable, s.MaxCount,
    i.ClassID, i.SubclassID, s.InventoryType, s.Bonding, s.ItemDelay, s.DmgVariance,
    s.AllowableClass, s.AllowableRaces_0, s.AllowableRaces_1, s.RequiredSkill, s.RequiredSkillRank,
    s.StartQuestID, s.ItemSet, s.Flags_0, s.Flags_1, s.Flags_2, s.Flags_3, s.Flags_4,
    (SELECT a.ItemDisplayInfoID FROM item_modified_appearance m JOIN item_appearance a ON a.ID = m.ItemAppearanceID
     WHERE m.ItemID = s.ID AND m.ItemAppearanceModifierID = 0 ORDER BY m.OrderIndex LIMIT 1) AS DisplayInfoID,
""" + ",\n".join(
    f"    s.StatModifier_bonusStat_{n}, s.StatPercentEditor_{n}" for n in range(10)
)


def normalize(name):
    return (name or "").strip().casefold()


def load_overrides(path, conn):
    """{kind: {source_id: retail_id}}; fails when a reviewed name no longer matches either side."""
    source_name_sql = {
        "item": "SELECT name FROM item_template WHERE entry = ?",
        "creature": "SELECT name FROM creature_template WHERE entry = ?",
    }
    retail_name_sql = {
        "item": "SELECT Display_lang FROM item_sparse WHERE ID = ?",
        "spell": "SELECT Name_lang FROM spell_name WHERE ID = ?",
        "creature": "SELECT name FROM tdb_creature_template WHERE entry = ?",
    }
    overrides = {"item": {}, "spell": {}, "creature": {}}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            kind, source_id, retail_id = (
                row["kind"],
                int(row["source_id"]),
                int(row["retail_id"]),
            )
            if not row["reason"].strip():
                raise SystemExit(f"{path.name}: {kind} {source_id} has no reason")
            retail = conn.execute(retail_name_sql[kind], (retail_id,)).fetchone()
            if retail is None or retail[0] != row["retail_name"]:
                raise SystemExit(
                    f"{path.name}: Retail {kind} {retail_id} is {retail}, reviewed as {row['retail_name']!r}"
                )
            if kind in source_name_sql:
                source = conn.execute(source_name_sql[kind], (source_id,)).fetchone()
                if source is None or source[0] != row["source_name"]:
                    raise SystemExit(
                        f"{path.name}: source {kind} {source_id} is {source}, reviewed as {row['source_name']!r}"
                    )
            overrides[kind][source_id] = retail_id
    return overrides


def map_ids(source_names, retail_names, overrides):
    """(mapping source id -> retail id, {source id: drop reason})."""
    mapping, reasons = {}, {}
    for source_id, name in source_names.items():
        if source_id in overrides:
            mapping[source_id] = overrides[source_id]
        elif source_id not in retail_names:
            reasons[source_id] = "no Retail row"
        elif normalize(retail_names[source_id]) != normalize(name):
            reasons[source_id] = f"Retail name {retail_names[source_id]!r} != {name!r}"
        else:
            mapping[source_id] = source_id
    return mapping, reasons


def read_wrath_spell_names(path):
    with path.open(newline="", encoding="utf-8") as handle:
        return {int(row["ID"]): row["Name_lang"] for row in csv.DictReader(handle)}


def create_map_table(conn, name, mapping):
    conn.execute(f"DROP TABLE IF EXISTS {name}")
    conn.execute(
        f"CREATE TEMP TABLE {name} (source_id INTEGER PRIMARY KEY, retail_id INTEGER NOT NULL)"
    )
    conn.executemany(f"INSERT INTO {name} VALUES (?, ?)", mapping.items())


def build_items(conn, item_map, item_reasons, retail_ids, report):
    """content_item: installed Retail definitions, retaining mapped AzerothCore SourceEntry."""
    for source_id, reason in sorted(item_reasons.items()):
        report.drop("item_template", f"entry={source_id}", source_id, reason)
    conn.execute(
        "CREATE TEMP TABLE item_source AS SELECT retail_id, source_id FROM temp.item_map"
    )
    conn.executemany(
        "INSERT INTO temp.item_source VALUES (?, NULL)",
        ((item,) for item in sorted(retail_ids - set(item_map.values()))),
    )
    project_retail_items(conn)
    db2.skyborne_data.project_items(conn)


def project_retail_items(conn):
    """Replace Retail definitions from the selected IDs and reviewed source mappings."""
    conn.execute("DROP TABLE IF EXISTS content_item")
    conn.execute(f"""CREATE TABLE content_item AS SELECT {ITEM_COLUMNS}, im.source_id AS SourceEntry
        FROM temp.item_source im JOIN item_sparse s ON s.ID = im.retail_id JOIN item i ON i.ID = s.ID""")
    conn.execute("CREATE UNIQUE INDEX content_item_id ON content_item(ID)")


def refresh_items(conn):
    """Reproject installed definitions only, retaining reviewed SourceEntry mappings."""
    conn.execute("""CREATE TEMP TABLE item_source AS
        SELECT s.ID AS retail_id, c.SourceEntry AS source_id
        FROM item_sparse s JOIN item i ON i.ID=s.ID LEFT JOIN content_item c ON c.ID=s.ID""")
    project_retail_items(conn)
    conn.execute("DROP TABLE temp.item_source")


def build_vendors(conn, report):
    clone_table(conn, "npc_vendor", "content_npc_vendor")
    conn.execute("""INSERT INTO content_npc_vendor
        SELECT v.entry, v.slot, COALESCE(m.retail_id, v.item), v.maxcount, v.incrtime, v.ExtendedCost, v.VerifiedBuild
        FROM npc_vendor v LEFT JOIN temp.item_map m ON m.source_id = v.item
        WHERE v.item < 0 OR m.retail_id IS NOT NULL""")
    for (
        entry,
        item,
    ) in conn.execute("""SELECT entry, item FROM npc_vendor v WHERE item > 0
            AND NOT EXISTS (SELECT 1 FROM temp.item_map m WHERE m.source_id = v.item)"""):
        report.drop("npc_vendor", f"entry={entry}", item, "item not mapped")


def build_quests(conn, item_map, spell_map, report):
    clone_table(conn, "quest_template", "content_quest_template")
    conn.execute(
        "INSERT INTO content_quest_template SELECT * FROM quest_template WHERE ID IN (SELECT ID FROM quest_v2)"
    )
    for (quest,) in conn.execute(
        "SELECT ID FROM quest_template WHERE ID NOT IN (SELECT ID FROM quest_v2)"
    ):
        report.drop(
            "quest_template", f"ID={quest}", quest, "quest id not in Retail QuestV2"
        )
    columns = [c for pair in REWARD_ITEMS for c in pair] + REWARD_SPELLS
    for row in conn.execute(
        f"SELECT ID, {', '.join(columns)} FROM content_quest_template"
    ).fetchall():
        quest, values = row[0], dict(zip(columns, row[1:]))
        updates = {}
        for item_col, amount_col in REWARD_ITEMS:
            source = values[item_col]
            if source and source not in item_map:
                report.drop(
                    "quest_template",
                    f"ID={quest} {item_col}",
                    source,
                    "reward item not mapped; zeroed",
                )
                updates.update({item_col: 0, amount_col: 0})
            elif source:
                updates[item_col] = item_map[source]
        for spell_col in REWARD_SPELLS:
            source = values[spell_col]
            if source > 0 and source not in spell_map:
                report.drop(
                    "quest_template",
                    f"ID={quest} {spell_col}",
                    source,
                    "reward spell not mapped; zeroed",
                )
                updates[spell_col] = 0
            elif source > 0:
                updates[spell_col] = spell_map[source]
        changed = {c: v for c, v in updates.items() if v != values[c]}
        if changed:
            conn.execute(
                f"UPDATE content_quest_template SET {', '.join(f'{c} = ?' for c in changed)} WHERE ID = ?",
                (*changed.values(), quest),
            )
    for table, key in QUEST_CHILDREN.items():
        clone_table(conn, table, f"content_{table}")
        conn.execute(
            f"INSERT INTO content_{table} SELECT * FROM {table} WHERE {key} IN (SELECT ID FROM content_quest_template)"
        )
        for (quest,) in conn.execute(
            f"SELECT {key} FROM {table} WHERE {key} NOT IN (SELECT ID FROM content_quest_template)"
        ):
            report.drop(table, f"{key}={quest}", quest, "quest dropped")


def build_loot(conn, report):
    for table in LOOT_TABLES:
        clone_table(conn, table, f"content_{table}")
        entry = (
            "(SELECT retail_id FROM temp.item_map WHERE source_id = l.Entry)"
            if table == "item_loot_template"
            else "l.Entry"
        )
        item = "CASE WHEN l.Reference != 0 THEN l.Item ELSE (SELECT retail_id FROM temp.item_map WHERE source_id = l.Item) END"
        conn.execute(f"""INSERT INTO content_{table}
            SELECT e, i, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount, Comment FROM (
                SELECT {entry} AS e, {item} AS i, l.* FROM {table} l) WHERE e IS NOT NULL AND i IS NOT NULL""")
        for source_entry, source_item, e, i in conn.execute(
            f"SELECT l.Entry, l.Item, {entry}, {item} FROM {table} l"
        ).fetchall():
            if e is None:
                report.drop(
                    table,
                    f"Entry={source_entry}",
                    source_entry,
                    "container item not mapped",
                )
            elif i is None:
                report.drop(
                    table, f"Entry={source_entry}", source_item, "item not mapped"
                )


def build_gameobjects(conn, report):
    clone_table(conn, "gameobject_template", "content_gameobject_template")
    conn.execute("""INSERT INTO content_gameobject_template SELECT * FROM gameobject_template t
        WHERE t.displayId = 0 OR t.displayId IN (SELECT ID FROM game_object_display_info)""")
    for (
        entry,
        display,
    ) in conn.execute("""SELECT entry, displayId FROM gameobject_template
            WHERE entry NOT IN (SELECT entry FROM content_gameobject_template)"""):
        report.drop(
            "gameobject_template",
            f"entry={entry}",
            display,
            "display not in Retail GameObjectDisplayInfo",
        )
    for table, key in (("gameobject_template_addon", "entry"), ("gameobject", "id")):
        clone_table(conn, table, f"content_{table}")
        kept = f"{key} IN (SELECT entry FROM content_gameobject_template)"
        conn.execute(f"INSERT INTO content_{table} SELECT * FROM {table} WHERE {kept}")
        row_key = "guid" if table == "gameobject" else "entry"
        for row, entry in conn.execute(
            f"SELECT {row_key}, {key} FROM {table} WHERE NOT ({kept})"
        ):
            report.drop(table, f"{row_key}={row}", entry, "template dropped")


def retail_model_reason(conn, entry, name, reviewed):
    """None when TrinityCore's Retail models for `entry` may replace AzerothCore's, else why not."""
    retail = conn.execute(
        "SELECT name FROM tdb_creature_template WHERE entry = ?", (entry,)
    ).fetchone()
    if retail is None:
        return "no TrinityCore Retail template"
    if not reviewed and normalize(retail[0]) != normalize(name):
        return f"TrinityCore Retail name {retail[0]!r} != {name!r}"
    displays = [
        d
        for (d,) in conn.execute(
            "SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID = ? ORDER BY Idx",
            (entry,),
        )
    ]
    if not displays:
        return "no TrinityCore Retail models"
    absent = [
        d
        for d in displays
        if not conn.execute(
            "SELECT 1 FROM creature_display_info WHERE ID = ?", (d,)
        ).fetchone()
    ]
    if absent:
        return f"TrinityCore Retail displays {absent} absent from Retail CreatureDisplayInfo"
    return None


def build_creature_models(conn, overrides, report):
    clone_table(conn, "creature_template_model", "content_creature_template_model")
    conn.execute(
        "INSERT INTO content_creature_template_model SELECT * FROM creature_template_model"
    )
    # A Retail spawn references its entry by TrinityCore's id, so that entry is TrinityCore's
    # creature even when TrinityCore renamed it (Pygmy Surf Crawler 3106 -> Surf Crawler).
    retail_spawned = retail_content.retail_spawned_entries(conn)
    wrath_only = conn.execute("""SELECT DISTINCT m.CreatureID, t.name FROM creature_template_model m
        LEFT JOIN creature_template t ON t.entry = m.CreatureID
        WHERE m.CreatureDisplayID NOT IN (SELECT ID FROM creature_display_info) ORDER BY m.CreatureID""").fetchall()
    for entry, name in wrath_only:
        retail_entry = overrides.get(entry, entry)
        reason = retail_model_reason(
            conn, retail_entry, name, entry in overrides or entry in retail_spawned
        )
        if reason:
            report.drop(
                "creature_template_model",
                f"CreatureID={entry}",
                entry,
                f"display absent from Retail kept: {reason}",
            )
            continue
        conn.execute(
            "DELETE FROM content_creature_template_model WHERE CreatureID = ?", (entry,)
        )
        conn.execute(
            """INSERT INTO content_creature_template_model
            SELECT ?, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild
            FROM tdb_creature_template_model WHERE CreatureID = ?""",
            (entry, retail_entry),
        )


def apply_policy(
    conn, wrath_spell_names, overrides_path, report, derived_merchant_records=()
):
    crafting_items = crafting_item_ids(conn)
    for item_id in sorted(crafting_items):
        for table in ("item_sparse", "item"):
            if (
                conn.execute(f"SELECT 1 FROM {table} WHERE ID=?", (item_id,)).fetchone()
                is None
            ):
                report.drop(
                    table,
                    f"ID={item_id}",
                    item_id,
                    "crafting item missing Retail source row",
                )
                crafting_items.discard(item_id)
    retail_content.create_retail_map_table(conn)
    wowhead_loot.load_wowhead_drops(conn, wowhead_loot.wowhead_root())
    retail_items = dict(conn.execute("SELECT ID, Display_lang FROM item_sparse"))
    retail_spells = dict(conn.execute("SELECT ID, Name_lang FROM spell_name"))
    overrides = load_overrides(overrides_path, conn)
    item_map, item_reasons = map_ids(
        dict(conn.execute("SELECT entry, name FROM item_template")),
        retail_items,
        overrides["item"],
    )
    spell_map, _ = map_ids(wrath_spell_names, retail_spells, overrides["spell"])
    create_map_table(conn, "item_map", item_map)
    create_map_table(conn, "spell_map", spell_map)
    merchant_items = {
        vendor["item"]
        for merchant in derived_merchant_records
        for vendor in merchant["vendors"]
    }
    build_items(
        conn,
        item_map,
        item_reasons,
        retail_content.retail_item_ids(conn) | crafting_items | merchant_items,
        report,
    )
    build_vendors(conn, report)
    build_quests(conn, item_map, spell_map, report)
    build_loot(conn, report)
    build_gameobjects(conn, report)
    build_creature_models(conn, overrides["creature"], report)
    retail_content.build_creatures(conn, report)
    retail_content.build_gameobjects(conn, report)
    retail_content.build_pools(conn, report)
    retail_content.build_vendors(conn, report)
    retail_content.build_loot(conn, report)
    retail_content.build_gossip(conn, report)
    retail_content.build_quests(conn, report)
    retail_content.build_creature_text(conn)
    if derived_merchant_records:
        derived_merchants.apply_derived_merchants(conn, derived_merchant_records)
    # Regeneration must retain instance population, not just one-time copy imports.
    import import_instance_population

    import_instance_population.apply_population(conn, report)


def main():
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--db", type=Path, default=PROJECT_DIR / "data/world.db")
    parser.add_argument("--csv-root", type=Path, default=db2.DEFAULT_CSV_ROOT)
    parser.add_argument(
        "--source-dir", type=Path, help="Forever DB2 directory; required for rebuild"
    )
    parser.add_argument("--overrides", type=Path, default=DEFAULT_OVERRIDES)
    parser.add_argument(
        "--derived-merchants", type=Path, default=DEFAULT_DERIVED_MERCHANTS
    )
    parser.add_argument(
        "--zephras-content", type=Path, default=zephras_content.DEFAULT_CONTENT
    )
    parser.add_argument(
        "--zephras-report",
        type=Path,
        default=PROJECT_DIR / "target/zephras-content-report.json",
    )
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--mode", choices=("rebuild", "items-refresh", "world-promote", "world-census"), default="rebuild")
    parser.add_argument("--maps", help="comma-separated explicit open-world map IDs; world-promote only")
    args = parser.parse_args()
    if args.mode == "rebuild" and args.source_dir is None:
        parser.error("--source-dir is required for rebuild")
    if args.mode == "world-census":
        import world_population

        if not args.maps:
            parser.error("world-census requires explicit hosted --maps")
        maps = [int(value) for value in args.maps.split(",")]
        with sqlite3.connect(f"file:{args.db}?mode=ro", uri=True) as conn:
            for map_id in maps:
                if conn.execute("SELECT InstanceType FROM map WHERE ID=?", (map_id,)).fetchone() != (0,):
                    parser.error(f"census map {map_id} is not a registered open world")
            result = world_population.zone_census(conn, maps, args.csv_root)
        world_population.write_report(args.report, result)
        print(f"Read-only census: {len(result)} zone rows; report: {args.report}")
        return
    if args.mode == "world-promote":
        import world_population

        if not args.maps:
            parser.error("world-promote requires --maps")
        maps = [int(value) for value in args.maps.split(",")]
        if args.check:
            world_population.population.validate_copy(args.db)
            with sqlite3.connect(f"file:{args.db}?mode=ro", uri=True) as conn:
                world_population.validate_maps(conn, maps)
            return
        result = world_population.import_database(args.db, maps)
        world_population.write_report(args.report, result)
        print(f"Promoted open-world maps {result['target_maps']}; report: {args.report}")
        return
    if args.maps:
        parser.error("--maps is only valid with --mode world-promote")
    if args.mode == "items-refresh":
        with sqlite3.connect(f"file:{args.db}?mode=rw", uri=True) as conn:
            if not args.check:
                conn.execute("BEGIN")
                refresh_items(conn)
            print(f"content_item: {conn.execute('SELECT count(*) FROM content_item').fetchone()[0]}")
        return
    wrath_path = args.csv_root / WRATH_BUILD / "SpellName.csv"
    missing = [str(p) for p in (wrath_path, args.overrides) if not p.is_file()]
    if missing:
        raise SystemExit(f"Missing content policy inputs: {', '.join(missing)}")
    merchants = derived_merchants.load_manifest(args.derived_merchants)
    zephras = zephras_content.load_input(args.zephras_content, source_dir=args.source_dir)
    if args.check:
        return
    wrath_spell_names = read_wrath_spell_names(wrath_path)
    report = Report()
    conn = sqlite3.connect(args.db, isolation_level=None)
    try:
        conn.execute("BEGIN")
        # Retire old ownership before rebuilding tables with fresh Retail rows.
        zephras_content.clear_overlay(conn)
        apply_policy(conn, wrath_spell_names, args.overrides, report, merchants)
        zephras_report = zephras_content.apply_overlay(conn, *zephras)
        conn.execute("COMMIT")
    except BaseException:
        if conn.in_transaction:
            conn.execute("ROLLBACK")
        raise
    finally:
        conn.close()
    report.write(args.report)
    zephras_content.write_report(args.zephras_report, zephras_report)
    conn = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    for name in sorted(
        r[0]
        for r in conn.execute(
            "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'content_%'"
        )
    ):
        print(f"  {name}: {conn.execute(f'SELECT count(*) FROM {name}').fetchone()[0]}")
    conn.close()
    for table, count in sorted(report.counts.items()):
        print(f"  dropped {table}: {count}")
    print(f"  report: {args.report}")


if __name__ == "__main__":
    main()
