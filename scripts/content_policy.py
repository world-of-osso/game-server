#!/usr/bin/env python3
"""Derive the Retail-first content_* tables the server reads from AzerothCore + Retail DB2 tables.

  content_policy.py [--db data/world.db] [--csv-root DIR] [--overrides CSV] [--report CSV] [--check]

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
  Retail world DB (tdb_creature_template_model) when that entry has the same name
  (or a reviewed creature override) and every one of its displays exists in Retail.
- content_creature: maps 0 and 1 (RETAIL_MAPS) spawn TrinityCore's base-phase
  Retail spawns (PhaseId 0, PhaseGroup 0, terrainSwapMap -1, difficulty 0 in
  spawnDifficulties) at guid + RETAIL_GUID_OFFSET instead of AzerothCore's; other
  maps keep AzerothCore's. content_creature_template adds the TrinityCore template of
  every spawned entry AzerothCore lacks, at the levels of the spawns nearest to its
  own that have an AzerothCore template (TrinityCore levels are ContentTuning
  scaling ranges). Addons, formations, waypoints and event memberships follow the
  spawns' source.

Every dropped row or zeroed reward is written to --report. Runs in one transaction.
"""

import argparse
import bisect
import csv
import sqlite3
import sys
from collections import Counter, defaultdict
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
    source_name_sql = {"item": "SELECT name FROM item_template WHERE entry = ?",
                       "creature": "SELECT name FROM creature_template WHERE entry = ?"}
    retail_name_sql = {"item": "SELECT Display_lang FROM item_sparse WHERE ID = ?",
                       "spell": "SELECT Name_lang FROM spell_name WHERE ID = ?",
                       "creature": "SELECT name FROM tdb_creature_template WHERE entry = ?"}
    overrides = {"item": {}, "spell": {}, "creature": {}}
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


def retail_model_reason(conn, entry, name, reviewed):
    """None when TrinityCore's Retail models for `entry` may replace AzerothCore's, else why not."""
    retail = conn.execute("SELECT name FROM tdb_creature_template WHERE entry = ?", (entry,)).fetchone()
    if retail is None:
        return "no TrinityCore Retail template"
    if not reviewed and normalize(retail[0]) != normalize(name):
        return f"TrinityCore Retail name {retail[0]!r} != {name!r}"
    displays = [d for (d,) in conn.execute(
        "SELECT CreatureDisplayID FROM tdb_creature_template_model WHERE CreatureID = ? ORDER BY Idx", (entry,))]
    if not displays:
        return "no TrinityCore Retail models"
    absent = [d for d in displays if not conn.execute("SELECT 1 FROM creature_display_info WHERE ID = ?", (d,)).fetchone()]
    if absent:
        return f"TrinityCore Retail displays {absent} absent from Retail CreatureDisplayInfo"
    return None


def build_creature_models(conn, overrides, report):
    clone_table(conn, "creature_template_model", "content_creature_template_model")
    conn.execute("INSERT INTO content_creature_template_model SELECT * FROM creature_template_model")
    wrath_only = conn.execute("""SELECT DISTINCT m.CreatureID, t.name FROM creature_template_model m
        LEFT JOIN creature_template t ON t.entry = m.CreatureID
        WHERE m.CreatureDisplayID NOT IN (SELECT ID FROM creature_display_info) ORDER BY m.CreatureID""").fetchall()
    for entry, name in wrath_only:
        retail_entry = overrides.get(entry, entry)
        reason = retail_model_reason(conn, retail_entry, name, entry in overrides)
        if reason:
            report.drop("creature_template_model", f"CreatureID={entry}", entry, f"display absent from Retail kept: {reason}")
            continue
        conn.execute("DELETE FROM content_creature_template_model WHERE CreatureID = ?", (entry,))
        conn.execute("""INSERT INTO content_creature_template_model
            SELECT ?, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild
            FROM tdb_creature_template_model WHERE CreatureID = ?""", (entry, retail_entry))


