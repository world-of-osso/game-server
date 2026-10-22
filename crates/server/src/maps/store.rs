//! Map data read at startup: `Map.db2` / `MapDifficulty.db2` entries and their
//! `MapDifficultyXCondition.db2` level conditions, area trigger teleports (`AreaTrigger.db2` shapes + TDB `areatrigger_teleport` →
//! `world_safe_locs`) and graveyards (`graveyard_zone`).

use std::collections::HashMap;

use bevy::prelude::*;
use rusqlite::{Connection, Row};

use super::WorldLocation;

/// `Map.db2` InstanceType (TrinityCore `MapTypes`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceType {
    /// `MAP_COMMON`: continents and other world maps.
    Common,
    /// `MAP_INSTANCE`: five-player dungeons.
    Party,
    Raid,
    Battleground,
    Arena,
    Scenario,
    /// Retail Map.db2 type 7: Home Interior (map 2783).
    House,
    /// Retail Map.db2 type 8: Founder's Point and Razorwind Shores.
    Neighborhood,
}

impl InstanceType {
    fn from_db2(value: u32) -> Option<Self> {
        Some(match value {
            0 => Self::Common,
            1 => Self::Party,
            2 => Self::Raid,
            3 => Self::Battleground,
            4 => Self::Arena,
            5 => Self::Scenario,
            7 => Self::House,
            8 => Self::Neighborhood,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapEntry {
    pub id: u32,
    /// `Directory`, lowercased: `world/maps/<directory>/`.
    pub directory: String,
    pub name: String,
    pub instance_type: InstanceType,
    /// `AreaTableID`: the area of positions the terrain names none for
    /// (TrinityCore `TerrainInfo::GetAreaId`), the zone of a WMO-only dungeon.
    pub area_table_id: u32,
    pub parent_map_id: i32,
    /// `CorpseMapID` and `Corpse`: where the dungeon's entrance lies on its parent map.
    pub corpse_map_id: i32,
    pub corpse: (f32, f32),
    pub max_players: u32,
    pub wdt_file_data_id: u32,
    pub expansion_id: u32,
}

impl MapEntry {
    /// TrinityCore `MapEntry::IsDungeon` (garrisons are not imported).
    pub fn is_dungeon(&self) -> bool {
        matches!(
            self.instance_type,
            InstanceType::Party | InstanceType::Raid | InstanceType::Scenario
        )
    }
}

/// `MapDifficulty.ResetInterval` (`MapDifficultyResetInterval`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetInterval {
    /// `MAP_DIFFICULTY_RESET_ANYTIME`: no reset schedule, no instance lock.
    Anytime,
    Daily,
    Weekly,
}

impl ResetInterval {
    fn from_db2(value: u32) -> Self {
        match value {
            1 => Self::Daily,
            2 => Self::Weekly,
            _ => Self::Anytime,
        }
    }
}

/// `MapDifficultyFlags::UseLootBasedLockInsteadOfInstanceLock`: locks per encounter.
const MAP_DIFFICULTY_FLAG_ENCOUNTER_LOCKS: u32 = 0x02;
/// `MapDifficultyFlags::DisableLockExtension`.
const MAP_DIFFICULTY_FLAG_DISABLE_LOCK_EXTENSION: u32 = 0x10;

/// `MapDifficulty.db2` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapDifficulty {
    pub id: u32,
    pub map_id: u32,
    pub difficulty_id: u32,
    /// `LockID`: difficulties of a map sharing it share instance locks (`InstanceLockKey`).
    pub lock_id: u32,
    pub reset_interval: ResetInterval,
    pub max_players: u32,
    pub flags: u32,
    /// Map.Flags_0 FlexibleRaidLocking; not a MapDifficulty flag.
    pub flexible_locking: bool,
}

impl MapDifficulty {
    /// `MapDifficultyEntry::HasResetSchedule`.
    pub fn has_reset_schedule(&self) -> bool {
        self.reset_interval != ResetInterval::Anytime
    }

    /// `MapDifficultyEntry::IsUsingEncounterLocks`.
    pub fn is_using_encounter_locks(&self) -> bool {
        self.flags & MAP_DIFFICULTY_FLAG_ENCOUNTER_LOCKS != 0
    }

    /// `MapDb2Entries::IsInstanceIdBound`: flexible raid and loot-based encounter locks
    /// resume individual progress rather than binding the player to one copy.
    pub fn is_instance_id_bound(&self) -> bool {
        !self.flexible_locking && !self.is_using_encounter_locks()
    }

    /// `MapDifficultyEntry::IsExtendable`.
    pub fn is_extendable(&self) -> bool {
        self.flags & MAP_DIFFICULTY_FLAG_DISABLE_LOCK_EXTENSION == 0
    }

    /// `MapDifficultyEntry::GetRaidDuration`, seconds.
    pub fn raid_duration_secs(&self) -> i64 {
        match self.reset_interval {
            ResetInterval::Anytime => 0,
            ResetInterval::Daily => 86_400,
            ResetInterval::Weekly => 604_800,
        }
    }

    /// `MapDb2Entries::GetKey`.
    pub fn lock_key(&self) -> (u32, u32) {
        (self.map_id, self.lock_id)
    }
}

/// `PlayerConditionFlags::WithinOrAboveRecord`: the ContentTuning maximum level does not
/// apply.
const PLAYER_CONDITION_WITHIN_OR_ABOVE_RECORD: u32 = 0x20;
/// `PlayerConditionFlags::Disabled`: the condition is always met.
const PLAYER_CONDITION_DISABLED: u32 = 0x100;

/// A `MapDifficultyXCondition.db2` row whose `PlayerCondition` names a ContentTuning: the
/// level range `ConditionMgr::IsPlayerMeetingCondition` (ConditionMgr.cpp:2929-2953)
/// requires. The other `PlayerCondition` fields (ModifierTree: boosted characters, class
/// trials, Timerunners) are not evaluated; `PlayerCondition.MinLevel` is ignored as
/// TrinityCore does.
#[derive(Debug, Clone, PartialEq)]
pub struct MapDifficultyCondition {
    /// `FailureDescription`, else the `PlayerCondition`'s `Failure_description`.
    pub failure_text: String,
    /// ContentTuning `MinLevel` (`content_tuning_level`).
    pub min_level: u8,
    /// ContentTuning `MaxLevel`; 0 under `WithinOrAboveRecord`.
    pub max_level: u8,
}

impl MapDifficultyCondition {
    pub fn is_met(&self, level: u8) -> bool {
        level >= self.min_level && (self.max_level == 0 || level <= self.max_level)
    }

    /// The failure text with `$pcMinLevel` resolved to the required level.
    pub fn failure(&self) -> String {
        self.failure_text
            .replace("$pcMinLevel", &self.min_level.to_string())
    }
}

/// `DIFFICULTY_NORMAL`, the player's default dungeon difficulty
/// (`Player::GetDungeonDifficultyID` of a new character).
pub const DIFFICULTY_NORMAL: u32 = 1;

/// `DifficultyFlags::DIFFICULTY_FLAG_DEFAULT`.
const DIFFICULTY_FLAG_DEFAULT: u32 = 0x02;
/// `DifficultyFlags::DIFFICULTY_FLAG_CAN_SELECT`: selectable in the difficulty menu.
const DIFFICULTY_FLAG_CAN_SELECT: u32 = 0x04;

/// `Difficulty.db2` row.
#[derive(Debug, Clone, PartialEq)]
pub struct DifficultyEntry {
    pub id: u32,
    pub name: String,
    pub instance_type: InstanceType,
    /// `FallbackDifficultyID`: the difficulty used where a map or template has none of
    /// this one (Mythic → Heroic → Normal → None).
    pub fallback: u32,
    pub flags: u32,
}

impl DifficultyEntry {
    pub fn is_legacy(&self) -> bool {
        self.flags & 0x20 != 0
    }

    pub fn can_select(&self) -> bool {
        self.flags & DIFFICULTY_FLAG_CAN_SELECT != 0
    }
}

/// `DungeonEncounter.db2` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonEncounter {
    pub id: u32,
    pub name: String,
    pub flags: u32,
    pub map_id: u32,
    /// -1 or 0: every difficulty of the map.
    pub difficulty_id: i32,
    /// Authored encounter display/run order, independent of script boss indices.
    pub order_index: i32,
    /// The lock's `CompletedEncountersMask` bit.
    pub bit: u32,
}

/// `AreaTrigger.db2` ShapeType of the teleport and quest triggers (sphere and box only).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AreaTriggerShape {
    Sphere {
        radius: f32,
    },
    /// Half extents along the box's yaw-rotated axes (`Box_length / 2` …).
    Box {
        half_length: f32,
        half_width: f32,
        half_height: f32,
        yaw: f32,
    },
}

