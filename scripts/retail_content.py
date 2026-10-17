"""Retail world content of RETAIL_MAPS (Eastern Kingdoms, Kalimdor, The Stockade) from TrinityCore TDB1210 (content_policy.py step).

The client renders Retail terrain and WMOs, so AzerothCore's Wrath content puts interior
NPCs in walls or under floors and misses Cataclysm-and-later NPCs. For RETAIL_MAPS this
module replaces AzerothCore's creature and game object spawns with TrinityCore's
base-phase Retail spawns, and resolves their quests, vendors, gossip and loot from the
tdb_* tables (imported verbatim by mysql_dump.py --prefix tdb_). AzerothCore rows stay
where TrinityCore has none for the same NPC or quest (measured gaps: quest starters,
loot of 143 NPCs). Every TrinityCore row is mapped into the AzerothCore-shaped content_*
table the server already reads. docs/specs/retail-world-content.md is the contract.
"""

from content_common import clone_table

RETAIL_MAPS = (0, 1, 34, 670, 2106, 2107, 761, 726, 566, 998, 727)
# TrinityCore's DEFAULT_PHASE 169 is unphased (PhaseShift.h:31, PhasingHandler::InitDbPhaseShift):
# spawns in it are seen by everyone, like PhaseId 0.
DEFAULT_PHASE = 169
# Map.db2 InstanceType of open world (0, spawns at DIFFICULTY_NONE) and party dungeons (1, MAP_INSTANCE: a copy
# spawns at its difficulty, one of the map's MapDifficulty.db2 rows).
INSTANCE_TYPE_WORLD, INSTANCE_TYPE_PARTY, INSTANCE_TYPE_BATTLEGROUND = 0, 1, 3
# TrinityCore guids reach 11.8M and 10 collide with kept AzerothCore spawns
# (maps 229, 230, 530, 609); AzerothCore's reach 5.3M.
RETAIL_GUID_OFFSET = 20_000_000
# TrinityCore DB2Manager::GetContentTuningData with worldserver.conf's default Expansion 11
# (Midnight): ContentTuningCalcType None/MinLevel/MaxLevel/PrevExpansionMaxLevel adds 0, 1,
# GetMaxLevelForExpansion(11) = 90 or GetMaxLevelForExpansion(10) = 80 (SharedDefines.h);
# levels clamp to 1..MAX_LEVEL 123 (DBCEnums.h).
CALC_TYPE_ADJUSTMENT = {0: 0, 1: 1, 2: 90, 3: 80}
MAX_LEVEL = 123
# creature_classlevelstats has basehp0..2 (Classic, TBC, Wrath); TrinityCore's
# HealthScalingExpansion -1 is the current expansion.
WRATH_HEALTH_EXPANSIONS = 2
# TDB1210 Seabraid service, NPCFlags2 bit19; retain only this implemented high flag.
TRADING_POST_NPC_FLAG = 0x0008_0000_0000_0000

MAP_LIST = ", ".join(str(m) for m in RETAIL_MAPS)


# `map` and `spawnDifficulties` of the queried spawn table (unqualified: the only table with them), `r` a
# temp.retail_map row (create_retail_map_table).
SPAWN_DIFFICULTY_MATCH = "',' || replace(spawnDifficulties, ' ', '') || ',' LIKE '%,' || r.difficulty || ',%'"
SPAWNS_ON_MAP = f"EXISTS (SELECT 1 FROM temp.retail_map r WHERE r.map_id = map AND {SPAWN_DIFFICULTY_MATCH})"
BASE_PHASE = f"""(COALESCE(PhaseId, 0) IN (0, {DEFAULT_PHASE}) AND COALESCE(PhaseGroup, 0) = 0 AND terrainSwapMap = -1
    AND {SPAWNS_ON_MAP})"""
# The spawn's difficulties the map offers, ascending ("0" on a continent): content_*.spawnDifficulties.
SPAWN_DIFFICULTIES = f"""(SELECT group_concat(difficulty, ',') FROM (SELECT r.difficulty FROM temp.retail_map r
    WHERE r.map_id = map AND {SPAWN_DIFFICULTY_MATCH} ORDER BY r.difficulty))"""


def create_retail_map_table(conn):
    """temp.retail_map: each (RETAIL_MAPS map, difficulty its copies spawn at): 0 for open world, every
    MapDifficulty.db2 DifficultyID of a party dungeon; battleground authored spawns use 0."""
    rows = dict(conn.execute(f"SELECT ID, InstanceType FROM map WHERE ID IN ({MAP_LIST})"))
    missing = [m for m in RETAIL_MAPS if m not in rows]
    unhosted = {m: t for m, t in rows.items() if t not in (INSTANCE_TYPE_WORLD, INSTANCE_TYPE_PARTY, INSTANCE_TYPE_BATTLEGROUND)}
    if missing or unhosted:
        raise SystemExit(f"RETAIL_MAPS without a Map.db2 row {missing} or with an unhosted InstanceType {unhosted}")
    conn.execute("DROP TABLE IF EXISTS temp.retail_map")
    conn.execute("""CREATE TEMP TABLE retail_map (map_id INTEGER NOT NULL, difficulty INTEGER NOT NULL,
        PRIMARY KEY (map_id, difficulty))""")
    for map_id, instance_type in sorted(rows.items()):
        if instance_type in (INSTANCE_TYPE_WORLD, INSTANCE_TYPE_BATTLEGROUND):
            difficulties = [0]
        else:
            difficulties = [d for (d,) in conn.execute(
                "SELECT DISTINCT DifficultyID FROM map_difficulty WHERE MapID = ? ORDER BY DifficultyID", (map_id,))]
            if not difficulties:
                raise SystemExit(f"dungeon map {map_id} has no MapDifficulty.db2 row")
        conn.executemany("INSERT INTO temp.retail_map VALUES (?, ?)", ((map_id, d) for d in difficulties))


def retail_spawned_entries(conn):
    """Creature entries a Retail (base-phase, RETAIL_MAPS) TrinityCore spawn references."""
    return {entry for (entry,) in conn.execute(
        f"SELECT DISTINCT id FROM tdb_creature WHERE map IN ({MAP_LIST}) AND {BASE_PHASE}")}


def not_base_phase_reason(phase_id, phase_group, terrain_swap, difficulties):
    if (phase_id or 0) not in (0, DEFAULT_PHASE):
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
    # TrinityCore `creature.modelid`: the spawn's display instead of its template's (ObjectMgr::ChooseDisplayId).
    conn.execute("ALTER TABLE content_creature ADD COLUMN modelid INTEGER NOT NULL DEFAULT 0")
    # TrinityCore `creature.spawnDifficulties` (the map's): the copy difficulties it spawns in.
    conn.execute("ALTER TABLE content_creature ADD COLUMN spawnDifficulties TEXT NOT NULL DEFAULT '0'")
    for guid, map_id in conn.execute(f"SELECT guid, map FROM creature WHERE map IN ({MAP_LIST}) ORDER BY guid"):
        report.drop("creature", f"guid={guid}", guid, f"map {map_id} spawn replaced by Retail spawns")
    conn.execute(f"""CREATE TEMP TABLE retail_spawn AS SELECT guid AS tdb_guid, guid + {RETAIL_GUID_OFFSET} AS guid
        FROM tdb_creature WHERE map IN ({MAP_LIST}) AND {BASE_PHASE}""")
    conn.execute("CREATE UNIQUE INDEX temp.retail_spawn_tdb ON retail_spawn(tdb_guid)")
    for guid, *phase in conn.execute(f"""SELECT guid, PhaseId, PhaseGroup, terrainSwapMap, spawnDifficulties
            FROM tdb_creature WHERE map IN ({MAP_LIST}) AND NOT ({BASE_PHASE}) ORDER BY guid"""):
        report.drop("tdb_creature", f"guid={guid}", guid, not_base_phase_reason(*phase))
    # unit_flags NULL (the template's) becomes 0, which the server also reads as "the template's"; no TDB1210
    # spawn overrides it with 0.
    conn.execute(f"""INSERT INTO content_creature (guid, id1, map, zoneId, areaId, equipment_id, position_x, position_y,
            position_z, orientation, spawntimesecs, wander_distance, currentwaypoint, MovementType, unit_flags, ScriptName,
            VerifiedBuild, modelid, spawnDifficulties)
        SELECT r.guid, c.id, c.map, c.zoneId, c.areaId, c.equipment_id, c.position_x, c.position_y, c.position_z,
            c.orientation, c.spawntimesecs, c.wander_distance, c.currentwaypoint, c.MovementType, COALESCE(c.unit_flags, 0),
            c.ScriptName, c.VerifiedBuild, c.modelid, {SPAWN_DIFFICULTIES}
        FROM temp.retail_spawn r JOIN tdb_creature c ON c.guid = r.tdb_guid""")
    # ObjectMgr::LoadCreatures ignores a modelid without creature_model_info.
    for guid, display in conn.execute("""SELECT guid, modelid FROM content_creature
            WHERE modelid <> 0 AND modelid NOT IN (SELECT ID FROM creature_display_info) ORDER BY guid""").fetchall():
        report.drop("tdb_creature", f"guid={guid - RETAIL_GUID_OFFSET}", display,
                    "modelid absent from Retail CreatureDisplayInfo; the template's models apply")
        conn.execute("UPDATE content_creature SET modelid = 0 WHERE guid = ?", (guid,))


def content_tuning_levels(min_level, max_level, min_type, max_type):
    """(min, max) creature level range of a ContentTuning row (TrinityCore GetContentTuningData)."""
    def level(base, calc_type):
        return min(max(base + CALC_TYPE_ADJUSTMENT[calc_type], 1), MAX_LEVEL)
    return level(min_level, min_type), level(max_level, max_type)


def health_expansion(health_scaling_expansion):
    """AzerothCore creature_template.exp (basehp column) for a TrinityCore HealthScalingExpansion."""
    if health_scaling_expansion < 0:
        return WRATH_HEALTH_EXPANSIONS
    return min(health_scaling_expansion, WRATH_HEALTH_EXPANSIONS)


def quest_min_level(min_level, min_type, target_level_delta):
    """GetContentTuningData's MinLevelWithDelta: clamp(MinLevel + TargetLevelDelta, 1, MAX_LEVEL),
    before MinLevel itself is clamped (DB2Stores.cpp:2233-2235)."""
    return min(max(min_level + CALC_TYPE_ADJUSTMENT[min_type] + target_level_delta, 1), MAX_LEVEL)


