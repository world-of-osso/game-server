"""Retail world content of maps 0 and 1 from TrinityCore TDB1210 (content_policy.py step).

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

RETAIL_MAPS = (0, 1)
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


def create_level_table(conn):
    """temp.tuning_level: the level range of every ContentTuning row."""
    conn.execute("CREATE TEMP TABLE tuning_level (ID INTEGER PRIMARY KEY, minlevel INTEGER, maxlevel INTEGER)")
    conn.executemany("INSERT INTO temp.tuning_level VALUES (?, ?, ?)", (
        (tuning, *content_tuning_levels(*values)) for tuning, *values in conn.execute(
            "SELECT ID, MinLevelSquish, MaxLevelSquish, MinLevelScalingOffset, MaxLevelScalingOffset FROM content_tuning")))


def build_templates(conn, report):
    """content_creature_template: AzerothCore's, plus TrinityCore's for spawned entries AzerothCore lacks."""
    clone_table(conn, "creature_template", "content_creature_template")
    conn.execute("INSERT INTO content_creature_template SELECT * FROM creature_template")
    conn.execute("ALTER TABLE content_creature_template ADD COLUMN ContentTuningID INTEGER NOT NULL DEFAULT 0")
    create_level_table(conn)
    conn.execute("""CREATE TEMP TABLE tdb_difficulty AS SELECT * FROM tdb_creature_template_difficulty WHERE DifficultyID = 0""")
    conn.execute("CREATE UNIQUE INDEX temp.tdb_difficulty_entry ON tdb_difficulty(Entry)")
    conn.create_function("health_expansion", 1, health_expansion, deterministic=True)
    # npcflag's high word is npcflag2. Levels are the ContentTuning range (level 1 without one).
    conn.execute("""INSERT INTO content_creature_template (entry, KillCredit1, KillCredit2, name, subname, IconName,
            minlevel, maxlevel, exp, faction, npcflag, speed_walk, speed_run, scale, rank, dmgschool, DamageModifier,
            BaseAttackTime, RangeAttackTime, BaseVariance, RangeVariance, unit_class, unit_flags, unit_flags2, family,
            type, type_flags, VehicleId, mingold, maxgold, AIName, MovementType, HealthModifier, ManaModifier,
            ArmorModifier, ExperienceModifier, RacialLeader, movementId, RegenHealth, flags_extra, ScriptName, VerifiedBuild,
            ContentTuningID)
        SELECT t.entry, t.KillCredit1, t.KillCredit2, t.name, t.subname, t.IconName,
            COALESCE(l.minlevel, 1), COALESCE(l.maxlevel, 1), health_expansion(d.HealthScalingExpansion), t.faction,
            t.npcflag & 4294967295, t.speed_walk, t.speed_run, t.scale,
            t.Classification, t.dmgschool, d.DamageModifier, t.BaseAttackTime, t.RangeAttackTime, t.BaseVariance,
            t.RangeVariance, t.unit_class, t.unit_flags, t.unit_flags2, t.family, t.type, d.TypeFlags, t.VehicleId,
            d.GoldMin, d.GoldMax, t.AIName, t.MovementType, d.HealthModifier, d.ManaModifier, d.ArmorModifier,
            t.ExperienceModifier, t.RacialLeader, t.movementId, t.RegenHealth, t.flags_extra, t.ScriptName, t.VerifiedBuild,
            d.ContentTuningID
        FROM tdb_creature_template t
        JOIN temp.tdb_difficulty d ON d.Entry = t.entry
        LEFT JOIN temp.tuning_level l ON l.ID = d.ContentTuningID
        WHERE t.entry IN (SELECT id1 FROM content_creature WHERE guid IN (SELECT guid FROM temp.retail_spawn))
          AND t.entry NOT IN (SELECT entry FROM creature_template)""")
    # AzerothCore templates TrinityCore also tunes take its ContentTuning range.
    conn.execute("""UPDATE content_creature_template SET ContentTuningID = d.ContentTuningID,
            minlevel = l.minlevel, maxlevel = l.maxlevel
        FROM temp.tdb_difficulty d JOIN temp.tuning_level l ON l.ID = d.ContentTuningID
        WHERE d.Entry = content_creature_template.entry
          AND content_creature_template.entry IN (SELECT entry FROM creature_template)""")
    for entry, tuning in conn.execute("""SELECT t.entry, d.ContentTuningID FROM content_creature_template t
            JOIN temp.tdb_difficulty d ON d.Entry = t.entry
            WHERE d.ContentTuningID NOT IN (SELECT ID FROM temp.tuning_level) ORDER BY t.entry""").fetchall():
        fixed = "level 1" if not conn.execute("SELECT 1 FROM creature_template WHERE entry = ?", (entry,)).fetchone() \
            else "AzerothCore levels kept"
        report.drop("tdb_creature_template_difficulty", f"Entry={entry}", tuning, f"ContentTuningID not in ContentTuning; {fixed}")
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