/// `AreaTriggerActionSetFlag::AllowWhileGhost`.
const ACTION_SET_ALLOW_WHILE_GHOST: u32 = 0x8;

/// An `AreaTrigger.db2` row with an `areatrigger_teleport` target.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerTeleport {
    pub id: u32,
    pub map_id: u32,
    /// WoW coordinates.
    pub center: Vec3,
    pub shape: AreaTriggerShape,
    pub allow_while_ghost: bool,
    pub target: WorldLocation,
}

impl AreaTriggerShape {
    /// `ShapeType` 0 (sphere) or 1 (box) from the `AreaTrigger.db2` columns `Radius` and
    /// `Box_length/width/height/yaw`; other shapes are not modelled.
    pub(crate) fn from_db2(
        shape_type: u32,
        radius: f32,
        [length, width, height, yaw]: [f32; 4],
    ) -> Option<Self> {
        match shape_type {
            0 => Some(Self::Sphere { radius }),
            1 => Some(Self::Box {
                half_length: length / 2.0,
                half_width: width / 2.0,
                half_height: height / 2.0,
                yaw,
            }),
            _ => None,
        }
    }

    /// TrinityCore `Player::IsInAreaTrigger` shape test of a trigger at `center` at
    /// `point` (WoW coordinates): `IsInDist` for a sphere, `Position::IsWithinBox` for a box.
    pub fn contains(&self, center: Vec3, point: Vec3) -> bool {
        match *self {
            Self::Sphere { radius } => point.distance_squared(center) < radius * radius,
            Self::Box {
                half_length,
                half_width,
                half_height,
                yaw,
            } => {
                let rotation = std::f64::consts::TAU - f64::from(yaw);
                let (sin, cos) = rotation.sin_cos();
                let dx = f64::from(point.x - center.x);
                let dy = f64::from(point.y - center.y);
                let rot_x = (dx * cos - dy * sin) as f32;
                let rot_y = (dy * cos + dx * sin) as f32;
                rot_x.abs() <= half_length
                    && rot_y.abs() <= half_width
                    && (point.z - center.z).abs() <= half_height
            }
        }
    }
}