# ContentTuningFlag Horde 0x08 / Alliance 0x10 (DBCEnums.h:452-457) give
# ContentTuningEntry::GetScalingFactionGroup 5 / 3 (DB2Structure.h:1008-1018).
def scaling_faction_group(flags):
    if flags & 0x08:
        return 5
    if flags & 0x10:
        return 3
    return 0


def create_level_table(conn):
    """content_tuning_level: GetContentTuningData levels of every ContentTuning row, read by
    this import (creature and quest levels) and by the server (Player::GetQuestLevel)."""
    conn.execute("DROP TABLE IF EXISTS content_tuning_level")
    conn.execute("""CREATE TABLE content_tuning_level (ID INTEGER PRIMARY KEY, minlevel INTEGER, maxlevel INTEGER,
        MinLevelWithDelta INTEGER, ScalingFactionGroup INTEGER)""")
    conn.executemany("INSERT INTO content_tuning_level VALUES (?, ?, ?, ?, ?)", (
        (tuning, *content_tuning_levels(low, high, low_type, high_type), quest_min_level(low, low_type, delta),
         scaling_faction_group(flags))
        for tuning, low, high, low_type, high_type, delta, flags in conn.execute(
            """SELECT ID, MinLevelSquish, MaxLevelSquish, MinLevelScalingOffset, MaxLevelScalingOffset,
                AllowedMinOffset, Flags FROM content_tuning""")))


def difficulty_fallbacks(conn):
    """Difficulty.db2 FallbackDifficultyID by ID (Mythic 23 -> Heroic 2 -> Normal 1 -> None 0)."""
    return dict(conn.execute("SELECT ID, FallbackDifficultyID FROM difficulty"))


def resolve_difficulty_rows(conn, spawned):
    """{(entry, difficulty): creature_template_difficulty DifficultyID} for each spawned (entry, difficulty):
    CreatureTemplate::GetDifficulty (Creature.cpp:252-262) takes the row of the difficulty, else of its
    FallbackDifficultyID, down to DifficultyID 0; pairs with no row on that chain are left out."""
    fallbacks = difficulty_fallbacks(conn)
    rows = {}
    for entry, difficulty in conn.execute("SELECT Entry, DifficultyID FROM tdb_creature_template_difficulty"):
        rows.setdefault(entry, set()).add(difficulty)
    resolved = {}
    for entry, difficulty in spawned:
        current, seen = difficulty, set()
        while current not in rows.get(entry, ()):
            if current == 0 or current in seen:
                current = None
                break
            seen.add(current)
            current = fallbacks.get(current, 0)
        if current is not None:
            resolved[(entry, difficulty)] = current
    return resolved


def create_difficulty_table(conn, *, map_list=MAP_LIST, base_phase=BASE_PHASE):
    """temp.spawn_map_difficulty: each (entry, difficulty) a Retail spawn of it spawns at.
    temp.spawn_row: the creature_template_difficulty row each of those uses (resolve_difficulty_rows).
    temp.tdb_difficulty: the row of the entry's lowest spawn difficulty, or DifficultyID 0 for entries spawned
    nowhere, the entry's content_creature_template values."""
    conn.execute(f"""CREATE TEMP TABLE spawn_map_difficulty AS SELECT DISTINCT c.id AS entry, r.difficulty
        FROM tdb_creature c JOIN temp.retail_map r ON r.map_id = c.map
        WHERE c.map IN ({map_list}) AND {base_phase}
          AND {SPAWN_DIFFICULTY_MATCH}""")
    spawned = conn.execute("SELECT entry, difficulty FROM temp.spawn_map_difficulty").fetchall()
    resolved = resolve_difficulty_rows(conn, spawned)
    conn.execute("CREATE TEMP TABLE spawn_row (entry INTEGER, difficulty INTEGER, row_difficulty INTEGER, "
                 "PRIMARY KEY (entry, difficulty))")
    conn.executemany("INSERT INTO temp.spawn_row VALUES (?, ?, ?)",
                     ((entry, difficulty, row) for (entry, difficulty), row in sorted(resolved.items())))
    conn.execute("""CREATE TEMP TABLE tdb_difficulty AS SELECT d.* FROM tdb_creature_template_difficulty d
        WHERE d.DifficultyID = COALESCE((SELECT s.row_difficulty FROM temp.spawn_row s WHERE s.entry = d.Entry
            ORDER BY s.difficulty LIMIT 1), 0)""")
    conn.execute("CREATE UNIQUE INDEX temp.tdb_difficulty_entry ON tdb_difficulty(Entry)")


def build_template_difficulties(conn, report):
    """content_creature_template_difficulty: for each dungeon difficulty a Retail entry spawns at, the template
    values of its creature_template_difficulty row (ContentTuning levels, scaling, modifiers; loot and money come
    with build_loot). The server uses it for creatures of a copy at that difficulty."""
    conn.execute("DROP TABLE IF EXISTS content_creature_template_difficulty")
    conn.execute("""CREATE TABLE content_creature_template_difficulty (entry INTEGER NOT NULL,
        DifficultyID INTEGER NOT NULL, minlevel INTEGER NOT NULL, maxlevel INTEGER NOT NULL, exp INTEGER NOT NULL,
        ContentTuningID INTEGER NOT NULL, HealthScalingExpansion INTEGER NOT NULL, LevelScalingDeltaMin INTEGER NOT NULL,
        LevelScalingDeltaMax INTEGER NOT NULL, HealthModifier REAL NOT NULL, ManaModifier REAL NOT NULL,
        ArmorModifier REAL NOT NULL, DamageModifier REAL NOT NULL, lootid INTEGER NOT NULL DEFAULT 0,
        mingold INTEGER NOT NULL DEFAULT 0, maxgold INTEGER NOT NULL DEFAULT 0, PRIMARY KEY (entry, DifficultyID))""")
    conn.execute("""INSERT INTO content_creature_template_difficulty (entry, DifficultyID, minlevel, maxlevel, exp,
            ContentTuningID, HealthScalingExpansion, LevelScalingDeltaMin, LevelScalingDeltaMax, HealthModifier,
            ManaModifier, ArmorModifier, DamageModifier)
        SELECT s.entry, s.difficulty, COALESCE(l.minlevel, 1), COALESCE(l.maxlevel, 1),
            health_expansion(d.HealthScalingExpansion), d.ContentTuningID, d.HealthScalingExpansion,
            d.LevelScalingDeltaMin, d.LevelScalingDeltaMax, d.HealthModifier, d.ManaModifier, d.ArmorModifier,
            d.DamageModifier
        FROM temp.spawn_row s
        JOIN tdb_creature_template_difficulty d ON d.Entry = s.entry AND d.DifficultyID = s.row_difficulty
        LEFT JOIN content_tuning_level l ON l.ID = d.ContentTuningID
        WHERE s.difficulty > 0 AND s.entry IN (SELECT entry FROM content_creature_template)""")


# Creature entries spells name in a TRANSFORM aura's MiscValue (SPELL_AURA_TRANSFORM 56): the
# server shows a model of that template (AuraEffect::HandleAuraTransform). They need a template
# though nothing spawns them.
TRANSFORM_ENTRIES = "SELECT EffectMiscValue_0 FROM spell_effect WHERE EffectAura = 56 AND EffectMiscValue_0 > 0"

# Aura428 linked toys and audited stationary scenery need authentic templates
# even without world spawns. TDB1210 incense, harmony totem, Artisan's Sign
# stationary banners, Pilfered Sweeper and Brewfest Chowdown Trophy have no
# click/script/control bindings in the local TDB1210 archive.
TOY_SCENERY_SUMMON_ENTRIES = (
    65298, 65387, 144025, 144026, 181826, 182459, 182461, 193182,
    201798, 208477, 214220, 220620, 233144, 233145, 233146, 233148,
    233149, 233150, 121762, 183727,
    # Source-defined stationary scarecrow, light globe, vesper and arena banners.
    # No interaction/control/AI contract: docs/toy-effects14-coverage.md.
    66768, 114846, 175582, 263908, 270689,
    # Source-defined scaled scenery, with independent display/object scale.
    156086, 241494, 246171, 267634,
    # Source-complete stationary warning signs, without interactions.
    73781, 88280, 156052, 218978,
    # Placed Ornate Dragon Statue and Stuffed Bear statue: item text is placement
    # only; no TDB AI/script/spell/click/npcflag binding. Attraction Sign90335 is
    # excluded: its arrow display41343 has TDB probability0 (invisible11686 spawns).
    194793, 203158,
    # Source group-private summons; preserve TDB identity/models without inventing
    # click scripts or interaction payloads. See docs/toy-effects14-coverage.md.
    80682, 125119, 99535, 116213, 125120, 139198, 139204, 143703,
    149954, 163441, 199054, 231372,
    # Sea-Blessed Shrine: source cursor placement, no click/AI/script payload.
    246037,
    # Xan'tish's Flute: preserve all four weighted native Snake display choices.
    # Source wild/default count is separate from template publication.
    71500,
    # Enchanted Stone Whistle: source Petrified Sand Piper, no AI/click payload.
    98300,
    # Ysera's Tear: source private vision; procedural display rendering is separate.
    99518,
    # Katy's Stampwhistle: preserve source Assistant35 and mailbox NPC flags.
    # Service and summon-category acceptance remain separate runtime gates.
    132969,
    # Scarab Storm: source guardian routing/count remains a separate runtime boundary.
    51113,
    # Spirit of Bashiok: one stationary source actor; native18/78 placement is separate.
    86916,
    # Winner's Podium: source47/138 placement, no AI/click/control bindings.
    240220,
)
TOY_BANNER_ITEM_IDS = (
    163986, 163987, 187834, 187957, 187958, 203716, 208057, 211424,
    218128, 232301, 232302, 232303, 232305, 232306, 232307,
)
TOY_SUMMON_ENTRIES = f"""SELECT s.EffectMiscValue_0 FROM toy t
    JOIN item_x_item_effect x ON x.ItemID = t.ItemID
    JOIN item_effect i ON i.ID = x.ItemEffectID AND i.TriggerType = 0
    JOIN spell_effect a ON a.SpellID = i.SpellID AND a.DifficultyID = 0 AND a.EffectAura = 428
    JOIN spell_effect s ON s.SpellID = a.EffectTriggerSpell AND s.DifficultyID = 0 AND s.Effect = 28
    WHERE s.EffectMiscValue_0 > 0
    UNION SELECT entry FROM tdb_creature_template WHERE entry IN {TOY_SCENERY_SUMMON_ENTRIES}"""