# --- Retail creature spawns (maps 0 and 1) ---
# The client renders Retail terrain and WMOs, so AzerothCore's Wrath spawn positions put
# interior NPCs in walls or under floors (docs/specs/retail-spawn-source.md).

RETAIL_MAPS = (0, 1)
# TrinityCore guids reach 11.8M and 10 collide with kept AzerothCore spawns
# (maps 229, 230, 530, 609); AzerothCore's reach 5.3M.
RETAIL_GUID_OFFSET = 20_000_000
# A TrinityCore-only template takes the levels of the median of this many nearest
# spawns (per spawn of the entry) that have an AzerothCore template.
LEVEL_NEIGHBOURS = 5
# Only spawns that carry an area's level count: not level-1 critters, non-combat pets or
# invisible triggers, not service NPCs (any npcflag: spirit healers, vendors) and not
# holiday spawns (any game event), whose levels are set per role.
CREATURE_TYPE_CRITTER = 8
CREATURE_TYPE_NON_COMBAT_PET = 12
CREATURE_FLAG_EXTRA_TRIGGER = 0x80

MAP_LIST = ", ".join(str(m) for m in RETAIL_MAPS)
BASE_PHASE = """COALESCE(PhaseId, 0) = 0 AND COALESCE(PhaseGroup, 0) = 0 AND terrainSwapMap = -1
    AND ',' || replace(spawnDifficulties, ' ', '') || ',' LIKE '%,0,%'"""


def not_base_phase_reason(phase_id, phase_group, terrain_swap, difficulties):
    if phase_id:
        return f"not base phase: PhaseId {phase_id}"
    if phase_group:
        return f"not base phase: PhaseGroup {phase_group}"
    if terrain_swap != -1:
        return f"not base phase: terrainSwapMap {terrain_swap}"
    return f"not base phase: spawnDifficulties {difficulties}"


def build_spawns(conn, report):
    """content_creature and temp.retail_spawn (TrinityCore guid -> content guid)."""
    clone_table(conn, "creature", "content_creature")
    conn.execute(f"INSERT INTO content_creature SELECT * FROM creature WHERE map NOT IN ({MAP_LIST})")
    for guid, map_id in conn.execute(f"SELECT guid, map FROM creature WHERE map IN ({MAP_LIST}) ORDER BY guid"):
        report.drop("creature", f"guid={guid}", guid, f"map {map_id} spawn replaced by Retail spawns")
    conn.execute(f"""CREATE TEMP TABLE retail_spawn AS SELECT guid AS tdb_guid, guid + {RETAIL_GUID_OFFSET} AS guid
        FROM tdb_creature WHERE map IN ({MAP_LIST}) AND {BASE_PHASE}""")
    conn.execute("CREATE UNIQUE INDEX temp.retail_spawn_tdb ON retail_spawn(tdb_guid)")
    for guid, *phase in conn.execute(f"""SELECT guid, PhaseId, PhaseGroup, terrainSwapMap, spawnDifficulties
            FROM tdb_creature WHERE map IN ({MAP_LIST}) AND NOT ({BASE_PHASE}) ORDER BY guid"""):
        report.drop("tdb_creature", f"guid={guid}", guid, not_base_phase_reason(*phase))
    conn.execute("""INSERT INTO content_creature (guid, id1, map, zoneId, areaId, equipment_id, position_x, position_y,
            position_z, orientation, spawntimesecs, wander_distance, currentwaypoint, MovementType, ScriptName, VerifiedBuild)
        SELECT r.guid, c.id, c.map, c.zoneId, c.areaId, c.equipment_id, c.position_x, c.position_y, c.position_z,
            c.orientation, c.spawntimesecs, c.wander_distance, c.currentwaypoint, c.MovementType, c.ScriptName, c.VerifiedBuild
        FROM temp.retail_spawn r JOIN tdb_creature c ON c.guid = r.tdb_guid""")