impl AreaTriggerTeleport {
    pub fn contains(&self, point: Vec3) -> bool {
        self.shape.contains(self.center, point)
    }
}

/// Map data (`sMapStore`, `sMapDifficultyStore`, `ObjectMgr` area triggers and
/// graveyards).
#[derive(Resource, Debug, Default, Clone)]
pub struct MapStore {
    pub maps: HashMap<u32, MapEntry>,
    /// `MapDifficulty.db2` by map, in `ID` order.
    pub difficulties: HashMap<u32, Vec<MapDifficulty>>,
    /// Level conditions by `MapDifficulty.db2` ID, in `OrderIndex` order
    /// (`DB2Manager::GetMapDifficultyConditions`).
    pub difficulty_conditions: HashMap<u32, Vec<MapDifficultyCondition>>,
    /// `Difficulty.db2` by ID.
    pub difficulty_entries: HashMap<u32, DifficultyEntry>,
    /// `DungeonEncounter.db2` by ID.
    pub dungeon_encounters: HashMap<u32, DungeonEncounter>,
    /// Unambiguous CASC journal → TDB map/display joins: (map, creature) → encounter bit.
    pub raid_creature_encounters: HashMap<(u32, u32), u32>,
    pub area_triggers: HashMap<u32, Vec<AreaTriggerTeleport>>,
    pub(crate) world_area_triggers: crate::world_area_triggers::Catalog,
    pub safe_locs: HashMap<u32, WorldLocation>,
    /// Nonzero WorldSafeLocs.TransportSpawnId: location coordinates are ship-relative.
    pub safe_loc_transports: HashMap<u32, u32>,
    /// `graveyard_zone`: ghost zone → WorldSafeLocs ids.
    pub graveyard_zones: HashMap<u32, Vec<u32>>,
    /// `instance_template.parent` by dungeon map.
    pub instance_parents: HashMap<u32, u32>,
    /// `instance_template.script` by dungeon map (`InstanceMap::CreateInstanceData`).
    pub instance_scripts: HashMap<u32, String>,
}

