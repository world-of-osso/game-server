#!/usr/bin/env python3
"""Derive the Retail-first content_* tables the server reads from AzerothCore + Retail DB2 tables.

  content_policy.py [--db data/world.db] [--csv-root DIR] [--overrides CSV] [--report CSV] [--check]

--check only verifies the inputs exist (import-azerothcore.sh preflight).

Needs world.db with the AzerothCore dump tables (mysql_dump.py) and the Retail
DB2 tables (db2.py). Policy (docs/wiki/systems/content-import.md):

- An AzerothCore item/spell id is kept when the Retail row with the same id has
  the same name (case-insensitive, trimmed), or when scripts/content_overrides.csv
  lists it. Item names come from item_template; Wrath spell names from the Wrath
  Classic SpellName CSV (AzerothCore ships none).
- content_item takes every attribute from Retail item_sparse/item/appearance.
- Class trainers (trainer.Type = 0) teach the Retail class skill line's non-passive
  spells gained by level (skill_line_ability AcquireMethod 2/4); other
  trainers keep AzerothCore spells whose id (and ReqAbility ids) map.
- Quests are kept when their id is in quest_v2; unmapped reward items/spells are
  zeroed. Vendor and loot rows need a mapped item.
- Game object templates are kept when their displayId is 0 (no model) or a Retail
  game_object_display_info row; spawns and addons need a kept template.

Every dropped row or zeroed reward is written to --report. Runs in one transaction.
"""

import argparse
import csv
import sqlite3
import sys
from collections import Counter
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPT_DIR))
import db2  # noqa: E402

PROJECT_DIR = SCRIPT_DIR.parent
WRATH_BUILD = "3.4.0.47659"
DEFAULT_OVERRIDES = SCRIPT_DIR / "content_overrides.csv"
DEFAULT_REPORT = PROJECT_DIR / "data/reports/content-dropped.csv"

REWARD_ITEMS = [(f"RewardItem{n}", f"RewardAmount{n}") for n in range(1, 5)] + [
    (f"RewardChoiceItemID{n}", f"RewardChoiceItemQuantity{n}") for n in range(1, 7)
]
REWARD_SPELLS = ["RewardSpell", "RewardDisplaySpell"]
QUEST_CHILDREN = {
    "quest_template_addon": "ID", "quest_offer_reward": "ID", "quest_request_items": "ID",
    "quest_poi": "QuestID", "quest_poi_points": "QuestID",
    "creature_queststarter": "quest", "creature_questender": "quest",
    "gameobject_queststarter": "quest", "gameobject_questender": "quest",
}
LOOT_TABLES = ["creature_loot_template", "gameobject_loot_template", "reference_loot_template", "item_loot_template"]
# SkillLineAbilityAcquireMethod AutomaticCharLevel / LearnedOrAutomaticCharLevel (TrinityCore DBCEnums.h):
# the spells a character gains by level. Learned (0) rows are talent/helper spells (Hotbar Slot 01,
# Bloodthirst Heal); NeverLearned (3) is excluded as in spell_info/class_data.rs.
ACQUIRE_BY_LEVEL = (2, 4)
ATTR0_PASSIVE = 0x40
ITEM_COLUMNS = """
    s.ID, s.Display_lang AS Name, s.OverallQualityID AS Quality, s.ItemLevel, s.RequiredLevel,
    s.BuyPrice, s.SellPrice, s.VendorStackCount, s.Stackable, s.MaxCount,
    i.ClassID, i.SubclassID, s.InventoryType, s.Bonding, s.ItemDelay, s.DmgVariance,
    s.AllowableClass, s.AllowableRaces_0, s.AllowableRaces_1, s.RequiredSkill, s.RequiredSkillRank,
    s.StartQuestID, s.ItemSet, s.Flags_0, s.Flags_1, s.Flags_2, s.Flags_3, s.Flags_4,
    (SELECT a.ItemDisplayInfoID FROM item_modified_appearance m JOIN item_appearance a ON a.ID = m.ItemAppearanceID
     WHERE m.ItemID = s.ID AND m.ItemAppearanceModifierID = 0 ORDER BY m.OrderIndex LIMIT 1) AS DisplayInfoID,
""" + ",\n".join(f"    s.StatModifier_bonusStat_{n}, s.StatPercentEditor_{n}" for n in range(10))


def normalize(name):
    return (name or "").strip().casefold()