class SpawnGrid:
    """Spawns bucketed into CELL-yard squares per map, for nearest-neighbour lookups."""

    CELL = 200.0

    def __init__(self, points):
        self.cells = defaultdict(list)
        self.per_map = Counter()
        for map_id, x, y, value in points:
            self.cells[(map_id, int(x // self.CELL), int(y // self.CELL))].append((x, y, value))
            self.per_map[map_id] += 1

    def nearest(self, map_id, x, y, count):
        """Values of the `count` nearest points on `map_id` (all of them if the map has fewer)."""
        count = min(count, self.per_map[map_id])
        cx, cy = int(x // self.CELL), int(y // self.CELL)
        found = []
        ring = 0
        # Rings past `ring` are at least ring * CELL yards away.
        while count and (len(found) < count or found[count - 1][0] > (ring * self.CELL) ** 2):
            for dx in range(-ring, ring + 1):
                for dy in ((-ring, ring) if abs(dx) != ring else range(-ring, ring + 1)):
                    for px, py, value in self.cells.get((map_id, cx + dx, cy + dy), ()):
                        bisect.insort(found, ((px - x) ** 2 + (py - y) ** 2, value))
            ring += 1
        return [value for _, value in found[:count]]


def derived_levels(conn):
    """{entry: (minlevel, maxlevel, exp)} for spawned TrinityCore-only entries."""
    known = conn.execute(f"""SELECT c.map, c.position_x, c.position_y, t.minlevel, t.maxlevel, t.exp
        FROM content_creature c JOIN creature_template t ON t.entry = c.id1
        WHERE c.guid IN (SELECT guid FROM temp.retail_spawn)
          AND t.type NOT IN ({CREATURE_TYPE_CRITTER}, {CREATURE_TYPE_NON_COMBAT_PET})
          AND t.flags_extra & {CREATURE_FLAG_EXTRA_TRIGGER} = 0 AND t.npcflag = 0
          AND c.guid - {RETAIL_GUID_OFFSET} NOT IN (SELECT guid FROM tdb_game_event_creature)""").fetchall()
    grid = SpawnGrid((m, x, y, (lo, hi, exp)) for m, x, y, lo, hi, exp in known)
    neighbours = defaultdict(list)
    for entry, map_id, x, y in conn.execute("""SELECT c.id1, c.map, c.position_x, c.position_y FROM content_creature c
            WHERE c.guid IN (SELECT guid FROM temp.retail_spawn)
              AND c.id1 NOT IN (SELECT entry FROM creature_template)"""):
        neighbours[entry].extend(grid.nearest(map_id, x, y, LEVEL_NEIGHBOURS))
    levels = {}
    for entry, values in neighbours.items():
        if values:
            values.sort()
            levels[entry] = values[(len(values) - 1) // 2]
    return levels


def build_templates(conn, report):
    """content_creature_template: AzerothCore's, plus TrinityCore's for spawned entries AzerothCore lacks."""
    clone_table(conn, "creature_template", "content_creature_template")
    conn.execute("INSERT INTO content_creature_template SELECT * FROM creature_template")
    conn.execute("CREATE TEMP TABLE retail_level (entry INTEGER PRIMARY KEY, minlevel INTEGER, maxlevel INTEGER, exp INTEGER)")
    conn.executemany("INSERT INTO temp.retail_level VALUES (?, ?, ?, ?)",
                     ((entry, *values) for entry, values in derived_levels(conn).items()))
    # Loot, pickpocket and skinning ids index TrinityCore's loot tables, which are not imported;
    # gossip menus and vendor lists are AzerothCore-keyed. npcflag's high word is npcflag2.
    conn.execute("""INSERT INTO content_creature_template (entry, KillCredit1, KillCredit2, name, subname, IconName,
            minlevel, maxlevel, exp, faction, npcflag, speed_walk, speed_run, scale, rank, dmgschool, DamageModifier,
            BaseAttackTime, RangeAttackTime, BaseVariance, RangeVariance, unit_class, unit_flags, unit_flags2, family,
            type, type_flags, VehicleId, mingold, maxgold, AIName, MovementType, HealthModifier, ManaModifier,
            ArmorModifier, ExperienceModifier, RacialLeader, movementId, RegenHealth, flags_extra, ScriptName, VerifiedBuild)
        SELECT t.entry, t.KillCredit1, t.KillCredit2, t.name, t.subname, t.IconName,
            l.minlevel, l.maxlevel, l.exp, t.faction, t.npcflag & 4294967295, t.speed_walk, t.speed_run, t.scale,
            t.Classification, t.dmgschool, d.DamageModifier, t.BaseAttackTime, t.RangeAttackTime, t.BaseVariance,
            t.RangeVariance, t.unit_class, t.unit_flags, t.unit_flags2, t.family, t.type, d.TypeFlags, t.VehicleId,
            d.GoldMin, d.GoldMax, t.AIName, t.MovementType, d.HealthModifier, d.ManaModifier, d.ArmorModifier,
            t.ExperienceModifier, t.RacialLeader, t.movementId, t.RegenHealth, t.flags_extra, t.ScriptName, t.VerifiedBuild
        FROM tdb_creature_template t
        JOIN tdb_creature_template_difficulty d ON d.Entry = t.entry AND d.DifficultyID = 0
        JOIN temp.retail_level l ON l.entry = t.entry""")
    retail_only = "SELECT entry FROM content_creature_template WHERE entry NOT IN (SELECT entry FROM creature_template)"
    conn.execute(f"""INSERT INTO content_creature_template_model
        SELECT CreatureID, Idx, CreatureDisplayID, DisplayScale, Probability, VerifiedBuild
        FROM tdb_creature_template_model WHERE CreatureID IN ({retail_only})""")
    for entry, display in conn.execute(f"""SELECT CreatureID, CreatureDisplayID FROM tdb_creature_template_model
            WHERE CreatureID IN ({retail_only})
              AND CreatureDisplayID NOT IN (SELECT ID FROM creature_display_info) ORDER BY CreatureID, Idx""").fetchall():
        report.drop("tdb_creature_template_model", f"CreatureID={entry}", display,
                    "display absent from Retail CreatureDisplayInfo kept")
    for guid, entry in conn.execute("""SELECT guid, id1 FROM content_creature
            WHERE guid IN (SELECT guid FROM temp.retail_spawn)
              AND id1 NOT IN (SELECT entry FROM content_creature_template) ORDER BY guid""").fetchall():
        report.drop("tdb_creature", f"guid={guid - RETAIL_GUID_OFFSET}", entry,
                    "no AzerothCore template and no TrinityCore template with a difficulty 0 row and derivable level")
        conn.execute("DELETE FROM content_creature WHERE guid = ?", (guid,))


def build_addons(conn, report):
    clone_table(conn, "creature_addon", "content_creature_addon")
    conn.execute("INSERT INTO content_creature_addon SELECT * FROM creature_addon WHERE guid IN (SELECT guid FROM content_creature)")
    for (guid,) in conn.execute("SELECT guid FROM creature_addon WHERE guid NOT IN (SELECT guid FROM content_creature)"):
        report.drop("creature_addon", f"guid={guid}", guid, "spawn replaced by Retail spawns")
    # AzerothCore packs UNIT_FIELD_BYTES_1 (stand state, vis flags, anim tier) and BYTES_2 (sheath, PvP flags).
    addon_columns = """PathId, mount, StandState | (VisFlags << 16) | (AnimTier << 24), SheathState | (PvPFlags << 8),
        emote, visibilityDistanceType, auras"""
    conn.execute(f"""INSERT INTO content_creature_addon SELECT r.guid, {addon_columns}
        FROM tdb_creature_addon a JOIN temp.retail_spawn r ON r.tdb_guid = a.guid
        WHERE r.guid IN (SELECT guid FROM content_creature)""")
    clone_table(conn, "creature_template_addon", "content_creature_template_addon")
    conn.execute("INSERT INTO content_creature_template_addon SELECT * FROM creature_template_addon")
    conn.execute(f"""INSERT INTO content_creature_template_addon SELECT a.entry, {addon_columns}
        FROM tdb_creature_template_addon a
        WHERE a.entry IN (SELECT entry FROM content_creature_template)
          AND a.entry NOT IN (SELECT entry FROM creature_template)""")


def build_waypoints(conn):
    """AzerothCore paths plus the TrinityCore paths of Retail addons (per-path move type onto every point)."""
    clone_table(conn, "waypoint_data", "content_waypoint_data")
    conn.execute("INSERT INTO content_waypoint_data SELECT * FROM waypoint_data")
    retail_paths = """SELECT a.path_id FROM content_creature_addon a JOIN temp.retail_spawn r ON r.guid = a.guid
        UNION SELECT path_id FROM content_creature_template_addon WHERE entry NOT IN (SELECT entry FROM creature_template)"""
    clash = conn.execute(f"SELECT DISTINCT id FROM waypoint_data WHERE id IN ({retail_paths}) AND id <> 0").fetchall()
    if clash:
        raise SystemExit(f"TrinityCore waypoint paths collide with AzerothCore waypoint_data ids: {clash[:10]}")
    conn.execute(f"""INSERT INTO content_waypoint_data (id, point, position_x, position_y, position_z, orientation, delay, move_type)
        SELECT n.PathId, n.NodeId, n.PositionX, n.PositionY, n.PositionZ, COALESCE(n.Orientation, 0), n.Delay, p.MoveType
        FROM tdb_waypoint_path_node n JOIN tdb_waypoint_path p ON p.PathId = n.PathId
        WHERE n.PathId IN ({retail_paths})""")


def build_formations_and_events(conn, report):
    clone_table(conn, "creature_formations", "content_creature_formations")
    conn.execute("""INSERT INTO content_creature_formations SELECT * FROM creature_formations
        WHERE memberGUID IN (SELECT guid FROM content_creature)""")
    conn.execute("""INSERT INTO content_creature_formations
        SELECT leader.guid, member.guid, f.dist, f.angle, f.groupAI, f.point_1, f.point_2
        FROM tdb_creature_formations f
        JOIN temp.retail_spawn member ON member.tdb_guid = f.memberGUID
        JOIN temp.retail_spawn leader ON leader.tdb_guid = f.leaderGUID""")
    clone_table(conn, "game_event_creature", "content_game_event_creature")
    conn.execute("""INSERT INTO content_game_event_creature SELECT * FROM game_event_creature
        WHERE guid IN (SELECT guid FROM content_creature)""")
    conn.execute("""INSERT INTO content_game_event_creature SELECT e.eventEntry, r.guid
        FROM tdb_game_event_creature e JOIN temp.retail_spawn r ON r.tdb_guid = e.guid""")
    for table, key in (("creature_formations", "memberGUID"), ("game_event_creature", "guid")):
        for (guid,) in conn.execute(f"SELECT {key} FROM {table} WHERE {key} NOT IN (SELECT guid FROM content_creature)"):
            report.drop(table, f"{key}={guid}", guid, "spawn replaced by Retail spawns")


def build_creatures(conn, report):
    build_spawns(conn, report)
    build_templates(conn, report)
    build_addons(conn, report)
    build_waypoints(conn)
    build_formations_and_events(conn, report)
    conn.execute("CREATE INDEX content_creature_map ON content_creature(map)")


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
    build_quests(conn, item_map, spell_map, report)
    build_loot(conn, report)
    build_gameobjects(conn, report)
    build_creature_models(conn, overrides["creature"], report)
    build_creatures(conn, report)


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