impl MapStore {
    pub fn map(&self, map_id: u32) -> Option<&MapEntry> {
        self.maps.get(&map_id)
    }

    /// `DB2Manager::GetMapDifficultyData`.
    pub fn map_difficulty(&self, map_id: u32, difficulty_id: u32) -> Option<&MapDifficulty> {
        self.difficulties
            .get(&map_id)?
            .iter()
            .find(|difficulty| difficulty.difficulty_id == difficulty_id)
    }

    /// The first level condition of MapDifficulty `map_difficulty_id` a player at `level`
    /// fails (`Player::Satisfy`, Player.cpp:20071-20082).
    pub fn unmet_condition(
        &self,
        map_difficulty_id: u32,
        level: u8,
    ) -> Option<&MapDifficultyCondition> {
        self.difficulty_conditions
            .get(&map_difficulty_id)?
            .iter()
            .find(|condition| !condition.is_met(level))
    }

    /// `DB2Manager::GetDownscaledMapDifficultyData` (DB2Stores.cpp:2778-2797): the map's
    /// row for `difficulty_id`, else for its `FallbackDifficultyID` chain, else the map's
    /// default difficulty (`DIFFICULTY_FLAG_DEFAULT`, else its first one).
    pub fn downscaled_map_difficulty(
        &self,
        map_id: u32,
        difficulty_id: u32,
    ) -> Option<&MapDifficulty> {
        let mut current = difficulty_id;
        loop {
            if let Some(found) = self.map_difficulty(map_id, current) {
                return Some(found);
            }
            if current == 0 {
                break;
            }
            current = self
                .difficulty_entries
                .get(&current)
                .map_or(0, |entry| entry.fallback);
        }
        self.default_map_difficulty(map_id)
    }

    /// `DB2Manager::GetDefaultMapDifficulty` (DB2Stores.cpp:2734-2765).
    pub(crate) fn default_map_difficulty(&self, map_id: u32) -> Option<&MapDifficulty> {
        let rows = self.difficulties.get(&map_id)?;
        let known = || {
            rows.iter()
                .filter(|row| self.difficulty_entries.contains_key(&row.difficulty_id))
        };
        known()
            .find(|row| {
                self.difficulty_entries[&row.difficulty_id].flags & DIFFICULTY_FLAG_DEFAULT != 0
            })
            .or_else(|| known().next())
    }

    /// `DungeonEncounter.db2` row `encounter_id` of `map_id` at `difficulty_id`
    /// (`BossInfo::GetDungeonEncounterForDifficulty`: a row for every difficulty is -1/0).
    pub fn dungeon_encounter(
        &self,
        encounter_id: u32,
        map_id: u32,
        difficulty_id: u32,
    ) -> Option<&DungeonEncounter> {
        self.dungeon_encounters
            .get(&encounter_id)
            .filter(|encounter| {
                encounter.map_id == map_id
                    && (encounter.difficulty_id <= 0
                        || encounter.difficulty_id as u32 == difficulty_id)
            })
    }

    pub fn encounters_for_map(&self, map_id: u32, difficulty_id: u32) -> Vec<&DungeonEncounter> {
        let mut encounters: Vec<_> = self
            .dungeon_encounters
            .values()
            .filter(|encounter| {
                encounter.map_id == map_id
                    && (encounter.difficulty_id <= 0
                        || encounter.difficulty_id as u32 == difficulty_id)
            })
            .collect();
        encounters.sort_by_key(|encounter| (encounter.order_index, encounter.id));
        encounters
    }

