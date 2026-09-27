use bevy::prelude::*;
use rusqlite::{Connection, OpenFlags, Row, params};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::creature_aggro::AggroMode;
use crate::level_scaling::{
    ArmorConstants, CreatureStats, ExpectedStats, LevelStats, TuningInputs,
};

pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    /// `subname`: the tooltip subtitle ("Weaponsmith"); empty when none.
    pub subname: String,
    /// `type`: Retail `CreatureType` (1 Beast … 7 Humanoid, 8 Critter …).
    pub creature_type: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub faction: u16,
    pub npc_flags: u32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub health_modifier: f32,
    /// `unit_class`: the `creature_classlevelstats.class` row.
    pub unit_class: u8,
    /// `exp`: which `creature_classlevelstats.basehp<exp>` column applies.
    pub expansion: u8,
    /// `rank`: 1 elite, 2 rare elite (Retail `CreatureClassifications`).
    pub rank: u8,
    /// `ExperienceModifier` (TrinityCore `ModExperience`).
    pub experience_modifier: f32,
    /// Critters and `CREATURE_FLAG_EXTRA_NO_XP` give no experience.
    pub gives_experience: bool,
    /// `ScriptName`: the AzerothCore CreatureScript (`creature_scripts`).
    pub script_name: String,
    /// `detection_range`: aggro radius against a same-level target.
    pub detection_range: f32,
    pub aggro_mode: AggroMode,
    /// `CREATURE_FLAG_EXTRA_TRIGGER`: the template spawns with its invisible model.
    pub trigger: bool,
    /// `unit_flags` (TrinityCore `UnitFlags`), unless the spawn overrides them.
    pub unit_flags: u32,
    pub models: Vec<CreatureModelInfo>,
    /// Retail ContentTuning scaling; `None` keeps the fixed `min_level` and
    /// creature_classlevelstats health.
    pub tuning: Option<TemplateTuning>,
    /// Armor at `min_level` of an untuned template (ExpectedStat
    /// CreatureArmor × ArmorModifier); a tuned one reads `tuning.stats`.
    pub armor: f32,
    /// `ManaModifier` (creature_template_difficulty): max mana is
    /// creature_classlevelstats `basemana` × this (Creature::UpdateLevelDependantStats).
    pub mana_modifier: f32,
    /// `UNIT_FLAG2_REGENERATE_POWER` in `unit_flags2` (Creature::Regenerate).
    pub regenerates_power: bool,
}

/// A template with a `ContentTuningID`: `min_level`/`max_level` are its
/// ContentTuning range (docs/specs/level-scaling.md).
pub struct TemplateTuning {
    pub content_tuning_id: u32,
    /// `LevelScalingDeltaMin..=Max`, rolled per spawn (Creature::ApplyLevelScaling).
    pub delta: (i8, i8),
    pub stats: Arc<LevelStats>,
}

/// `creature_template_model` row.
pub struct CreatureModelInfo {
    pub display_id: u32,
    /// Weight of the model among the template's (`GetRandomValidModel`).
    pub probability: f32,
    /// `CreatureModelInfo::is_trigger`: the display's model file is an invisible trigger model.
    pub is_trigger: bool,
}

pub struct CreatureSpawn {
    pub guid: u32,
    pub template_entry: u32,
    pub map: u16,
    pub zone_id: u16,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub wander_distance: f32,
    pub movement_type: u8,
    /// `creature.spawntimesecs`: respawn delay after the corpse decays.
    pub spawntimesecs: u32,
    /// `creature.modelid`: the spawn's display instead of its template's.
    pub display_id: Option<u32>,
    /// `creature.unit_flags`: the spawn's unit flags instead of its template's.
    pub unit_flags: Option<u32>,
}

pub struct WaypointPath {
    pub points: Vec<WaypointPoint>,
}

#[derive(Default)]
pub struct WaypointPoint {
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    /// Facing (WoW radians) applied on arrival when `delay` > 0.
    pub orientation: Option<f32>,
    /// Pause after arriving, in milliseconds.
    pub delay: u32,
    pub move_type: WaypointMoveType,
}