CLASS_SUMMON_ENTRIES = """WITH RECURSIVE class_line AS (
    SELECT l.ID, c.ID AS class_id FROM skill_line l JOIN chr_classes c
      ON c.Name_lang=l.DisplayName_lang WHERE l.CategoryID=7 AND c.ID BETWEEN 1 AND 13
), roots(id) AS (
    SELECT a.Spell FROM skill_line_ability a JOIN class_line l ON l.ID=a.SkillLine
      WHERE a.AcquireMethod<>3 AND (a.ClassMask=0 OR (a.ClassMask & (1 << (l.class_id-1)))<>0)
    UNION SELECT s.SpellID FROM specialization_spells s JOIN chr_specialization c ON c.ID=s.SpecID
      WHERE c.ClassID BETWEEN 1 AND 13 AND c.OrderIndex<>4
    UNION SELECT MasterySpellID_0 FROM chr_specialization WHERE ClassID BETWEEN 1 AND 13 AND OrderIndex<>4
    UNION SELECT MasterySpellID_1 FROM chr_specialization WHERE ClassID BETWEEN 1 AND 13 AND OrderIndex<>4
    UNION SELECT d.SpellID FROM trait_definition d
      JOIN trait_node_entry e ON e.TraitDefinitionID=d.ID
      JOIN trait_node_x_trait_node_entry x ON x.TraitNodeEntryID=e.ID
      JOIN trait_node n ON n.ID=x.TraitNodeID
      JOIN skill_line_x_trait_tree t ON t.TraitTreeID=n.TraitTreeID
      JOIN class_line l ON l.ID=t.SkillLineID
), reachable(id) AS (
    SELECT id FROM roots WHERE id>0
    UNION SELECT e.EffectTriggerSpell FROM spell_effect e JOIN reachable r ON r.id=e.SpellID
      WHERE e.DifficultyID=0 AND e.EffectTriggerSpell>0
)
SELECT e.EffectMiscValue_0 FROM spell_effect e JOIN reachable r ON r.id=e.SpellID
WHERE e.DifficultyID=0 AND e.Effect IN (28,56) AND e.EffectMiscValue_0>0"""


def insert_tdb_templates(conn, entries, *, replace_legacy=False):
    """TrinityCore's templates (difficulty 0 values, ContentTuning levels) of the `entries` query AzerothCore lacks
    and content_creature_template does not hold yet. Needs temp.tdb_difficulty, content_tuning_level and
    health_expansion."""
    # Instance imports replace same-ID legacy templates with their authentic Retail identity.
    legacy_filter = "" if replace_legacy else "AND t.entry NOT IN (SELECT entry FROM creature_template)"
    # npcflag's high word is npcflag2. Levels are the ContentTuning range (level 1 without one).
    conn.execute(f"""INSERT INTO content_creature_template (entry, KillCredit1, KillCredit2, name, subname, IconName,
            minlevel, maxlevel, exp, faction, npcflag, speed_walk, speed_run, scale, rank, dmgschool, DamageModifier,
            BaseAttackTime, RangeAttackTime, BaseVariance, RangeVariance, unit_class, unit_flags, unit_flags2, family,
            type, type_flags, VehicleId, mingold, maxgold, AIName, MovementType, HealthModifier, ManaModifier,
            ArmorModifier, ExperienceModifier, RacialLeader, movementId, RegenHealth, flags_extra, ScriptName, VerifiedBuild,
            ContentTuningID, HealthScalingExpansion, LevelScalingDeltaMin, LevelScalingDeltaMax)
        SELECT t.entry, t.KillCredit1, t.KillCredit2, t.name, t.subname, t.IconName,
            COALESCE(l.minlevel, 1), COALESCE(l.maxlevel, 1), health_expansion(d.HealthScalingExpansion), t.faction,
            t.npcflag & (4294967295 | {TRADING_POST_NPC_FLAG}), t.speed_walk, t.speed_run, t.scale,
            t.Classification, t.dmgschool, d.DamageModifier, t.BaseAttackTime, t.RangeAttackTime, t.BaseVariance,
            t.RangeVariance, t.unit_class, t.unit_flags, t.unit_flags2, t.family, t.type, d.TypeFlags, t.VehicleId,
            d.GoldMin, d.GoldMax, t.AIName, t.MovementType, d.HealthModifier, d.ManaModifier, d.ArmorModifier,
            t.ExperienceModifier, t.RacialLeader, t.movementId, t.RegenHealth, t.flags_extra, t.ScriptName, t.VerifiedBuild,
            d.ContentTuningID, d.HealthScalingExpansion, d.LevelScalingDeltaMin, d.LevelScalingDeltaMax
        FROM tdb_creature_template t
        JOIN temp.tdb_difficulty d ON d.Entry = t.entry
        LEFT JOIN content_tuning_level l ON l.ID = d.ContentTuningID
        WHERE t.entry IN ({entries})
          {legacy_filter}
          AND t.entry NOT IN (SELECT entry FROM content_creature_template)""")


def take_retail_taming_columns(conn):
    """Taming reads Retail's creature type, family and difficulty TypeFlags (CreatureTemplate::IsTameable,
    CreatureData.h): an AzerothCore template TrinityCore also has takes them (Old Blanchy 582 is a tameable
    Beast in Retail, a Critter in AzerothCore). Needs temp.tdb_difficulty."""
    conn.execute("""UPDATE content_creature_template SET type = t.type, family = t.family, type_flags = d.TypeFlags
        FROM tdb_creature_template t JOIN temp.tdb_difficulty d ON d.Entry = t.entry
        WHERE t.entry = content_creature_template.entry
          AND content_creature_template.entry IN (SELECT entry FROM creature_template)""")


def build_templates(conn, report):
    """content_creature_template: AzerothCore's, plus TrinityCore's for spawned entries AzerothCore lacks and for
    entries spells transform into or class/spec/talent spell closures summon. No world spawns are added."""
    clone_table(conn, "creature_template", "content_creature_template")
    conn.execute("INSERT INTO content_creature_template SELECT * FROM creature_template")
    for column in ("ContentTuningID", "HealthScalingExpansion", "LevelScalingDeltaMin", "LevelScalingDeltaMax"):
        conn.execute(f"ALTER TABLE content_creature_template ADD COLUMN {column} INTEGER NOT NULL DEFAULT 0")
    create_level_table(conn)
    create_difficulty_table(conn)
    conn.create_function("health_expansion", 1, health_expansion, deterministic=True)
    insert_tdb_templates(conn, "SELECT id1 FROM content_creature WHERE guid IN (SELECT guid FROM temp.retail_spawn)")
    insert_tdb_templates(conn, TRANSFORM_ENTRIES)
    insert_tdb_templates(conn, TOY_SUMMON_ENTRIES)
    insert_tdb_templates(conn, CLASS_SUMMON_ENTRIES)
    # Retail quest objectives count Retail kill credits (Player::KilledMonster credits the entry
    # and its KillCredit1/2): an AzerothCore template TrinityCore also has takes them.
    conn.execute("""UPDATE content_creature_template SET KillCredit1 = t.KillCredit1, KillCredit2 = t.KillCredit2
        FROM tdb_creature_template t
        WHERE t.entry = content_creature_template.entry
          AND content_creature_template.entry IN (SELECT entry FROM creature_template)""")
    take_retail_taming_columns(conn)
    # AzerothCore templates TrinityCore also tunes take its ContentTuning range, and the
    # difficulty-0 scaling expansion, level delta and modifiers ExpectedStat is scaled by.
    conn.execute("""UPDATE content_creature_template SET ContentTuningID = d.ContentTuningID,
            minlevel = l.minlevel, maxlevel = l.maxlevel, HealthScalingExpansion = d.HealthScalingExpansion,
            LevelScalingDeltaMin = d.LevelScalingDeltaMin, LevelScalingDeltaMax = d.LevelScalingDeltaMax,
            HealthModifier = d.HealthModifier, ArmorModifier = d.ArmorModifier, DamageModifier = d.DamageModifier
        FROM temp.tdb_difficulty d JOIN content_tuning_level l ON l.ID = d.ContentTuningID
        WHERE d.Entry = content_creature_template.entry
          AND content_creature_template.entry IN (SELECT entry FROM creature_template)""")
    for entry, tuning in conn.execute("""SELECT t.entry, d.ContentTuningID FROM content_creature_template t
            JOIN temp.tdb_difficulty d ON d.Entry = t.entry
            WHERE d.ContentTuningID NOT IN (SELECT ID FROM content_tuning_level) ORDER BY t.entry""").fetchall():
        fixed = "level 1" if not conn.execute("SELECT 1 FROM creature_template WHERE entry = ?", (entry,)).fetchone() \
            else "AzerothCore levels kept"
        report.drop("tdb_creature_template_difficulty", f"Entry={entry}", tuning, f"ContentTuningID not in ContentTuning; {fixed}")
    build_template_difficulties(conn, report)
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
                    "no AzerothCore template and no TrinityCore template with a difficulty 0 row")
        conn.execute("DELETE FROM content_creature WHERE guid = ?", (guid,))


# TrinityCore addon columns as AzerothCore packs them: UNIT_FIELD_BYTES_1 (stand state, vis flags, anim tier)
# and BYTES_2 (sheath, PvP flags).
ADDON_COLUMNS = """PathId, mount, StandState | (VisFlags << 16) | (AnimTier << 24), SheathState | (PvPFlags << 8),
        emote, visibilityDistanceType, auras"""


def build_addons(conn, report):
    clone_table(conn, "creature_addon", "content_creature_addon")
    conn.execute("INSERT INTO content_creature_addon SELECT * FROM creature_addon WHERE guid IN (SELECT guid FROM content_creature)")
    for (guid,) in conn.execute("SELECT guid FROM creature_addon WHERE guid NOT IN (SELECT guid FROM content_creature)"):
        report.drop("creature_addon", f"guid={guid}", guid, "spawn replaced by Retail spawns")
    conn.execute(f"""INSERT INTO content_creature_addon SELECT r.guid, {ADDON_COLUMNS}
        FROM tdb_creature_addon a JOIN temp.retail_spawn r ON r.tdb_guid = a.guid
        WHERE r.guid IN (SELECT guid FROM content_creature)""")
    clone_table(conn, "creature_template_addon", "content_creature_template_addon")
    conn.execute("INSERT INTO content_creature_template_addon SELECT * FROM creature_template_addon")
    conn.execute(f"""INSERT INTO content_creature_template_addon SELECT a.entry, {ADDON_COLUMNS}
        FROM tdb_creature_template_addon a
        WHERE a.entry IN (SELECT entry FROM content_creature_template)
          AND a.entry NOT IN (SELECT entry FROM creature_template)""")