    pub fn area_triggers_on(&self, map_id: u32) -> &[AreaTriggerTeleport] {
        self.area_triggers.get(&map_id).map_or(&[], Vec::as_slice)
    }

    /// `ObjectMgr::GetGoBackTrigger` (ObjectMgr.cpp:7268-7290): the target of a trigger on
    /// `map_id` leading to its parent (`instance_template.parent`, else `CorpseMapID`).
    pub fn go_back_location(&self, map_id: u32) -> Option<WorldLocation> {
        let entry = self.map(map_id)?;
        if entry.corpse_map_id < 0 {
            return None;
        }
        let parent = entry
            .is_dungeon()
            .then(|| self.instance_parents.get(&map_id).copied())
            .flatten()
            .unwrap_or(entry.corpse_map_id as u32);
        self.area_triggers_on(map_id)
            .iter()
            .map(|trigger| trigger.target)
            .find(|target| target.map_id == parent)
    }

    /// `ObjectMgr::GetMapEntranceTrigger` (ObjectMgr.cpp:7295-7306): the target of the
    /// first trigger leading into `map_id` (lowest trigger id).
    pub fn entrance_location(&self, map_id: u32) -> Option<WorldLocation> {
        self.area_triggers
            .values()
            .flatten()
            .filter(|trigger| trigger.target.map_id == map_id)
            .min_by_key(|trigger| trigger.id)
            .map(|trigger| trigger.target)
    }
}

impl MapStore {
    /// Maps 0 and 1 as `Map.db2` lists them, for tests without world.db.
    #[cfg(test)]
    pub fn continents() -> Self {
        let continent = |id: u32, directory: &str, name: &str, wdt_file_data_id: u32| MapEntry {
            id,
            directory: directory.into(),
            name: name.into(),
            instance_type: InstanceType::Common,
            area_table_id: 0,
            parent_map_id: -1,
            corpse_map_id: -1,
            corpse: (0.0, 0.0),
            max_players: 0,
            wdt_file_data_id,
            expansion_id: 0,
        };
        Self {
            maps: HashMap::from([
                (0, continent(0, "azeroth", "Eastern Kingdoms", 775_971)),
                (1, continent(1, "kalimdor", "Kalimdor", 782_779)),
            ]),
            ..Default::default()
        }
    }
}

pub fn load_map_store(conn: &Connection) -> rusqlite::Result<MapStore> {
    let safe_locs = load_safe_locs(conn)?;
    Ok(MapStore {
        maps: load_maps(conn)?,
        world_area_triggers: crate::world_area_triggers::load_catalog(conn)?,
        difficulties: load_difficulties(conn)?,
        difficulty_conditions: load_difficulty_conditions(conn)?,
        difficulty_entries: load_difficulty_entries(conn)?,
        dungeon_encounters: load_dungeon_encounters(conn)?,
        raid_creature_encounters: load_raid_creature_encounters(conn)?,
        area_triggers: load_area_triggers(conn, &safe_locs)?,
        graveyard_zones: load_graveyard_zones(conn)?,
        instance_parents: load_instance_parents(conn)?,
        instance_scripts: load_instance_scripts(conn)?,
        safe_locs,
        safe_loc_transports: load_safe_loc_transports(conn)?,
    })
}

fn load_raid_creature_encounters(conn: &Connection) -> rusqlite::Result<HashMap<(u32, u32), u32>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='raid_encounter_creature')",
        [],
        |row| row.get(0),
    )?;
    let mut bindings = if exists {
        let mut statement =
            conn.prepare("SELECT map_id, creature_entry, bit FROM raid_encounter_creature")?;
        statement
            .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?
    } else {
        warn!(
            "Raid boss credit catalog absent; import source bindings before raid boss credit is available"
        );
        HashMap::new()
    };
    load_bossmod_bindings(conn, &mut bindings)?;
    Ok(bindings)
}