/// `waypoint_data.move_type` (AzerothCore `WaypointMoveType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WaypointMoveType {
    #[default]
    Walk,
    Run,
    Land,
    Takeoff,
}

/// `creature_addon` (per spawn) or `creature_template_addon` (per entry) row.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CreatureAddon {
    /// `waypoint_data.id` walked by MovementType 2 spawns.
    pub path_id: u32,
    /// Mount CreatureDisplayInfo ID.
    pub mount: u32,
    pub auras: Vec<u32>,
}

/// `creature_formations` row for a member other than its leader.
#[derive(Debug, Clone, PartialEq)]
pub struct FormationInfo {
    pub leader_guid: u32,
    pub follow_dist: f32,
    /// Radians (the table stores degrees).
    pub follow_angle: f32,
    pub group_ai: u32,
}

/// `GROUP_AI_FLAG_FOLLOW_LEADER`: the member moves with its patrolling leader.
pub const GROUP_AI_FLAG_FOLLOW_LEADER: u32 = 0x200;

#[derive(Resource, Default)]
pub struct WorldData {
    pub templates: HashMap<u32, CreatureTemplate>,
    pub spawns: Vec<CreatureSpawn>,
    pub spawns_by_map: HashMap<u16, Vec<usize>>,
    /// Keyed by `waypoint_data.id` (path ID, not creature guid).
    pub waypoints: HashMap<u32, WaypointPath>,
    /// `creature_addon` keyed by spawn guid.
    pub addons: HashMap<u32, CreatureAddon>,
    /// `creature_template_addon` keyed by template entry.
    pub template_addons: HashMap<u32, CreatureAddon>,
    /// `creature_formations` keyed by member guid (leaders' own rows omitted).
    pub formations: HashMap<u32, FormationInfo>,
    /// `creature_classlevelstats` basehp0..2 keyed by (level, class).
    pub class_level_base_hp: HashMap<(u8, u8), [u32; 3]>,
    /// `creature_classlevelstats` basemana keyed by (level, class).
    pub class_level_base_mana: HashMap<(u8, u8), u32>,
    pub armor_constants: ArmorConstants,
}

impl WorldData {
    /// AzerothCore `Creature::GetCreatureAddon`: the spawn's row, else its template's.
    pub fn addon_for(&self, spawn: &CreatureSpawn) -> Option<&CreatureAddon> {
        self.addons
            .get(&spawn.guid)
            .or_else(|| self.template_addons.get(&spawn.template_entry))
    }
}

pub fn load_world_data(path: &Path) -> WorldData {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db");

    let expected_stats = load_expected_stats(&conn);
    let mut templates = load_templates(&conn, &expected_stats);
    load_template_models(&conn, &mut templates);
    let spawns = load_spawns(&conn);
    let spawns_by_map = build_spawns_by_map(&spawns);
    let waypoints = load_waypoints(&conn);
    let addons = load_addons(
        &conn,
        "SELECT guid, path_id, mount, auras FROM content_creature_addon",
    );
    let template_addons = load_addons(
        &conn,
        "SELECT entry, path_id, mount, auras FROM content_creature_template_addon",
    );
    let formations = load_formations(&conn);
    let class_level_base_hp = load_class_level_base_hp(&conn);
    let class_level_base_mana = load_class_level_base_mana(&conn);
    let armor_constants = expected_stats.armor_constants();

    info!(
        "WorldData loaded: {} templates, {} spawns, {} maps, {} waypoint paths, {} addons, {} formation members",
        templates.len(),
        spawns.len(),
        spawns_by_map.len(),
        waypoints.len(),
        addons.len() + template_addons.len(),
        formations.len(),
    );

    WorldData {
        templates,
        spawns,
        spawns_by_map,
        waypoints,
        addons,
        template_addons,
        formations,
        class_level_base_hp,
        class_level_base_mana,
        armor_constants,
    }
}