def build_equipment(conn):
    """creature_equip_template of the entry's spawn source: TrinityCore's (with AppearanceModIDs) for Retail-spawned
    entries, AzerothCore's for the rest; creature.equipment_id picks the row (Creature::LoadEquipment)."""
    conn.execute("DROP TABLE IF EXISTS content_creature_equip_template")
    conn.execute("""CREATE TABLE content_creature_equip_template (CreatureID INTEGER NOT NULL, ID INTEGER NOT NULL,
        ItemID1 INTEGER NOT NULL, AppearanceModID1 INTEGER NOT NULL, ItemID2 INTEGER NOT NULL,
        AppearanceModID2 INTEGER NOT NULL, ItemID3 INTEGER NOT NULL, AppearanceModID3 INTEGER NOT NULL,
        PRIMARY KEY (CreatureID, ID))""")
    conn.execute("""INSERT INTO content_creature_equip_template SELECT CreatureID, ID, ItemID1, AppearanceModID1,
        ItemID2, AppearanceModID2, ItemID3, AppearanceModID3 FROM tdb_creature_equip_template
        WHERE CreatureID IN (SELECT entry FROM temp.retail_entry)""")
    conn.execute("""INSERT INTO content_creature_equip_template SELECT CreatureID, ID, ItemID1, 0, ItemID2, 0, ItemID3, 0
        FROM creature_equip_template WHERE CreatureID NOT IN (SELECT entry FROM temp.retail_entry)""")


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
    conn.execute("""CREATE TEMP TABLE retail_entry AS SELECT DISTINCT id1 AS entry FROM content_creature
        WHERE guid IN (SELECT guid FROM temp.retail_spawn)""")
    conn.execute("CREATE UNIQUE INDEX temp.retail_entry_entry ON retail_entry(entry)")
    build_equipment(conn)


# --- Items, vendors and loot ---
# TrinityCore's item ids are Retail ids already; its loot ids live beside AzerothCore's.

# TrinityCore loot template and reference ids (loot tables reach 700,018, AzerothCore's 1.6M).
TDB_LOOT_OFFSET = 20_000_000
# TrinityCore ItemVendorType and LootStoreItem::Type.
VENDOR_ITEM, VENDOR_CURRENCY = 1, 2
LOOT_ITEM, LOOT_REFERENCE, LOOT_CURRENCY = 0, 1, 2
TDB_LOOT_TABLES = ("creature_loot_template", "gameobject_loot_template", "reference_loot_template", "item_loot_template")


def retail_item_ids(conn):
    """Every installed Item/ItemSparse definition, independent of older TDB references.

    TrinityCore a352b1fa ObjectMgr.cpp:2991-3000 walks the ItemSparse store and requires
    its matching Item entry. Content references decide availability, not definitions.
    """
    return {item for (item,) in conn.execute(
        "SELECT s.ID FROM item_sparse s JOIN item i ON i.ID = s.ID")}


def build_vendors(conn, report):
    """TrinityCore's vendor list for every Retail-spawned vendor that has one."""
    retail = f"""SELECT entry FROM tdb_npc_vendor WHERE type = {VENDOR_ITEM}
        AND entry IN (SELECT entry FROM temp.retail_entry)"""
    for entry, item in conn.execute(f"SELECT entry, item FROM content_npc_vendor WHERE entry IN ({retail})").fetchall():
        report.drop("npc_vendor", f"entry={entry}", item, "vendor list replaced by TrinityCore's")
    conn.execute(f"DELETE FROM content_npc_vendor WHERE entry IN ({retail})")
    rows = "FROM tdb_npc_vendor WHERE entry IN (SELECT entry FROM temp.retail_entry)"
    # "+item": probe by entry only; an index probe per (entry, item) pair is a cross product.
    conn.execute(f"""INSERT INTO content_npc_vendor (entry, slot, item, maxcount, incrtime, ExtendedCost, VerifiedBuild)
        SELECT entry, slot, item, maxcount, incrtime, ExtendedCost, VerifiedBuild {rows}
        AND type = {VENDOR_ITEM} AND +item IN (SELECT ID FROM content_item)""")
    for entry, item, kind in conn.execute(f"""SELECT entry, item, type {rows}
            AND (type <> {VENDOR_ITEM} OR +item NOT IN (SELECT ID FROM content_item))""").fetchall():
        reason = "currency cost row; currencies are not modelled" if kind == VENDOR_CURRENCY else "item not in Retail ItemSparse"
        report.drop("tdb_npc_vendor", f"entry={entry}", item, reason)


def insert_tdb_loot(conn, table, where, report, offset_entry=True):
    """Map TrinityCore loot rows (ItemType item/reference) of `table` into content_<table>."""
    entry = f"Entry + {TDB_LOOT_OFFSET}" if offset_entry else "Entry"
    reference = f"CASE ItemType WHEN {LOOT_REFERENCE} THEN Item + {TDB_LOOT_OFFSET} ELSE 0 END"
    # TrinityCore's loot tables have no primary key: a row repeated under (Entry, Item, Reference,
    # GroupId), all exact copies in TDB1210, is kept once.
    for key, item, copies in conn.execute(f"""SELECT {entry}, Item, count(*) FROM tdb_{table} WHERE ({where})
            AND ItemType IN ({LOOT_ITEM}, {LOOT_REFERENCE}) GROUP BY Entry, ItemType, Item, GroupId HAVING count(*) > 1
            """).fetchall():
        report.drop(f"tdb_{table}", f"Entry={key}", item, f"row repeated {copies} times; kept once")
    conn.execute(f"""INSERT OR IGNORE INTO content_{table}
            (Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount, Comment)
        SELECT {entry}, CASE ItemType WHEN {LOOT_REFERENCE} THEN Item + {TDB_LOOT_OFFSET} ELSE Item END, {reference},
            Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount, Comment
        FROM tdb_{table} WHERE ({where}) AND (ItemType = {LOOT_REFERENCE}
            OR (ItemType = {LOOT_ITEM} AND Item IN (SELECT ID FROM content_item)))""")
    for key, item, kind in conn.execute(f"""SELECT {entry}, Item, ItemType FROM tdb_{table} WHERE ({where})
            AND NOT (ItemType = {LOOT_REFERENCE} OR (ItemType = {LOOT_ITEM} AND Item IN (SELECT ID FROM content_item)))
            """).fetchall():
        reason = "currency loot; currencies are not modelled" if kind == LOOT_CURRENCY else "item not in Retail ItemSparse"
        report.drop(f"tdb_{table}", f"Entry={key}", item, reason)


def build_loot(conn, report):
    """TrinityCore loot (ids + TDB_LOOT_OFFSET) for Retail-spawned creatures that have some, AzerothCore's otherwise;
    per dungeon difficulty from that difficulty's creature_template_difficulty row."""
    conn.execute("""CREATE TEMP TABLE retail_loot AS SELECT d.Entry AS entry, d.LootID AS loot_id, d.GoldMin, d.GoldMax
        FROM temp.tdb_difficulty d
        WHERE d.Entry IN (SELECT entry FROM temp.retail_entry)
          AND d.LootID IN (SELECT Entry FROM tdb_creature_loot_template)""")
    conn.execute("""CREATE TEMP TABLE retail_difficulty_loot AS SELECT s.entry, s.difficulty, d.LootID AS loot_id,
            d.GoldMin, d.GoldMax
        FROM temp.spawn_row s JOIN tdb_creature_template_difficulty d ON d.Entry = s.entry AND d.DifficultyID = s.row_difficulty
        WHERE s.difficulty > 0 AND d.LootID IN (SELECT Entry FROM tdb_creature_loot_template)""")
    conn.execute(f"""UPDATE content_creature_template SET lootid = l.loot_id + {TDB_LOOT_OFFSET},
            mingold = l.GoldMin, maxgold = l.GoldMax
        FROM temp.retail_loot l WHERE l.entry = content_creature_template.entry""")
    conn.execute(f"""UPDATE content_creature_template_difficulty SET lootid = l.loot_id + {TDB_LOOT_OFFSET},
            mingold = l.GoldMin, maxgold = l.GoldMax
        FROM temp.retail_difficulty_loot l
        WHERE l.entry = content_creature_template_difficulty.entry AND l.difficulty = content_creature_template_difficulty.DifficultyID""")
    insert_tdb_loot(conn, "creature_loot_template", """Entry IN (SELECT loot_id FROM temp.retail_loot
        UNION SELECT loot_id FROM temp.retail_difficulty_loot)""", report)
    insert_tdb_loot(conn, "reference_loot_template", "1", report)
    insert_tdb_loot(conn, "gameobject_loot_template", "1", report)
    for container, item in conn.execute("""SELECT Entry, Item FROM content_item_loot_template
            WHERE Entry IN (SELECT Entry FROM tdb_item_loot_template)""").fetchall():
        report.drop("item_loot_template", f"Entry={container}", item, "container loot replaced by TrinityCore's")
    conn.execute("DELETE FROM content_item_loot_template WHERE Entry IN (SELECT Entry FROM tdb_item_loot_template)")
    insert_tdb_loot(conn, "item_loot_template", "Entry IN (SELECT ID FROM content_item)", report, offset_entry=False)
    build_wowhead_loot(conn, report)
    # Retail-spawned creatures with no loot table in any source drop nothing, money included
    # (TrinityCore difficulty-0 gold is ignored for them).
    no_loot = """entry IN (SELECT entry FROM temp.retail_entry)
        AND (lootid = 0 OR lootid NOT IN (SELECT Entry FROM content_creature_loot_template))"""
    for entry, low, high in conn.execute(f"""SELECT entry, mingold, maxgold FROM content_creature_template
            WHERE {no_loot} ORDER BY entry""").fetchall():
        report.drop("content_creature_template", f"entry={entry}", entry,
                    f"no loot table in either source; money {low}-{high} set to 0")
    conn.execute(f"UPDATE content_creature_template SET mingold = 0, maxgold = 0 WHERE {no_loot}")
    conn.execute(f"UPDATE content_creature_template_difficulty SET mingold = 0, maxgold = 0 WHERE {no_loot}")