fn load_bossmod_bindings(
    conn: &Connection,
    bindings: &mut HashMap<(u32, u32), u32>,
) -> rusqlite::Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='bossmod_binding')",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(());
    }
    let mut statement = conn.prepare("SELECT map_id,npc_entry,bit FROM bossmod_binding")?;
    let rows = statement.query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))?;
    for row in rows {
        let (key, bit) = row?;
        if let Some(existing) = bindings.insert(key, bit) {
            if existing != bit {
                return Err(rusqlite::Error::InvalidQuery);
            }
        }
    }
    Ok(())
}

fn load_maps(conn: &Connection) -> rusqlite::Result<HashMap<u32, MapEntry>> {
    let mut statement = conn.prepare(
        "SELECT ID, Directory, MapName_lang, InstanceType, AreaTableID, ParentMapID,
                CorpseMapID, Corpse_0, Corpse_1, MaxPlayers, WdtFileDataID, ExpansionID FROM map",
    )?;
    let rows = statement.query_map([], map_from_row)?;
    let mut maps = HashMap::new();
    for row in rows {
        if let Some(entry) = row? {
            maps.insert(entry.id, entry);
        }
    }
    Ok(maps)
}

fn map_from_row(row: &Row) -> rusqlite::Result<Option<MapEntry>> {
    let id: u32 = row.get(0)?;
    let instance_type: u32 = row.get(3)?;
    let Some(instance_type) = InstanceType::from_db2(instance_type) else {
        warn!("Map {id} has unknown InstanceType {instance_type}; not loaded");
        return Ok(None);
    };
    Ok(Some(MapEntry {
        id,
        directory: row.get::<_, String>(1)?.to_ascii_lowercase(),
        name: row.get(2)?,
        instance_type,
        area_table_id: row.get(4)?,
        parent_map_id: row.get(5)?,
        corpse_map_id: row.get(6)?,
        corpse: (row.get::<_, f64>(7)? as f32, row.get::<_, f64>(8)? as f32),
        max_players: row.get(9)?,
        wdt_file_data_id: row.get(10)?,
        expansion_id: row.get(11)?,
    }))
}