fn creature_stats(row: &Row) -> rusqlite::Result<CreatureStats> {
    Ok(CreatureStats {
        health: row.get(1)?,
        auto_attack_dps: row.get(2)?,
        armor: row.get(3)?,
        armor_constant: row.get(4)?,
    })
}

/// expected_stat, expected_stat_mod and content_tuning_x_expected (DB2Manager::LoadStores).
pub(crate) fn load_expected_stats(conn: &Connection) -> ExpectedStats {
    let mut stats = ExpectedStats::default();
    let mut stmt = conn
        .prepare(
            "SELECT Lvl, CreatureHealth, CreatureAutoAttackDps, CreatureArmor, ArmorConstant, ExpansionID
             FROM expected_stat ORDER BY ID",
        )
        .expect("Failed to prepare expected_stat query");
    let rows = stmt
        .query_map(params![], |row| {
            Ok((
                (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(5)? as i32),
                creature_stats(row)?,
            ))
        })
        .expect("Failed to query expected_stat");
    stats.by_level = rows
        .map(|row| row.expect("Failed to read expected_stat row"))
        .collect();
    let mut stmt = conn
        .prepare("SELECT ID, CreatureHealthMod, CreatureAutoAttackDPSMod, CreatureArmorMod, ArmorConstantMod
                 FROM expected_stat_mod")
        .expect("Failed to prepare expected_stat_mod query");
    let rows = stmt
        .query_map(params![], |row| {
            Ok((row.get::<_, i64>(0)? as u32, creature_stats(row)?))
        })
        .expect("Failed to query expected_stat_mod");
    stats.mods = rows
        .map(|row| row.expect("Failed to read expected_stat_mod row"))
        .collect();
    // Without Mythic+ (active season 0) a row with a minimum season never
    // applies and one with only a maximum season always does (ExpectedStatModReducer).
    let mut stmt = conn
        .prepare(
            "SELECT ContentTuningID, ExpectedStatModID FROM content_tuning_x_expected
             WHERE MinMythicPlusSeasonID = 0 ORDER BY ID",
        )
        .expect("Failed to prepare content_tuning_x_expected query");
    let rows = stmt
        .query_map(params![], |row| {
            Ok((row.get::<_, i64>(0)? as u32, row.get::<_, i64>(1)? as u32))
        })
        .expect("Failed to query content_tuning_x_expected");
    for row in rows {
        let (content_tuning_id, mod_id) =
            row.expect("Failed to read content_tuning_x_expected row");
        stats
            .content_tuning_mods
            .entry(content_tuning_id)
            .or_default()
            .push(mod_id);
    }
    stats
}

/// expected_stat CreatureSpellDamage and CreatureAutoAttackDps (a later ID replaces
/// an earlier one, as `_expectedStatsByLevel`).
pub(crate) fn load_creature_spell_stats(
    conn: &Connection,
) -> crate::level_scaling::CreatureSpellStats {
    let mut stmt = conn
        .prepare(
            "SELECT Lvl, ExpansionID, CreatureSpellDamage, CreatureAutoAttackDps
             FROM expected_stat ORDER BY ID",
        )
        .expect("Failed to prepare expected_stat spell query");
    let rows = stmt
        .query_map(params![], |row| {
            Ok((
                (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as i32),
                (row.get::<_, f64>(2)? as f32, row.get::<_, f64>(3)? as f32),
            ))
        })
        .expect("Failed to query expected_stat");
    let by_level = rows
        .map(|row| row.expect("Failed to read expected_stat row"))
        .collect();
    let mut stmt = conn
        .prepare("SELECT ID, ExpansionID FROM content_tuning")
        .expect("Failed to prepare content_tuning query");
    let tuning_expansion = stmt
        .query_map(params![], |row| {
            Ok((row.get::<_, i64>(0)? as u32, row.get::<_, i64>(1)? as i32))
        })
        .expect("Failed to query content_tuning")
        .map(|row| row.expect("Failed to read content_tuning row"))
        .collect();
    crate::level_scaling::CreatureSpellStats {
        by_level,
        tuning_expansion,
    }
}

fn load_class_level_base_mana(conn: &Connection) -> HashMap<(u8, u8), u32> {
    let mut stmt = conn
        .prepare("SELECT level, class, basemana FROM creature_classlevelstats")
        .expect("Failed to prepare creature_classlevelstats mana query");
    stmt.query_map(params![], |row| {
        Ok((
            (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as u8),
            row.get(2)?,
        ))
    })
    .expect("Failed to query creature_classlevelstats")
    .map(|row| row.expect("Failed to read creature_classlevelstats row"))
    .collect()
}

fn load_class_level_base_hp(conn: &Connection) -> HashMap<(u8, u8), [u32; 3]> {
    let mut stmt = conn
        .prepare("SELECT level, class, basehp0, basehp1, basehp2 FROM creature_classlevelstats")
        .expect("Failed to prepare creature_classlevelstats query");
    stmt.query_map(params![], |row| {
        Ok((
            (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as u8),
            [row.get(2)?, row.get(3)?, row.get(4)?],
        ))
    })
    .expect("Failed to query creature_classlevelstats")
    .map(|row| row.expect("Failed to read creature_classlevelstats row"))
    .collect()
}

const CREATURE_TYPE_CRITTER: i64 = 8;
const CREATURE_FLAG_EXTRA_NO_XP: i64 = 0x40;
const CREATURE_FLAG_EXTRA_TRIGGER: i64 = 0x80;
const UNIT_FLAG2_REGENERATE_POWER: i64 = 0x800;
/// TrinityCore `ObjectMgr::LoadCreatureModelInfo` `trigggerCreatureModelFileID`: the model files of
/// invisible trigger displays.
const TRIGGER_MODEL_FILE_DATA_IDS: &str = "124640, 124641, 124642, 343863, 439302";

fn template_from_row(row: &Row, expected: &ExpectedStats) -> rusqlite::Result<CreatureTemplate> {
    let unit_class = row.get::<_, i64>(14)? as u8;
    let health_modifier = row.get(6)?;
    let armor_modifier = row.get(23)?;
    let content_tuning_id = row.get::<_, i64>(18)? as u32;
    let tuning = (content_tuning_id != 0)
        .then(|| -> rusqlite::Result<TemplateTuning> {
            let inputs = TuningInputs {
                content_tuning_id,
                health_scaling_expansion: row.get::<_, i64>(19)? as i32,
                unit_class,
                health_modifier,
                damage_modifier: row.get(22)?,
                armor_modifier,
            };
            Ok(TemplateTuning {
                content_tuning_id,
                delta: (row.get::<_, i64>(20)? as i8, row.get::<_, i64>(21)? as i8),
                stats: Arc::new(LevelStats::build(expected, &inputs)),
            })
        })
        .transpose()?;
    Ok(CreatureTemplate {
        entry: row.get::<_, i64>(0)? as u32,
        min_level: row.get::<_, i64>(1)? as u8,
        max_level: row.get::<_, i64>(2)? as u8,
        faction: row.get::<_, i64>(3)? as u16,
        npc_flags: row.get::<_, i64>(4)? as u32,
        speed_walk: row.get(5)?,
        health_modifier,
        name: row.get(7)?,
        subname: row.get::<_, Option<String>>(24)?.unwrap_or_default(),
        creature_type: row.get::<_, i64>(10)? as u8,
        rank: row.get::<_, i64>(8)? as u8,
        experience_modifier: row.get(9)?,
        gives_experience: row.get::<_, i64>(10)? != CREATURE_TYPE_CRITTER
            && row.get::<_, i64>(11)? & CREATURE_FLAG_EXTRA_NO_XP == 0,
        speed_run: row.get(12)?,
        script_name: row.get(13)?,
        unit_class,
        expansion: row.get::<_, i64>(15)? as u8,
        detection_range: row.get(16)?,
        aggro_mode: AggroMode::for_template(
            row.get::<_, i64>(10)? as u32,
            row.get::<_, i64>(11)? as u32,
            row.get::<_, i64>(17)? as u32,
            &row.get::<_, String>(13)?,
        ),
        trigger: row.get::<_, i64>(11)? & CREATURE_FLAG_EXTRA_TRIGGER != 0,
        unit_flags: row.get::<_, i64>(17)? as u32,
        models: Vec::new(),
        armor: expected.untuned_armor(row.get::<_, i64>(1)? as u8, unit_class, armor_modifier),
        tuning,
        mana_modifier: row.get(25)?,
        regenerates_power: row.get::<_, i64>(26)? & UNIT_FLAG2_REGENERATE_POWER != 0,
    })
}

pub(crate) fn load_templates(
    conn: &Connection,
    expected: &ExpectedStats,
) -> HashMap<u32, CreatureTemplate> {
    let mut stmt = conn
        .prepare(
            "SELECT entry, minlevel, maxlevel, faction, npcflag, speed_walk, HealthModifier, name,
                    rank, ExperienceModifier, type, flags_extra, speed_run, ScriptName, unit_class, exp,
                    detection_range, unit_flags, ContentTuningID, HealthScalingExpansion,
                    LevelScalingDeltaMin, LevelScalingDeltaMax, DamageModifier, ArmorModifier, subname,
                    ManaModifier, unit_flags2
             FROM content_creature_template",
        )
        .expect("Failed to prepare content_creature_template query");

    let rows = stmt
        .query_map(params![], |row| template_from_row(row, expected))
        .expect("Failed to query content_creature_template");

    let mut map = HashMap::new();
    for row in rows {
        let t = row.expect("Failed to read content_creature_template row");
        map.insert(t.entry, t);
    }
    map
}

pub(crate) fn load_template_models(
    conn: &Connection,
    templates: &mut HashMap<u32, CreatureTemplate>,
) {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT model.CreatureID, model.CreatureDisplayID, model.Probability,
                    COALESCE(data.FileDataID IN ({TRIGGER_MODEL_FILE_DATA_IDS}), 0)
             FROM content_creature_template_model model
             LEFT JOIN creature_display_info display ON display.ID = model.CreatureDisplayID
             LEFT JOIN creature_model_data data ON data.ID = display.ModelID
             ORDER BY model.CreatureID, model.Idx"
        ))
        .expect("Failed to prepare content_creature_template_model query");

    let rows = stmt
        .query_map(params![], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                CreatureModelInfo {
                    display_id: row.get::<_, i64>(1)? as u32,
                    probability: row.get(2)?,
                    is_trigger: row.get(3)?,
                },
            ))
        })
        .expect("Failed to query content_creature_template_model");

    for row in rows {
        let (creature_id, model) = row.expect("Failed to read content_creature_template_model row");
        if let Some(template) = templates.get_mut(&creature_id) {
            template.models.push(model);
        }
    }
}