def load_overrides(path, conn):
    """{kind: {source_id: retail_id}}; fails when a reviewed name no longer matches either side."""
    source_name_sql = {"item": "SELECT name FROM item_template WHERE entry = ?"}
    retail_name_sql = {"item": "SELECT Display_lang FROM item_sparse WHERE ID = ?",
                       "spell": "SELECT Name_lang FROM spell_name WHERE ID = ?"}
    overrides = {"item": {}, "spell": {}}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            kind, source_id, retail_id = row["kind"], int(row["source_id"]), int(row["retail_id"])
            if not row["reason"].strip():
                raise SystemExit(f"{path.name}: {kind} {source_id} has no reason")
            retail = conn.execute(retail_name_sql[kind], (retail_id,)).fetchone()
            if retail is None or retail[0] != row["retail_name"]:
                raise SystemExit(f"{path.name}: Retail {kind} {retail_id} is {retail}, reviewed as {row['retail_name']!r}")
            if kind in source_name_sql:
                source = conn.execute(source_name_sql[kind], (source_id,)).fetchone()
                if source is None or source[0] != row["source_name"]:
                    raise SystemExit(f"{path.name}: source {kind} {source_id} is {source}, reviewed as {row['source_name']!r}")
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


def clone_table(conn, source, target):
    """Create empty `target` with `source`'s DDL (keeps column types and primary key)."""
    ddl = conn.execute("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?", (source,)).fetchone()[0]
    conn.execute(f"DROP TABLE IF EXISTS {target}")
    conn.execute(ddl.replace(f"CREATE TABLE {source}", f"CREATE TABLE {target}", 1))


def create_map_table(conn, name, mapping):
    conn.execute(f"DROP TABLE IF EXISTS {name}")
    conn.execute(f"CREATE TEMP TABLE {name} (source_id INTEGER PRIMARY KEY, retail_id INTEGER NOT NULL)")
    conn.executemany(f"INSERT INTO {name} VALUES (?, ?)", mapping.items())


class Report:
    def __init__(self):
        self.rows = []
        self.counts = Counter()

    def drop(self, table, key, source_id, reason):
        self.rows.append((table, key, source_id, reason))
        self.counts[table] += 1

    def write(self, path):
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("w", newline="", encoding="utf-8") as handle:
            writer = csv.writer(handle)
            writer.writerow(["table", "key", "source_id", "reason"])
            writer.writerows(self.rows)


def build_items(conn, item_map, item_reasons, report):
    for source_id, reason in sorted(item_reasons.items()):
        report.drop("item_template", f"entry={source_id}", source_id, reason)
    conn.execute("DROP TABLE IF EXISTS content_item")
    conn.execute(f"""CREATE TABLE content_item AS SELECT {ITEM_COLUMNS}, im.source_id AS SourceEntry
        FROM temp.item_map im JOIN item_sparse s ON s.ID = im.retail_id LEFT JOIN item i ON i.ID = s.ID""")
    conn.execute("CREATE UNIQUE INDEX content_item_id ON content_item(ID)")


def build_vendors(conn, report):
    clone_table(conn, "npc_vendor", "content_npc_vendor")
    conn.execute("""INSERT INTO content_npc_vendor
        SELECT v.entry, v.slot, COALESCE(m.retail_id, v.item), v.maxcount, v.incrtime, v.ExtendedCost, v.VerifiedBuild
        FROM npc_vendor v LEFT JOIN temp.item_map m ON m.source_id = v.item
        WHERE v.item < 0 OR m.retail_id IS NOT NULL""")
    for entry, item in conn.execute("""SELECT entry, item FROM npc_vendor v WHERE item > 0
            AND NOT EXISTS (SELECT 1 FROM temp.item_map m WHERE m.source_id = v.item)"""):
        report.drop("npc_vendor", f"entry={entry}", item, "item not mapped")