# Wowhead-sourced loot template ids: entry + WOWHEAD_LOOT_OFFSET at an entry's lowest spawn difficulty (TDB's
# reach 20.7M), entry + WOWHEAD_LOOT_OFFSET + difficulty * WOWHEAD_DIFFICULTY_STRIDE at its other dungeon
# difficulties (entries stay below 1M).
WOWHEAD_LOOT_OFFSET = 30_000_000
WOWHEAD_DIFFICULTY_STRIDE = 1_000_000
ITEM_CLASS_QUEST = 12


def build_wowhead_loot(conn, report):
    """Wowhead drops (wowhead_loot.py) for Retail-spawned creatures without TrinityCore loot at a spawn difficulty:
    one ungrouped row per item and difficulty, Chance = count / outof of that difficulty's mode, stack from
    Wowhead; Quest-class items need the quest. Wowhead has no money data."""
    conn.execute("""CREATE TEMP TABLE spawn_base_difficulty AS SELECT entry, min(difficulty) AS difficulty
        FROM temp.spawn_map_difficulty GROUP BY entry""")
    conn.execute(f"""CREATE TEMP TABLE wowhead_entry AS SELECT DISTINCT w.entry, s.difficulty,
            CASE WHEN s.difficulty = b.difficulty THEN w.entry + {WOWHEAD_LOOT_OFFSET}
                 ELSE w.entry + {WOWHEAD_LOOT_OFFSET} + s.difficulty * {WOWHEAD_DIFFICULTY_STRIDE} END AS loot_id,
            s.difficulty = b.difficulty AS base
        FROM wowhead_creature_drop w
        JOIN temp.spawn_map_difficulty s ON s.entry = w.entry AND s.difficulty = w.difficulty
        JOIN temp.spawn_base_difficulty b ON b.entry = w.entry
        WHERE w.entry IN (SELECT entry FROM temp.retail_entry)
          AND NOT (s.difficulty = b.difficulty AND w.entry IN (SELECT entry FROM temp.retail_loot))
          AND NOT EXISTS (SELECT 1 FROM temp.retail_difficulty_loot l WHERE l.entry = w.entry AND l.difficulty = s.difficulty)""")
    for entry, scrape in conn.execute("""SELECT DISTINCT entry, scrape FROM wowhead_creature_drop
            WHERE entry NOT IN (SELECT entry FROM temp.wowhead_entry) ORDER BY entry""").fetchall():
        report.drop("wowhead_creature_drop", f"entry={entry}", entry,
                    f"{scrape}: not Retail-spawned, TrinityCore loot kept, or no drops at its spawn difficulty")
    rows = """FROM wowhead_creature_drop w JOIN temp.wowhead_entry e ON e.entry = w.entry AND e.difficulty = w.difficulty"""
    conn.execute(f"""INSERT INTO content_creature_loot_template
            (Entry, Item, Reference, Chance, QuestRequired, LootMode, GroupId, MinCount, MaxCount, Comment)
        SELECT e.loot_id, w.item, 0, 100.0 * w.count / w.outof, w.classs = {ITEM_CLASS_QUEST}, 1, 0,
            w.stack_min, w.stack_max, 'wowhead ' || w.scrape {rows} AND w.item IN (SELECT ID FROM content_item)""")
    for entry, item in conn.execute(f"""SELECT w.entry, w.item {rows}
            AND w.item NOT IN (SELECT ID FROM content_item) ORDER BY 1, 2""").fetchall():
        report.drop("wowhead_creature_drop", f"entry={entry}", item, "item not in Retail ItemSparse")
    conn.execute("""UPDATE content_creature_template SET lootid = e.loot_id, mingold = 0, maxgold = 0
        FROM temp.wowhead_entry e WHERE e.entry = content_creature_template.entry AND e.base""")
    conn.execute("""UPDATE content_creature_template_difficulty SET lootid = e.loot_id, mingold = 0, maxgold = 0
        FROM temp.wowhead_entry e
        WHERE e.entry = content_creature_template_difficulty.entry AND e.difficulty = content_creature_template_difficulty.DifficultyID""")


# --- Game objects ---

# TrinityCore GameObjectTypes whose Data1 is a gameobject_loot_template id (chest, fishing hole).
GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_FISHINGHOLE = 3, 25
AZEROTHCORE_DATA_FIELDS = 24
TDB_DATA_FIELDS = 35


def build_gameobjects(conn, report):
    """TrinityCore base-phase objects on RETAIL_MAPS (guid + RETAIL_GUID_OFFSET) with TrinityCore's templates."""
    for guid, map_id in conn.execute(f"SELECT guid, map FROM content_gameobject WHERE map IN ({MAP_LIST})").fetchall():
        report.drop("gameobject", f"guid={guid}", guid, f"map {map_id} spawn replaced by Retail spawns")
    conn.execute(f"DELETE FROM content_gameobject WHERE map IN ({MAP_LIST})")
    # No events are activated: positive-only memberships never spawn (as for creatures).
    holiday = """guid IN (SELECT guid FROM tdb_game_event_gameobject WHERE eventEntry > 0)
        AND guid NOT IN (SELECT guid FROM tdb_game_event_gameobject WHERE eventEntry < 0)"""
    conn.execute(f"""CREATE TEMP TABLE retail_gameobject AS SELECT guid AS tdb_guid, id FROM tdb_gameobject
        WHERE map IN ({MAP_LIST}) AND {BASE_PHASE} AND NOT ({holiday})""")
    for guid, *phase in conn.execute(f"""SELECT guid, PhaseId, PhaseGroup, terrainSwapMap, spawnDifficulties
            FROM tdb_gameobject WHERE map IN ({MAP_LIST}) AND NOT ({BASE_PHASE}) ORDER BY guid"""):
        report.drop("tdb_gameobject", f"guid={guid}", guid, not_base_phase_reason(*phase))
    for (guid,) in conn.execute(f"""SELECT guid FROM tdb_gameobject WHERE map IN ({MAP_LIST}) AND {BASE_PHASE}
            AND {holiday} ORDER BY guid"""):
        report.drop("tdb_gameobject", f"guid={guid}", guid, "holiday spawn (positive game event only)")
    build_gameobject_templates(conn, report)
    # TrinityCore `gameobject.spawnDifficulties` (the map's): the copy difficulties it spawns in.
    conn.execute("ALTER TABLE content_gameobject ADD COLUMN spawnDifficulties TEXT NOT NULL DEFAULT '0'")
    conn.execute(f"""INSERT INTO content_gameobject (guid, id, map, zoneId, areaId, position_x, position_y, position_z,
            orientation, rotation0, rotation1, rotation2, rotation3, spawntimesecs, animprogress, state, ScriptName,
            VerifiedBuild, spawnDifficulties)
        SELECT g.guid + {RETAIL_GUID_OFFSET}, g.id, g.map, g.zoneId, g.areaId, g.position_x, g.position_y, g.position_z,
            g.orientation, g.rotation0, g.rotation1, g.rotation2, g.rotation3, g.spawntimesecs, g.animprogress, g.state,
            g.ScriptName, g.VerifiedBuild, {SPAWN_DIFFICULTIES}
        FROM temp.retail_gameobject r JOIN tdb_gameobject g ON g.guid = r.tdb_guid
        WHERE g.id IN (SELECT entry FROM content_gameobject_template)""")
    for guid, entry in conn.execute("""SELECT tdb_guid, id FROM temp.retail_gameobject
            WHERE id NOT IN (SELECT entry FROM content_gameobject_template) ORDER BY tdb_guid"""):
        report.drop("tdb_gameobject", f"guid={guid}", entry, "template dropped")


# TrinityCore SpawnObjectType of pool_members.type (SpawnData.h): creature, gameobject, child pool.
POOL_CREATURE, POOL_GAMEOBJECT, POOL_POOL = 0, 1, 2


def build_pools(conn, report):
    """PoolMgr spawn pools (PoolMgr::LoadFromDB): every TDB pool_template and game_event_pool row, and the
    pool_members of the Retail spawns (creature and gameobject spawnId at their content guid) and child pools."""
    conn.execute("DROP TABLE IF EXISTS content_pool_template")
    conn.execute("""CREATE TABLE content_pool_template (entry INTEGER NOT NULL PRIMARY KEY,
        max_limit INTEGER NOT NULL, description TEXT)""")
    conn.execute("INSERT INTO content_pool_template SELECT entry, max_limit, description FROM tdb_pool_template")
    conn.execute("DROP TABLE IF EXISTS content_pool_members")
    conn.execute("""CREATE TABLE content_pool_members (type INTEGER NOT NULL, spawnId INTEGER NOT NULL,
        poolSpawnId INTEGER NOT NULL, chance REAL NOT NULL, description TEXT, PRIMARY KEY (type, spawnId))""")
    spawns = {POOL_CREATURE: "content_creature", POOL_GAMEOBJECT: "content_gameobject"}
    for member_type, table in spawns.items():
        conn.execute(f"""INSERT INTO content_pool_members
            SELECT type, spawnId + {RETAIL_GUID_OFFSET}, poolSpawnId, chance, description FROM tdb_pool_members
            WHERE type = ? AND spawnId + {RETAIL_GUID_OFFSET} IN (SELECT guid FROM {table})""", (member_type,))
        for (guid,) in conn.execute(f"""SELECT spawnId FROM tdb_pool_members WHERE type = ?
                AND spawnId + {RETAIL_GUID_OFFSET} NOT IN (SELECT guid FROM {table}) ORDER BY spawnId""",
                                    (member_type,)).fetchall():
            report.drop("tdb_pool_members", f"type={member_type} spawnId={guid}", guid, f"spawn not in {table}")
    conn.execute(f"INSERT INTO content_pool_members SELECT * FROM tdb_pool_members WHERE type = {POOL_POOL}")
    conn.execute("DROP TABLE IF EXISTS content_game_event_pool")
    conn.execute("""CREATE TABLE content_game_event_pool (eventEntry INTEGER NOT NULL,
        pool_entry INTEGER NOT NULL PRIMARY KEY)""")
    conn.execute("INSERT INTO content_game_event_pool SELECT eventEntry, pool_entry FROM tdb_game_event_pool")