fn load_difficulties(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<MapDifficulty>>> {
    let mut statement = conn.prepare(
        "SELECT d.ID, d.MapID, d.DifficultyID, d.LockID, d.ResetInterval, d.MaxPlayers, d.Flags, m.Flags_0
         FROM map_difficulty d JOIN map m ON m.ID=d.MapID ORDER BY d.ID",
    )?;
    let mut difficulties: HashMap<u32, Vec<MapDifficulty>> = HashMap::new();
    let rows = statement.query_map([], |row| {
        Ok(MapDifficulty {
            id: row.get(0)?,
            map_id: row.get(1)?,
            difficulty_id: row.get(2)?,
            lock_id: row.get(3)?,
            reset_interval: ResetInterval::from_db2(row.get(4)?),
            max_players: row.get(5)?,
            flags: row.get(6)?,
            flexible_locking: (row.get::<_, i64>(7)? as u32) & 0x8000 != 0,
        })
    })?;
    for row in rows {
        let difficulty = row?;
        difficulties
            .entry(difficulty.map_id)
            .or_default()
            .push(difficulty);
    }
    Ok(difficulties)
}

/// Conditions whose `PlayerCondition` has a ContentTuning level range; a condition
/// without one, or naming no `PlayerCondition` row, is met (`IsPlayerMeetingCondition`
/// finds nothing to check).
fn load_difficulty_conditions(
    conn: &Connection,
) -> rusqlite::Result<HashMap<u32, Vec<MapDifficultyCondition>>> {
    let mut statement = conn.prepare(
        "SELECT x.MapDifficultyID, x.FailureDescription_lang, p.Failure_description_lang,
                p.Flags, l.minlevel, l.maxlevel
         FROM map_difficulty_x_condition x
         JOIN player_condition p ON p.ID = x.PlayerConditionID
         JOIN content_tuning_level l ON l.ID = p.ContentTuningID
         WHERE p.Flags & ?1 = 0
         ORDER BY x.MapDifficultyID, x.OrderIndex",
    )?;
    let rows = statement.query_map([PLAYER_CONDITION_DISABLED], |row| {
        let own_text: String = row.get(1)?;
        let flags: u32 = row.get(3)?;
        let max_level: u8 = row.get(5)?;
        Ok((
            row.get::<_, u32>(0)?,
            MapDifficultyCondition {
                failure_text: if own_text.is_empty() {
                    row.get(2)?
                } else {
                    own_text
                },
                min_level: row.get(4)?,
                max_level: if flags & PLAYER_CONDITION_WITHIN_OR_ABOVE_RECORD != 0 {
                    0
                } else {
                    max_level
                },
            },
        ))
    })?;
    let mut conditions: HashMap<u32, Vec<MapDifficultyCondition>> = HashMap::new();
    for row in rows {
        let (map_difficulty_id, condition) = row?;
        conditions
            .entry(map_difficulty_id)
            .or_default()
            .push(condition);
    }
    Ok(conditions)
}

fn load_difficulty_entries(conn: &Connection) -> rusqlite::Result<HashMap<u32, DifficultyEntry>> {
    let mut statement = conn.prepare(
        "SELECT ID, Name_lang, InstanceType, FallbackDifficultyID, Flags FROM difficulty",
    )?;
    let rows = statement.query_map([], |row| {
        let id: u32 = row.get(0)?;
        let name: String = row.get(1)?;
        let instance_type: u32 = row.get(2)?;
        let fallback: u32 = row.get(3)?;
        let flags: u32 = row.get(4)?;
        Ok(InstanceType::from_db2(instance_type).map(|instance_type| {
            (
                id,
                DifficultyEntry {
                    id,
                    name,
                    instance_type,
                    fallback,
                    flags,
                },
            )
        }))
    })?;
    let mut entries = HashMap::new();
    for row in rows {
        if let Some((id, entry)) = row? {
            entries.insert(id, entry);
        }
    }
    Ok(entries)
}

fn load_dungeon_encounters(conn: &Connection) -> rusqlite::Result<HashMap<u32, DungeonEncounter>> {
    let mut statement =
        conn.prepare("SELECT ID, MapID, DifficultyID, Bit, OrderIndex, Name_lang, Flags FROM dungeon_encounter WHERE Bit BETWEEN 0 AND 31")?;
    let rows = statement.query_map([], |row| {
        Ok(DungeonEncounter {
            id: row.get(0)?,
            map_id: row.get(1)?,
            difficulty_id: row.get(2)?,
            bit: row.get(3)?,
            order_index: row.get(4)?,
            name: row.get(5)?,
            flags: row.get(6)?,
        })
    })?;
    let encounters: HashMap<_, _> = rows
        .map(|row| row.map(|encounter| (encounter.id, encounter)))
        .collect::<rusqlite::Result<_>>()?;
    validate_encounter_counts(&encounters)?;
    Ok(encounters)
}

fn validate_encounter_counts(encounters: &HashMap<u32, DungeonEncounter>) -> rusqlite::Result<()> {
    let mut counts = HashMap::<(u32, i32), usize>::new();
    for encounter in encounters.values() {
        *counts
            .entry((encounter.map_id, encounter.difficulty_id.max(0)))
            .or_default() += 1;
    }
    for (&(map, difficulty), &count) in &counts {
        let shared_count = if difficulty == 0 {
            0
        } else {
            counts.get(&(map, 0)).copied().unwrap_or(0)
        };
        if count + shared_count > shared::protocol::DUNGEON_ENCOUNTER_MAX_ROWS {
            error!(
                map,
                difficulty,
                count,
                shared_count,
                "DungeonEncounter objective list exceeds protocol bound"
            );
            return Err(rusqlite::Error::InvalidQuery);
        }
    }
    Ok(())
}

/// `ObjectMgr::LoadWorldSafeLocs`: `Facing` is stored in degrees.
fn load_safe_locs(conn: &Connection) -> rusqlite::Result<HashMap<u32, WorldLocation>> {
    let mut statement =
        conn.prepare("SELECT ID, MapID, LocX, LocY, LocZ, Facing FROM tdb_world_safe_locs")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            WorldLocation {
                map_id: row.get(1)?,
                position: Vec3::new(
                    row.get::<_, f64>(2)? as f32,
                    row.get::<_, f64>(3)? as f32,
                    row.get::<_, f64>(4)? as f32,
                ),
                orientation: (row.get::<_, f64>(5)? as f32).to_radians(),
            },
        ))
    })?;
    rows.collect()
}