def build_trainers(conn, report):
    clone_table(conn, "trainer_spell", "content_trainer_spell")
    conn.execute(f"""INSERT INTO content_trainer_spell
        (TrainerId, SpellId, MoneyCost, ReqSkillLine, ReqSkillRank, ReqAbility1, ReqAbility2, ReqAbility3, ReqLevel, VerifiedBuild)
        SELECT DISTINCT t.Id, a.Spell, 0, 0, 0, 0, 0, 0, COALESCE(l.SpellLevel, 0), 0
        FROM trainer t
        JOIN chr_classes c ON c.ID = t.Requirement
        JOIN skill_line sl ON sl.CategoryID = 7 AND sl.DisplayName_lang = c.Name_lang
        JOIN skill_line_ability a ON a.SkillLine = sl.ID
        JOIN spell_name n ON n.ID = a.Spell
        LEFT JOIN spell_levels l ON l.SpellID = a.Spell AND l.DifficultyID = 0
        LEFT JOIN spell_misc m ON m.SpellID = a.Spell AND m.DifficultyID = 0
        WHERE t.Type = 0 AND (a.ClassMask = 0 OR a.ClassMask & (1 << (c.ID - 1)) != 0)
          AND a.AcquireMethod IN {ACQUIRE_BY_LEVEL} AND COALESCE(m.Attributes_0, 0) & {ATTR0_PASSIVE} = 0""")
    for trainer, spell in conn.execute(
            "SELECT s.TrainerId, s.SpellId FROM trainer_spell s JOIN trainer t ON t.Id = s.TrainerId WHERE t.Type = 0"):
        report.drop("trainer_spell", f"TrainerId={trainer}", spell, "class trainer: Retail class skill line replaces AzerothCore ranks")
    mapped = "(SELECT retail_id FROM temp.spell_map WHERE source_id = s.{0})"
    ability = "CASE WHEN s.{0} = 0 THEN 0 ELSE " + mapped + " END"
    rows = conn.execute(f"""SELECT s.TrainerId, s.SpellId, {mapped.format('SpellId')}, s.MoneyCost, s.ReqSkillLine, s.ReqSkillRank,
            {ability.format('ReqAbility1')}, {ability.format('ReqAbility2')}, {ability.format('ReqAbility3')}, s.ReqLevel, s.VerifiedBuild
        FROM trainer_spell s JOIN trainer t ON t.Id = s.TrainerId WHERE t.Type != 0""").fetchall()
    for trainer, source_spell, spell, cost, skill, rank, req1, req2, req3, level, build in rows:
        if spell is None:
            report.drop("trainer_spell", f"TrainerId={trainer}", source_spell, "spell not mapped")
        elif None in (req1, req2, req3):
            report.drop("trainer_spell", f"TrainerId={trainer}", source_spell, "required ability not mapped")
        else:
            conn.execute("INSERT INTO content_trainer_spell VALUES (?,?,?,?,?,?,?,?,?,?)",
                         (trainer, spell, cost, skill, rank, req1, req2, req3, level, build))


def build_quests(conn, item_map, spell_map, report):
    clone_table(conn, "quest_template", "content_quest_template")
    conn.execute("INSERT INTO content_quest_template SELECT * FROM quest_template WHERE ID IN (SELECT ID FROM quest_v2)")
    for (quest,) in conn.execute("SELECT ID FROM quest_template WHERE ID NOT IN (SELECT ID FROM quest_v2)"):
        report.drop("quest_template", f"ID={quest}", quest, "quest id not in Retail QuestV2")
    columns = [c for pair in REWARD_ITEMS for c in pair] + REWARD_SPELLS
    for row in conn.execute(f"SELECT ID, {', '.join(columns)} FROM content_quest_template").fetchall():
        quest, values = row[0], dict(zip(columns, row[1:]))
        updates = {}
        for item_col, amount_col in REWARD_ITEMS:
            source = values[item_col]
            if source and source not in item_map:
                report.drop("quest_template", f"ID={quest} {item_col}", source, "reward item not mapped; zeroed")
                updates.update({item_col: 0, amount_col: 0})
            elif source:
                updates[item_col] = item_map[source]
        for spell_col in REWARD_SPELLS:
            source = values[spell_col]
            if source > 0 and source not in spell_map:
                report.drop("quest_template", f"ID={quest} {spell_col}", source, "reward spell not mapped; zeroed")
                updates[spell_col] = 0
            elif source > 0:
                updates[spell_col] = spell_map[source]
        changed = {c: v for c, v in updates.items() if v != values[c]}
        if changed:
            conn.execute(f"UPDATE content_quest_template SET {', '.join(f'{c} = ?' for c in changed)} WHERE ID = ?",
                         (*changed.values(), quest))
    for table, key in QUEST_CHILDREN.items():
        clone_table(conn, table, f"content_{table}")
        conn.execute(f"INSERT INTO content_{table} SELECT * FROM {table} WHERE {key} IN (SELECT ID FROM content_quest_template)")
        for (quest,) in conn.execute(f"SELECT {key} FROM {table} WHERE {key} NOT IN (SELECT ID FROM content_quest_template)"):
            report.drop(table, f"{key}={quest}", quest, "quest dropped")