def build_gameobject_templates(conn, report, *, retain_missing_displays=False):
    """TrinityCore's template (Data0-23, chest loot ids offset) replaces AzerothCore's for Retail-spawned entries."""
    retail = "SELECT DISTINCT id FROM temp.retail_gameobject"
    conn.execute(f"DELETE FROM content_gameobject_template WHERE entry IN ({retail})")
    conn.execute(f"DELETE FROM content_gameobject_template_addon WHERE entry IN ({retail})")
    data = ", ".join(f"Data{n}" for n in range(AZEROTHCORE_DATA_FIELDS))
    # TC GameObjectTemplate::GetLootId: type50 gatheringNode.chestLoot is
    # the same TDB loot namespace as chest and fishing-hole loot.
    loot_types = f"({GAMEOBJECT_TYPE_CHEST}, {GAMEOBJECT_TYPE_FISHINGHOLE}, 50)"
    mapped = data.replace("Data1,", f"CASE WHEN type IN {loot_types} AND Data1 <> 0 THEN Data1 + {TDB_LOOT_OFFSET} "
                                   "ELSE Data1 END,", 1)
    displayed = "(displayId = 0 OR displayId IN (SELECT ID FROM game_object_display_info))"
    included = "1" if retain_missing_displays else displayed
    conn.execute(f"""INSERT INTO content_gameobject_template (entry, type, displayId, name, IconName, castBarCaption, unk1,
            size, {data}, AIName, ScriptName, VerifiedBuild)
        SELECT entry, type, displayId, name, IconName, castBarCaption, unk1, size, {mapped}, AIName, ScriptName,
            VerifiedBuild
        FROM tdb_gameobject_template WHERE entry IN ({retail}) AND {included}""")
    for entry, display in conn.execute(f"""SELECT entry, displayId FROM tdb_gameobject_template
            WHERE entry IN ({retail}) AND NOT {displayed}"""):
        reason = "display absent from Retail GameObjectDisplayInfo"
        if retain_missing_displays:
            reason += "; authentic template retained, rendering unavailable"
        report.drop("tdb_gameobject_template", f"entry={entry}", display, reason)
    for n in range(AZEROTHCORE_DATA_FIELDS, TDB_DATA_FIELDS):
        for entry, value in conn.execute(f"""SELECT entry, Data{n} FROM tdb_gameobject_template
                WHERE entry IN (SELECT entry FROM content_gameobject_template) AND entry IN ({retail})
                  AND Data{n} <> 0"""):
            report.drop("tdb_gameobject_template", f"entry={entry} Data{n}", value, "Data24-34 not in the AzerothCore schema")
    conn.execute(f"""INSERT INTO content_gameobject_template_addon
            (entry, faction, flags, mingold, maxgold, artkit0, artkit1, artkit2, artkit3)
        SELECT entry, faction, flags, mingold, maxgold, artkit0, artkit1, artkit2, artkit3
        FROM tdb_gameobject_template_addon WHERE entry IN ({retail})
          AND entry IN (SELECT entry FROM content_gameobject_template)""")


# --- Gossip ---

# TrinityCore GossipOptionNpc → AzerothCore (OptionType, OptionNpcFlag, OptionIcon). Values are
# confirmed by the TDB option texts ("I want to browse your goods." 1, "Show me where I can
# fly." 2, "Train me." 3, "I'd like to browse the items up for auction." 10, stables 12).
GOSSIP_OPTION_NPC = {
    0: (1, 0x1, 0),              # None: GOSSIP_OPTION_GOSSIP, UNIT_NPC_FLAG_GOSSIP, chat
    1: (3, 0x80, 1),             # Vendor
    2: (4, 0x2000, 2),           # TaxiNode
    3: (5, 0x10, 3),             # Trainer
    4: (6, 0x4000, 4),           # SpiritHealer
    5: (8, 0x10000, 5),          # Binder (innkeeper)
    6: (9, 0x20000, 6),          # Banker
    7: (10, 0x40000, 7),         # PetitionVendor
    8: (11, 0x80000, 8),         # GuildTabardVendor
    9: (12, 0x100000, 9),        # Battlemaster
    10: (13, 0x200000, 6),       # Auctioneer
    12: (14, 0x400000, 0),       # Stablemaster
    47: (47, TRADING_POST_NPC_FLAG, 0),  # Mainline PerksProgramVendor
}
GOSSIP_OPTION_NONE = 0
NPC_TEXT_SLOTS = 8


def retail_menu_closure(conn):
    """Menus of Retail-spawned NPCs (TrinityCore's lowest per NPC) and every TrinityCore submenu they reach."""
    menus = {menu for (menu,) in conn.execute("SELECT gossip_menu_id FROM temp.retail_gossip")}
    frontier = set(menus)
    while frontier:
        found = {menu for (menu,) in conn.execute(
            f"SELECT DISTINCT ActionMenuID FROM tdb_gossip_menu_option WHERE ActionMenuID <> 0 "
            f"AND MenuID IN ({','.join(map(str, frontier))})")}
        frontier = found - menus
        menus |= found
    return menus


def build_gossip(conn, report):
    """TrinityCore menus, options and texts for Retail-spawned NPCs; AzerothCore's for every other menu."""
    conn.execute("""CREATE TEMP TABLE retail_gossip AS SELECT CreatureID AS entry, MIN(MenuID) AS gossip_menu_id
        FROM tdb_creature_template_gossip WHERE CreatureID IN (SELECT entry FROM temp.retail_entry) GROUP BY CreatureID""")
    for entry, menu, used in conn.execute("""SELECT g.CreatureID, g.MenuID, r.gossip_menu_id FROM tdb_creature_template_gossip g
            JOIN temp.retail_gossip r ON r.entry = g.CreatureID WHERE g.MenuID <> r.gossip_menu_id ORDER BY 1, 2""").fetchall():
        report.drop("tdb_creature_template_gossip", f"CreatureID={entry}", menu,
                    f"more than one menu; the server has no conditions, lowest MenuID {used} used")
    conn.execute("""UPDATE content_creature_template SET gossip_menu_id = r.gossip_menu_id
        FROM temp.retail_gossip r WHERE r.entry = content_creature_template.entry""")
    conn.execute("CREATE TEMP TABLE retail_menu (MenuID INTEGER PRIMARY KEY)")
    conn.executemany("INSERT INTO temp.retail_menu VALUES (?)", ((menu,) for menu in sorted(retail_menu_closure(conn))))
    menus = "SELECT MenuID FROM temp.retail_menu"
    for table in ("gossip_menu", "gossip_menu_option"):
        clone_table(conn, table, f"content_{table}")
        conn.execute(f"INSERT INTO content_{table} SELECT * FROM {table} WHERE MenuID NOT IN ({menus})")
    conn.execute(f"""INSERT INTO content_gossip_menu (MenuID, TextID)
        SELECT MenuID, TextID FROM tdb_gossip_menu WHERE MenuID IN ({menus})""")
    build_gossip_options(conn, menus, report)
    texts = f"SELECT TextID FROM tdb_gossip_menu WHERE MenuID IN ({menus})"
    clone_table(conn, "npc_text", "content_npc_text")
    conn.execute(f"INSERT INTO content_npc_text SELECT * FROM npc_text WHERE ID NOT IN ({texts})")
    slots = range(NPC_TEXT_SLOTS)
    conn.execute(f"""INSERT INTO content_npc_text (ID, {', '.join(f'BroadcastTextID{n}, Probability{n}' for n in slots)})
        SELECT ID, {', '.join(f'BroadcastTextID{n}, Probability{n}' for n in slots)}
        FROM tdb_npc_text WHERE ID IN ({texts})""")
    build_broadcast_text(conn)
    for text, broadcast in conn.execute(" UNION ".join(
            f"SELECT ID, BroadcastTextID{n} FROM content_npc_text WHERE ID IN ({texts}) AND BroadcastTextID{n} <> 0"
            f" AND BroadcastTextID{n} NOT IN (SELECT ID FROM content_broadcast_text)" for n in slots)).fetchall():
        report.drop("tdb_npc_text", f"ID={text}", broadcast,
                    "BroadcastText id in neither TrinityCore's nor AzerothCore's broadcast_text")


def build_broadcast_text(conn):
    """content_broadcast_text: TrinityCore's Retail BroadcastText (hotfixes DB, latest VerifiedBuild per ID;
    Text/Text1 are the male/female texts), plus AzerothCore's rows for ids TrinityCore lacks."""
    clone_table(conn, "broadcast_text", "content_broadcast_text")
    conn.execute("""INSERT INTO content_broadcast_text (ID, LanguageID, MaleText, FemaleText, EmoteID1, EmoteID2,
            EmoteID3, EmoteDelay1, EmoteDelay2, EmoteDelay3, SoundEntriesId, EmotesID, Flags, VerifiedBuild)
        SELECT ID, LanguageID, Text, Text1, EmoteID1, EmoteID2, EmoteID3, EmoteDelay1, EmoteDelay2, EmoteDelay3,
            SoundKitID1, EmotesID, Flags, VerifiedBuild
        FROM tdb_broadcast_text t WHERE VerifiedBuild = (SELECT MAX(VerifiedBuild) FROM tdb_broadcast_text l
            WHERE l.ID = t.ID)""")
    conn.execute("INSERT INTO content_broadcast_text SELECT * FROM broadcast_text WHERE ID NOT IN (SELECT ID FROM tdb_broadcast_text)")


def build_creature_text(conn):
    """content_creature_text: every TrinityCore creature_text row (CreatureTextMgr::LoadCreatureTexts loads
    the whole table; scripts Talk by CreatureID and GroupID)."""
    clone_table(conn, "tdb_creature_text", "content_creature_text")
    conn.execute("INSERT INTO content_creature_text SELECT * FROM tdb_creature_text")


def build_gossip_options(conn, menus, report):
    conn.execute("CREATE TEMP TABLE gossip_option_npc (npc INTEGER PRIMARY KEY, type INTEGER, flag INTEGER, icon INTEGER)")
    conn.executemany("INSERT INTO temp.gossip_option_npc VALUES (?, ?, ?, ?)",
                     ((npc, *values) for npc, values in GOSSIP_OPTION_NPC.items()))
    conn.execute(f"""INSERT INTO content_gossip_menu_option (MenuID, OptionID, OptionIcon, OptionText, OptionBroadcastTextID,
            OptionType, OptionNpcFlag, ActionMenuID, ActionPoiID, BoxCoded, BoxMoney, BoxText, BoxBroadcastTextID,
            VerifiedBuild)
        SELECT o.MenuID, o.OptionID, COALESCE(n.icon, 0), o.OptionText, o.OptionBroadcastTextID,
            COALESCE(n.type, {GOSSIP_OPTION_NONE}), COALESCE(n.flag, 1), o.ActionMenuID, o.ActionPoiID, o.BoxCoded,
            o.BoxMoney, o.BoxText, o.BoxBroadcastTextID, o.VerifiedBuild
        FROM tdb_gossip_menu_option o LEFT JOIN temp.gossip_option_npc n ON n.npc = o.OptionNpc
        WHERE o.MenuID IN ({menus})""")
    for menu, option, npc in conn.execute(f"""SELECT MenuID, OptionID, OptionNpc FROM tdb_gossip_menu_option
            WHERE MenuID IN ({menus}) AND OptionNpc NOT IN (SELECT npc FROM temp.gossip_option_npc)
            ORDER BY MenuID, OptionID""").fetchall():
        report.drop("tdb_gossip_menu_option", f"MenuID={menu} OptionID={option}", npc,
                    f"GossipOptionNpc {npc} has no AzerothCore option type; hidden")