fn spawn_from_row(row: &Row) -> rusqlite::Result<CreatureSpawn> {
    Ok(CreatureSpawn {
        guid: row.get::<_, i64>(0)? as u32,
        template_entry: row.get::<_, i64>(1)? as u32,
        map: row.get::<_, i64>(2)? as u16,
        zone_id: row.get::<_, i64>(3)? as u16,
        position_x: row.get(4)?,
        position_y: row.get(5)?,
        position_z: row.get(6)?,
        wander_distance: row.get(7)?,
        movement_type: row.get::<_, i64>(8)? as u8,
        orientation: row.get(9)?,
        spawntimesecs: row.get::<_, i64>(10)? as u32,
        display_id: nonzero(row.get(11)?),
        unit_flags: nonzero(row.get(12)?),
    })
}

/// 0 in a spawn override column means the template's value.
fn nonzero(value: i64) -> Option<u32> {
    (value != 0).then_some(value as u32)
}

fn load_spawns(conn: &Connection) -> Vec<CreatureSpawn> {
    // No events are activated yet. Negative memberships spawn while inactive.
    let mut stmt = conn
        .prepare(
            "SELECT guid, id1, map, zoneId, position_x, position_y, position_z,
                    wander_distance, MovementType, orientation, spawntimesecs, modelid, unit_flags
             FROM content_creature creature
             WHERE NOT EXISTS (
                 SELECT 1 FROM content_game_event_creature event WHERE event.guid = creature.guid
             ) OR EXISTS (
                 SELECT 1 FROM content_game_event_creature event
                 WHERE event.guid = creature.guid AND event.eventEntry < 0
             )",
        )
        .expect("Failed to prepare content_creature query");

    let rows = stmt
        .query_map(params![], spawn_from_row)
        .expect("Failed to query content_creature");

    rows.map(|r| r.expect("Failed to read content_creature row"))
        .collect()
}