def build_loot(conn, report):
    for table in LOOT_TABLES:
        clone_table(conn, table, f"content_{table}")
        entry = "(SELECT retail_id FROM temp.item_map WHERE source_id = l.Entry)" if table == "item_loot_template" else "l.Entry"
        item = "CASE WHEN l.Reference != 0 THEN l.Item ELSE (SELECT retail_id FROM temp.item_map WHERE source_id = l.Item) END"
        conn.execute(f"""INSERT INTO content_{table}
            SELECT e, i, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount, Comment FROM (
                SELECT {entry} AS e, {item} AS i, l.* FROM {table} l) WHERE e IS NOT NULL AND i IS NOT NULL""")
        for source_entry, source_item, e, i in conn.execute(
                f"SELECT l.Entry, l.Item, {entry}, {item} FROM {table} l").fetchall():
            if e is None:
                report.drop(table, f"Entry={source_entry}", source_entry, "container item not mapped")
            elif i is None:
                report.drop(table, f"Entry={source_entry}", source_item, "item not mapped")


def build_gameobjects(conn, report):
    clone_table(conn, "gameobject_template", "content_gameobject_template")
    conn.execute("""INSERT INTO content_gameobject_template SELECT * FROM gameobject_template t
        WHERE t.displayId = 0 OR t.displayId IN (SELECT ID FROM game_object_display_info)""")
    for entry, display in conn.execute("""SELECT entry, displayId FROM gameobject_template
            WHERE entry NOT IN (SELECT entry FROM content_gameobject_template)"""):
        report.drop("gameobject_template", f"entry={entry}", display, "display not in Retail GameObjectDisplayInfo")
    for table, key in (("gameobject_template_addon", "entry"), ("gameobject", "id")):
        clone_table(conn, table, f"content_{table}")
        kept = f"{key} IN (SELECT entry FROM content_gameobject_template)"
        conn.execute(f"INSERT INTO content_{table} SELECT * FROM {table} WHERE {kept}")
        row_key = "guid" if table == "gameobject" else "entry"
        for row, entry in conn.execute(f"SELECT {row_key}, {key} FROM {table} WHERE NOT ({kept})"):
            report.drop(table, f"{row_key}={row}", entry, "template dropped")


def apply_policy(conn, wrath_spell_names, overrides_path, report):
    retail_items = dict(conn.execute("SELECT ID, Display_lang FROM item_sparse"))
    retail_spells = dict(conn.execute("SELECT ID, Name_lang FROM spell_name"))
    overrides = load_overrides(overrides_path, conn)
    item_map, item_reasons = map_ids(dict(conn.execute("SELECT entry, name FROM item_template")), retail_items, overrides["item"])
    spell_map, _ = map_ids(wrath_spell_names, retail_spells, overrides["spell"])
    create_map_table(conn, "item_map", item_map)
    create_map_table(conn, "spell_map", spell_map)
    build_items(conn, item_map, item_reasons, report)
    build_vendors(conn, report)
    build_trainers(conn, report)
    build_quests(conn, item_map, spell_map, report)
    build_loot(conn, report)
    build_gameobjects(conn, report)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--db", type=Path, default=PROJECT_DIR / "data/world.db")
    parser.add_argument("--csv-root", type=Path, default=db2.DEFAULT_CSV_ROOT)
    parser.add_argument("--overrides", type=Path, default=DEFAULT_OVERRIDES)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    wrath_path = args.csv_root / WRATH_BUILD / "SpellName.csv"
    missing = [str(p) for p in (wrath_path, args.overrides) if not p.is_file()]
    if missing:
        raise SystemExit(f"Missing content policy inputs: {', '.join(missing)}")
    if args.check:
        return
    wrath_spell_names = read_wrath_spell_names(wrath_path)
    report = Report()
    conn = sqlite3.connect(args.db, isolation_level=None)
    try:
        conn.execute("BEGIN")
        apply_policy(conn, wrath_spell_names, args.overrides, report)
        conn.execute("COMMIT")
    except BaseException:
        if conn.in_transaction:
            conn.execute("ROLLBACK")
        raise
    finally:
        conn.close()
    report.write(args.report)
    conn = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    for name in sorted(r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'content_%'")):
        print(f"  {name}: {conn.execute(f'SELECT count(*) FROM {name}').fetchone()[0]}")
    conn.close()
    for table, count in sorted(report.counts.items()):
        print(f"  dropped {table}: {count}")
    print(f"  report: {args.report}")


if __name__ == "__main__":
    main()