# --- Quests ---

# TrinityCore QuestObjectiveType values the AzerothCore quest shape holds.
OBJECTIVE_MONSTER, OBJECTIVE_ITEM, OBJECTIVE_GAMEOBJECT, OBJECTIVE_PLAYERKILLS = 0, 1, 2, 9
# QUEST_OBJECTIVE_AREATRIGGER: held as AzerothCore's exploration flag plus areatrigger_involvedrelation.
OBJECTIVE_AREATRIGGER = 10
# TrinityCore QuestFlags a quest completes through (Player::CanCompleteQuest needs Explored):
# QUEST_FLAGS_COMPLETION_EVENT 0x2 | QUEST_FLAGS_COMPLETION_AREA_TRIGGER 0x4 (QuestDef.h:215-216).
QUEST_FLAGS_COMPLETION_EXPLORED = 0x2 | 0x4
# AzerothCore QUEST_SPECIAL_FLAGS_EXPLORATION_OR_EVENT (QuestDef.h:161): the quest completes when an
# area is explored or a script event happens. TrinityCore's 0x2 is AUTO_PUSH_TO_PARTY (QuestDef.h:321).
SPECIAL_FLAGS_EXPLORATION_OR_EVENT = 0x2
QUEST_MONEY_DIFFICULTIES = 10
NPC_OR_GO_SLOTS, ITEM_SLOTS = 4, 6
# AzerothCore column ← TrinityCore column, same meaning.
QUEST_COLUMNS = {name: name for name in (
    "ID", "QuestType", "QuestSortID", "QuestInfoID", "SuggestedGroupNum", "RewardNextQuest", "RewardXPDifficulty",
    "RewardMoneyDifficulty", "RewardSpell", "RewardHonor", "RewardKillHonor", "StartItem", "Flags",
    *[f"RewardItem{n}" for n in range(1, 5)], *[f"RewardAmount{n}" for n in range(1, 5)],
    *[f"ItemDrop{n}" for n in range(1, 5)], *[f"ItemDropQuantity{n}" for n in range(1, 5)],
    *[f"RewardChoiceItemID{n}" for n in range(1, 7)], *[f"RewardChoiceItemQuantity{n}" for n in range(1, 7)],
    "POIContinent", "POIx", "POIy", "POIPriority", "RewardTitle", "RewardArenaPoints",
    *[f"RewardFaction{kind}{n}" for n in range(1, 6) for kind in ("ID", "Value", "Override")],
    "TimeAllowed", "AllowableRaces", "LogTitle", "LogDescription", "QuestDescription", "AreaDescription",
    "QuestCompletionLog", "VerifiedBuild", "ContentTuningID", "RewardMoneyMultiplier")} | {
    "RewardDisplaySpell": "RewardDisplaySpell1"}
REWARD_ITEM_COLUMNS = ([f"RewardItem{n}" for n in range(1, 5)] + [f"RewardChoiceItemID{n}" for n in range(1, 7)]
                       + [f"ItemDrop{n}" for n in range(1, 5)] + ["StartItem"])
REWARD_ITEM_AMOUNTS = {f"RewardItem{n}": f"RewardAmount{n}" for n in range(1, 5)} | {
    f"RewardChoiceItemID{n}": f"RewardChoiceItemQuantity{n}" for n in range(1, 7)} | {
    f"ItemDrop{n}": f"ItemDropQuantity{n}" for n in range(1, 5)}
RELATION_TABLES = {"creature_queststarter": "temp.retail_entry", "creature_questender": "temp.retail_entry",
                   "gameobject_queststarter": "temp.retail_gameobject_entry",
                   "gameobject_questender": "temp.retail_gameobject_entry"}


def retail_relations(conn):
    """temp.retail_relation (tbl, id, quest): TrinityCore's relations of Retail-spawned NPCs and objects.

    AzerothCore relations are not added for TrinityCore quests without a starter: 1,993 of the
    2,068 such AzerothCore rows are quests TrinityCore flags 0x4000 with no ContentTuning
    (removed by Cataclysm, e.g. 783 "[DEPRECATED] A Threat Within"), the rest holiday or
    removed quests (holiday quest givers are game_event_creature_quest in TrinityCore)."""
    conn.execute("CREATE TEMP TABLE retail_gameobject_entry AS SELECT DISTINCT id AS entry FROM temp.retail_gameobject")
    conn.execute("CREATE TEMP TABLE retail_relation (tbl TEXT, id INTEGER, quest INTEGER)")
    for table, spawned in RELATION_TABLES.items():
        conn.execute(f"""INSERT INTO temp.retail_relation SELECT '{table}', id, quest FROM tdb_{table}
            WHERE id IN (SELECT entry FROM {spawned})""")


def build_quests(conn, report):
    """TrinityCore quests (mapped to the AzerothCore shape) for Retail-spawned quest givers and enders."""
    retail_relations(conn)
    for (quest,) in conn.execute("""SELECT DISTINCT quest FROM temp.retail_relation
            WHERE quest NOT IN (SELECT ID FROM quest_v2) ORDER BY quest""").fetchall():
        report.drop("tdb_quest_template", f"ID={quest}", quest, "quest id not in Retail QuestV2")
    conn.execute("""CREATE TEMP TABLE retail_quest AS SELECT DISTINCT quest AS ID FROM temp.retail_relation
        WHERE quest IN (SELECT ID FROM quest_v2) AND quest IN (SELECT ID FROM tdb_quest_template)""")
    quests = "SELECT ID FROM temp.retail_quest"
    for (quest,) in conn.execute(f"SELECT ID FROM content_quest_template WHERE ID IN ({quests})").fetchall():
        report.drop("quest_template", f"ID={quest}", quest, "replaced by TrinityCore's quest")
    conn.execute(f"CREATE TEMP TABLE azerothcore_quest AS SELECT * FROM content_quest_template WHERE ID IN ({quests})")
    conn.execute(f"DELETE FROM content_quest_template WHERE ID IN ({quests})")
    conn.execute("ALTER TABLE content_quest_template ADD COLUMN ContentTuningID INTEGER NOT NULL DEFAULT 0")
    conn.execute("ALTER TABLE content_quest_template ADD COLUMN RewardMoneyMultiplier REAL NOT NULL DEFAULT 1")
    insert_quest_templates(conn, quests, report)
    map_quest_objectives(conn, quests, report)
    build_quest_children(conn, quests, report)
    map_quest_exploration(conn, quests, report)
    build_quest_relations(conn, report)


def insert_quest_templates(conn, quests, report):
    targets = ", ".join(QUEST_COLUMNS)
    sources = ", ".join(f"q.{column}" for column in QUEST_COLUMNS.values())
    # QuestLevel -1 scales to the player (Quest::XPValue); MinLevel is ContentTuning's minimum.
    # RewardMoney is TrinityCore's Quest::MaxMoneyValue (QuestDef.cpp:486-494):
    # QuestMoneyReward[ContentTuning MaxLevel].Difficulty[RewardMoneyDifficulty] ×
    # RewardMoneyMultiplier, truncated to uint32; 0 without ContentTuning. The server pays each
    # player Quest::MoneyValue (quest_rules.rs quest_money) from RewardMoneyDifficulty,
    # RewardMoneyMultiplier, ContentTuningID, content_tuning_level and quest_money_reward.
    difficulty = "CASE q.RewardMoneyDifficulty " + " ".join(
        f"WHEN {n} THEN m.Difficulty_{n}" for n in range(QUEST_MONEY_DIFFICULTIES)) + " ELSE 0 END"
    conn.execute(f"""INSERT INTO content_quest_template ({targets}, QuestLevel, MinLevel, RewardMoney)
        SELECT {sources}, -1, COALESCE(l.minlevel, 0),
            COALESCE(CAST({difficulty} * q.RewardMoneyMultiplier AS INTEGER), 0)
        FROM tdb_quest_template q LEFT JOIN content_tuning_level l ON l.ID = q.ContentTuningID
        LEFT JOIN quest_money_reward m ON m.ID = l.maxlevel WHERE q.ID IN ({quests})""")
    fill_rewards_from_azerothcore(conn, quests, report)
    for column in REWARD_ITEM_COLUMNS:
        amount = REWARD_ITEM_AMOUNTS.get(column)
        for quest, item in conn.execute(f"""SELECT ID, {column} FROM content_quest_template WHERE ID IN ({quests})
                AND {column} <> 0 AND {column} NOT IN (SELECT ID FROM content_item)""").fetchall():
            report.drop("tdb_quest_template", f"ID={quest} {column}", item, "item not in Retail ItemSparse; zeroed")
            conn.execute(f"UPDATE content_quest_template SET {column} = 0{f', {amount} = 0' if amount else ''} WHERE ID = ?",
                         (quest,))
    for column, source in (("RewardSpell", "RewardSpell"), ("RewardDisplaySpell", "RewardDisplaySpell1")):
        for quest, spell in conn.execute(f"""SELECT ID, {column} FROM content_quest_template WHERE ID IN ({quests})
                AND {column} > 0 AND {column} NOT IN (SELECT ID FROM spell_name)""").fetchall():
            report.drop("tdb_quest_template", f"ID={quest} {source}", spell, "spell not in Retail SpellName; zeroed")
            conn.execute(f"UPDATE content_quest_template SET {column} = 0 WHERE ID = ?", (quest,))


# TrinityCore's reward columns come from sniffed quest offers: 1 of 3,212 Retail quests has
# choice rewards; AzerothCore's (Retail-mapped) group fills an all-zero TrinityCore group.
REWARD_GROUPS = ([(f"RewardItem{n}", f"RewardAmount{n}") for n in range(1, 5)],
                 [(f"RewardChoiceItemID{n}", f"RewardChoiceItemQuantity{n}") for n in range(1, 7)])