fn build_spawns_by_map(spawns: &[CreatureSpawn]) -> HashMap<u16, Vec<usize>> {
    let mut map: HashMap<u16, Vec<usize>> = HashMap::new();
    for (idx, spawn) in spawns.iter().enumerate() {
        map.entry(spawn.map).or_default().push(idx);
    }
    map
}

fn waypoint_point_from_row(row: &Row) -> rusqlite::Result<(u32, WaypointPoint)> {
    Ok((
        row.get::<_, i64>(0)? as u32,
        WaypointPoint {
            position_x: row.get(2)?,
            position_y: row.get(3)?,
            position_z: row.get(4)?,
            orientation: row.get(5)?,
            delay: row.get::<_, i64>(6)? as u32,
            move_type: match row.get::<_, i64>(7)? {
                1 => WaypointMoveType::Run,
                2 => WaypointMoveType::Land,
                3 => WaypointMoveType::Takeoff,
                _ => WaypointMoveType::Walk,
            },
        },
    ))
}

fn load_waypoints(conn: &Connection) -> HashMap<u32, WaypointPath> {
    let mut stmt = conn
        .prepare(
            // WaypointMgr::Load skips move_type >= WAYPOINT_MOVE_TYPE_MAX.
            "SELECT id, point, position_x, position_y, position_z, orientation, delay, move_type
             FROM content_waypoint_data
             WHERE move_type < 4
             ORDER BY id, point",
        )
        .expect("Failed to prepare content_waypoint_data query");

    let rows = stmt
        .query_map(params![], waypoint_point_from_row)
        .expect("Failed to query content_waypoint_data");

    let mut map: HashMap<u32, WaypointPath> = HashMap::new();
    for row in rows {
        let (path_id, point) = row.expect("Failed to read content_waypoint_data row");
        map.entry(path_id)
            .or_insert_with(|| WaypointPath { points: Vec::new() })
            .points
            .push(point);
    }
    map
}