fn load_safe_loc_transports(conn: &Connection) -> rusqlite::Result<HashMap<u32, u32>> {
    let mut statement = conn.prepare("SELECT ID, TransportSpawnId FROM tdb_world_safe_locs WHERE TransportSpawnId IS NOT NULL AND TransportSpawnId <> 0")?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}

const AREA_TRIGGER_QUERY: &str = "SELECT a.ID, a.ContinentID, a.Pos_0, a.Pos_1, a.Pos_2, \
     a.ShapeType, a.Radius, a.Box_length, a.Box_width, a.Box_height, a.Box_yaw, \
     COALESCE(s.Flags, 0), t.PortLocID \
     FROM tdb_areatrigger_teleport t \
     JOIN area_trigger a ON a.ID = t.ID \
     LEFT JOIN area_trigger_action_set s ON s.ID = a.AreaTriggerActionSetID \
     ORDER BY a.ID";

/// `ObjectMgr::LoadAreaTriggerTeleports`; rows naming a missing trigger or port
/// location are skipped (TrinityCore logs and skips them too).
fn load_area_triggers(
    conn: &Connection,
    safe_locs: &HashMap<u32, WorldLocation>,
) -> rusqlite::Result<HashMap<u32, Vec<AreaTriggerTeleport>>> {
    let mut statement = conn.prepare(AREA_TRIGGER_QUERY)?;
    let rows = statement.query_map([], |row| {
        let float =
            |index: usize| -> rusqlite::Result<f32> { Ok(row.get::<_, f64>(index)? as f32) };
        let shape = AreaTriggerShape::from_db2(
            row.get(5)?,
            float(6)?,
            [float(7)?, float(8)?, float(9)?, float(10)?],
        );
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            Vec3::new(float(2)?, float(3)?, float(4)?),
            shape,
            row.get::<_, u32>(11)?,
            row.get::<_, u32>(12)?,
        ))
    })?;
    let mut triggers: HashMap<u32, Vec<AreaTriggerTeleport>> = HashMap::new();
    for row in rows {
        let (id, map_id, center, shape, action_flags, port_loc) = row?;
        let Some(shape) = shape else {
            warn!("Area trigger {id} teleport has an unsupported shape; skipped");
            continue;
        };
        let Some(&target) = safe_locs.get(&port_loc) else {
            warn!("Area trigger {id} names missing world_safe_locs {port_loc}; skipped");
            continue;
        };
        triggers
            .entry(map_id)
            .or_default()
            .push(AreaTriggerTeleport {
                id,
                map_id,
                center,
                shape,
                allow_while_ghost: action_flags & ACTION_SET_ALLOW_WHILE_GHOST != 0,
                target,
            });
    }
    Ok(triggers)
}

fn load_graveyard_zones(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<u32>>> {
    let mut statement =
        conn.prepare("SELECT GhostZone, ID FROM tdb_graveyard_zone ORDER BY GhostZone, ID")?;
    let mut zones: HashMap<u32, Vec<u32>> = HashMap::new();
    let rows = statement.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
    for row in rows {
        let (zone, safe_loc) = row?;
        zones.entry(zone).or_default().push(safe_loc);
    }
    Ok(zones)
}

fn load_instance_scripts(conn: &Connection) -> rusqlite::Result<HashMap<u32, String>> {
    let mut statement =
        conn.prepare("SELECT map, script FROM tdb_instance_template WHERE script <> ''")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect()
}

fn load_instance_parents(conn: &Connection) -> rusqlite::Result<HashMap<u32, u32>> {
    let mut statement = conn.prepare("SELECT map, parent FROM tdb_instance_template")?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))?;
    rows.collect()
}