def fill_rewards_from_azerothcore(conn, quests, report):
    """An all-zero TrinityCore group takes the same-ID AzerothCore quest's mapped group (reported)."""
    for group in REWARD_GROUPS:
        columns = [column for pair in group for column in pair]
        empty = " AND ".join(f"q.{item} = 0" for item, _ in group)
        filled = f"""q.ID IN ({quests}) AND {empty}
              AND q.ID IN (SELECT ID FROM temp.azerothcore_quest WHERE {group[0][0]} <> 0)"""
        for (quest,) in conn.execute(f"SELECT q.ID FROM content_quest_template q WHERE {filled} ORDER BY q.ID").fetchall():
            report.drop("content_quest_template", f"ID={quest} {group[0][0][:-1]}", quest,
                        "empty TrinityCore reward group filled from the same-ID AzerothCore quest")
        conn.execute(f"""UPDATE content_quest_template AS q SET ({', '.join(columns)}) =
            (SELECT {', '.join(columns)} FROM temp.azerothcore_quest a WHERE a.ID = q.ID)
            WHERE {filled}""")


def map_quest_objectives(conn, quests, report):
    """quest_objectives → RequiredNpcOrGo1-4 (game objects negative), RequiredItemId1-6, RequiredPlayerKills."""
    objectives = conn.execute(f"""SELECT QuestID, ID, Type, ObjectID, Amount, Description FROM tdb_quest_objectives
        WHERE QuestID IN ({quests}) ORDER BY QuestID, "Order", StorageIndex, ID""").fetchall()
    by_quest = {}
    for quest, objective, kind, object_id, amount, description in objectives:
        slots = by_quest.setdefault(quest, {"npc": [], "item": [], "kills": 0})
        if kind in (OBJECTIVE_MONSTER, OBJECTIVE_GAMEOBJECT) and len(slots["npc"]) < NPC_OR_GO_SLOTS:
            slots["npc"].append((object_id if kind == OBJECTIVE_MONSTER else -object_id, amount, description or ""))
        elif kind == OBJECTIVE_ITEM and len(slots["item"]) < ITEM_SLOTS:
            slots["item"].append((object_id, amount))
        elif kind == OBJECTIVE_PLAYERKILLS:
            slots["kills"] = amount
        elif kind == OBJECTIVE_AREATRIGGER:
            continue  # map_quest_exploration
        elif kind in (OBJECTIVE_MONSTER, OBJECTIVE_GAMEOBJECT, OBJECTIVE_ITEM):
            report.drop("tdb_quest_objectives", f"QuestID={quest} ID={objective}", object_id,
                        f"more objectives of type {kind} than the AzerothCore quest shape holds")
        else:
            report.drop("tdb_quest_objectives", f"QuestID={quest} ID={objective}", object_id,
                        f"objective type {kind} not supported")
    for quest, slots in by_quest.items():
        values = {"RequiredPlayerKills": slots["kills"]}
        for n, (object_id, amount, text) in enumerate(slots["npc"], 1):
            values |= {f"RequiredNpcOrGo{n}": object_id, f"RequiredNpcOrGoCount{n}": amount, f"ObjectiveText{n}": text}
        for n, (item, amount) in enumerate(slots["item"], 1):
            values |= {f"RequiredItemId{n}": item, f"RequiredItemCount{n}": amount}
        conn.execute(f"UPDATE content_quest_template SET {', '.join(f'{c} = ?' for c in values)} WHERE ID = ?",
                     (*values.values(), quest))


def map_quest_exploration(conn, quests, report):
    """QUEST_OBJECTIVE_AREATRIGGER and the completion flags → AzerothCore's EXPLORATION_OR_EVENT special flag;
    TrinityCore's areatrigger_involvedrelation plus each objective's own trigger → content_areatrigger_involvedrelation.

    ObjectMgr::LoadQuestAreaTriggers (ObjectMgr.cpp:6436-6494) keeps a relation only for a quest with
    QUEST_FLAGS_COMPLETION_AREA_TRIGGER or an area trigger objective, and adds every objective's ObjectID."""
    for (quest,) in conn.execute(f"""SELECT ID FROM content_quest_template_addon WHERE ID IN ({quests})
            AND SpecialFlags & {SPECIAL_FLAGS_EXPLORATION_OR_EVENT} ORDER BY ID""").fetchall():
        report.drop("tdb_quest_template_addon", f"ID={quest} SpecialFlags", SPECIAL_FLAGS_EXPLORATION_OR_EVENT,
                    "QUEST_SPECIAL_FLAGS_AUTO_PUSH_TO_PARTY not modelled; cleared")
    conn.execute(f"""UPDATE content_quest_template_addon SET SpecialFlags = SpecialFlags & ~{SPECIAL_FLAGS_EXPLORATION_OR_EVENT}
        WHERE ID IN ({quests})""")
    explored = f"""SELECT ID FROM content_quest_template WHERE ID IN ({quests}) AND (Flags & {QUEST_FLAGS_COMPLETION_EXPLORED}
        OR ID IN (SELECT QuestID FROM tdb_quest_objectives WHERE Type = {OBJECTIVE_AREATRIGGER}))"""
    conn.execute(f"""INSERT OR IGNORE INTO content_quest_template_addon (ID) SELECT ID FROM ({explored})""")
    conn.execute(f"""UPDATE content_quest_template_addon SET SpecialFlags = SpecialFlags | {SPECIAL_FLAGS_EXPLORATION_OR_EVENT}
        WHERE ID IN ({explored})""")
    conn.execute(f"DELETE FROM content_areatrigger_involvedrelation WHERE quest IN ({quests})")
    triggers = f"""SELECT id, quest FROM tdb_areatrigger_involvedrelation UNION
        SELECT ObjectID, QuestID FROM tdb_quest_objectives WHERE Type = {OBJECTIVE_AREATRIGGER} AND ObjectID > 0"""
    kept = f"quest IN ({explored}) AND id IN (SELECT ID FROM area_trigger)"
    for trigger, quest in conn.execute(f"""SELECT id, quest FROM ({triggers}) WHERE quest IN ({quests}) AND NOT ({kept})
            ORDER BY id""").fetchall():
        report.drop("tdb_areatrigger_involvedrelation", f"id={trigger}", quest,
                    "quest has no area trigger objective or completion flag, or AreaTrigger.db2 lacks the id")
    conn.execute(f"""INSERT OR IGNORE INTO content_areatrigger_involvedrelation (id, quest)
        SELECT id, quest FROM ({triggers}) WHERE quest IN ({quests}) AND {kept}""")


def build_quest_children(conn, quests, report):
    """Addon, offer/request texts and POIs: TrinityCore's rows where it has them, AzerothCore's otherwise."""
    addon = ("ID, MaxLevel, AllowableClasses, SourceSpellID, PrevQuestID, NextQuestID, ExclusiveGroup, "
             "RewardMailTemplateID, RewardMailDelay, RequiredSkillID, RequiredSkillPoints, RequiredMinRepFaction, "
             "RequiredMaxRepFaction, RequiredMinRepValue, RequiredMaxRepValue, ProvidedItemCount, SpecialFlags")
    children = {
        "quest_template_addon": ("ID", addon, addon),
        "quest_offer_reward": ("ID", "ID, Emote1, Emote2, Emote3, Emote4, EmoteDelay1, EmoteDelay2, EmoteDelay3, "
                               "EmoteDelay4, RewardText, VerifiedBuild", None),
        "quest_request_items": ("ID", "ID, EmoteOnComplete, EmoteOnIncomplete, CompletionText, VerifiedBuild", None),
        # One POI per (QuestID, Idx1), the lowest BlobIndex: the points are keyed by Idx1 alone.
        "quest_poi": ("QuestID", "QuestID, id, ObjectiveIndex, MapID, WorldMapAreaId, Floor, Priority, Flags, VerifiedBuild",
                      "QuestID, Idx1, ObjectiveIndex, MapID, UiMapID, 0, Priority, Flags, VerifiedBuild"),
        "quest_poi_points": ("QuestID", "QuestID, Idx1, Idx2, X, Y, VerifiedBuild", None),
    }
    for table, (key, columns, sources) in children.items():
        tdb_keyed = f"SELECT {key} FROM tdb_{table} WHERE {key} IN ({quests})"
        conn.execute(f"DELETE FROM content_{table} WHERE {key} IN ({tdb_keyed})")
        lowest_blob = """AND BlobIndex = (SELECT MIN(BlobIndex) FROM tdb_quest_poi b
            WHERE b.QuestID = tdb_quest_poi.QuestID AND b.Idx1 = tdb_quest_poi.Idx1)""" if table == "quest_poi" else ""
        conn.execute(f"""INSERT INTO content_{table} ({columns}) SELECT {sources or columns} FROM tdb_{table}
            WHERE {key} IN ({quests}) {lowest_blob}""")
        if table == "quest_poi":
            for quest, index, blob in conn.execute(f"""SELECT QuestID, Idx1, BlobIndex FROM tdb_quest_poi
                    WHERE QuestID IN ({quests}) AND NOT (1 {lowest_blob}) ORDER BY 1, 2, 3""").fetchall():
                report.drop("tdb_quest_poi", f"QuestID={quest} Idx1={index}", blob, "second blob of the POI; lowest kept")
        # AzerothCore rows of a TrinityCore quest without TrinityCore rows stay; so do other quests'.
        conn.execute(f"""INSERT OR IGNORE INTO content_{table} SELECT * FROM {table}
            WHERE {key} IN ({quests}) AND {key} NOT IN ({tdb_keyed})""")


def build_quest_relations(conn, report):
    """Retail-spawned NPCs/objects give and end the resolved relations; every other entry keeps AzerothCore's."""
    for table, spawned in RELATION_TABLES.items():
        for entry, quest in conn.execute(f"""SELECT id, quest FROM content_{table} WHERE id IN (SELECT entry FROM {spawned})
                AND (id, quest) NOT IN (SELECT id, quest FROM temp.retail_relation WHERE tbl = '{table}')""").fetchall():
            report.drop(table, f"id={entry}", quest, "relation of a Retail-spawned entry replaced by TrinityCore's")
        conn.execute(f"DELETE FROM content_{table} WHERE id IN (SELECT entry FROM {spawned})")
        conn.execute(f"""INSERT OR IGNORE INTO content_{table} (id, quest) SELECT id, quest FROM temp.retail_relation
            WHERE tbl = '{table}' AND quest IN (SELECT ID FROM content_quest_template)""")