fn addon_from_row(row: &Row) -> rusqlite::Result<(u32, CreatureAddon)> {
    let auras: Option<String> = row.get(3)?;
    Ok((
        row.get::<_, i64>(0)? as u32,
        CreatureAddon {
            path_id: row.get::<_, i64>(1)? as u32,
            mount: row.get::<_, i64>(2)? as u32,
            auras: parse_addon_auras(auras.as_deref().unwrap_or("")),
        },
    ))
}

/// Space-separated spell IDs; ObjectMgr::LoadCreatureAddons drops unparseable
/// tokens and duplicates.
fn parse_addon_auras(text: &str) -> Vec<u32> {
    let mut auras = Vec::new();
    for spell_id in text
        .split(' ')
        .filter_map(|token| token.parse::<u32>().ok())
    {
        if !auras.contains(&spell_id) {
            auras.push(spell_id);
        }
    }
    auras
}

fn load_addons(conn: &Connection, sql: &str) -> HashMap<u32, CreatureAddon> {
    let mut stmt = conn
        .prepare(sql)
        .expect("Failed to prepare creature addon query");
    let rows = stmt
        .query_map(params![], addon_from_row)
        .expect("Failed to query creature addons");
    rows.map(|r| r.expect("Failed to read creature addon row"))
        .collect()
}

fn formation_from_row(row: &Row) -> rusqlite::Result<(u32, FormationInfo)> {
    Ok((
        row.get::<_, i64>(1)? as u32,
        FormationInfo {
            leader_guid: row.get::<_, i64>(0)? as u32,
            follow_dist: row.get(2)?,
            follow_angle: row.get::<_, f32>(3)?.to_radians(),
            group_ai: row.get::<_, i64>(4)? as u32,
        },
    ))
}

fn load_formations(conn: &Connection) -> HashMap<u32, FormationInfo> {
    let mut stmt = conn
        .prepare(
            // A leader joins its group only through its own row
            // (CreatureGroup::AddMember), so groups without one never move.
            "SELECT leaderGUID, memberGUID, dist, angle, groupAI
             FROM content_creature_formations member
             WHERE leaderGUID <> memberGUID AND EXISTS (
                 SELECT 1 FROM content_creature_formations leader
                 WHERE leader.memberGUID = member.leaderGUID
                   AND leader.leaderGUID = member.leaderGUID
             )",
        )
        .expect("Failed to prepare content_creature_formations query");
    let rows = stmt
        .query_map(params![], formation_from_row)
        .expect("Failed to query content_creature_formations");
    rows.map(|r| r.expect("Failed to read content_creature_formations row"))
        .collect()
}

fn startup_load_world_data(mut commands: Commands) {
    let path = Path::new("data/world.db");
    let data = load_world_data(path);
    commands.insert_resource(data.armor_constants.clone());
    commands.insert_resource(data);
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("Failed to open world.db");
    let templates = crate::faction::load_faction_templates(&conn)
        .expect("Failed to load faction_template; run scripts/db2.py import");
    info!("Loaded {} faction templates", templates.0.len());
    commands.insert_resource(templates);
    let gossip =
        crate::npc_interaction::load_gossip_catalog(&conn).expect("Failed to load gossip menus");
    commands.insert_resource(gossip);
    let vendors =
        crate::merchant::load_vendor_catalog(&conn).expect("Failed to load content_npc_vendor");
    info!("Loaded {} vendor lists", vendors.vendors.len());
    commands.insert_resource(vendors);
    let professions = crate::profession::load_profession_data(&conn)
        .expect("Failed to load profession data; run scripts/import-azerothcore.sh");
    commands.insert_resource(professions);
    let loot = crate::loot::load_loot_tables(&conn)
        .expect("Failed to load content_creature_loot_template; run scripts/import-azerothcore.sh");
    info!("Loaded loot tables for {} creatures", loot.creature.len());
    commands.insert_resource(loot);
    let taxi = crate::taxi::load_taxi_network(&conn)
        .expect("Failed to load taxi_nodes/taxi_path/taxi_path_node; run scripts/db2.py import");
    info!("Loaded {} flight points", taxi.nodes.len());
    commands.insert_resource(taxi);
    let durability = crate::durability::load_item_durability(&conn)
        .expect("Failed to load item durability; run scripts/db2.py import");
    info!("Loaded durability for {} items", durability.0.len());
    commands.insert_resource(durability);
    let game_objects = crate::game_objects::load_game_object_spawns(&conn)
        .expect("Failed to load content_gameobject; run scripts/import-azerothcore.sh");
    info!("Loaded {} game object spawns", game_objects.0.len());
    commands.insert_resource(game_objects);
    let meeting_stones = crate::meeting_stone::load_meeting_stone_data(&conn)
        .expect("Failed to load meeting stone and ritual templates");
    commands.insert_resource(meeting_stones);
    let maps = crate::maps::store::load_map_store(&conn)
        .expect("Failed to load map data; run scripts/import-azerothcore.sh");
    info!(
        "Loaded {} maps, area trigger teleports on {} maps",
        maps.maps.len(),
        maps.area_triggers.len()
    );
    commands.insert_resource(maps);
    let prices = crate::bank::load_bank_tab_prices(&conn)
        .expect("Failed to load bank_tab; run scripts/db2.py import");
    commands.insert_resource(prices);
    commands.insert_resource(load_creature_spell_stats(&conn));
}

pub struct WorldDataPlugin;

impl Plugin for WorldDataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup_load_world_data);
    }
}
