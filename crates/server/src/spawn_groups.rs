//! Condition-driven creature spawn groups: TrinityCore `ObjectMgr::LoadSpawnGroups`,
//! `Map::UpdateSpawnGroupConditions` and the `ConditionMgr` checks they use (a352b1fa).
//!
//! TDB1210 `spawn_group` / `spawn_group_template` rows with
//! `CONDITION_SOURCE_TYPE_SPAWN_GROUP` (33) conditions decide which member creatures a
//! battleground copy holds (Alterac Valley node owners, defender tiers, mine crews). A
//! group's members never spawn with ordinary map content; its battleground runtime
//! spawns them while the conditions hold. A group meets its conditions when every
//! condition of one `ElseGroup` holds, each negated by `NegativeCondition`.

use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Maps whose battleground runtime evaluates spawn group conditions.
const CONDITIONED_MAPS: [u16; 1] = [30];
/// TDB1210 spawn guids are promoted into the content namespace at this offset.
const RETAIL_GUID_OFFSET: u32 = 20_000_000;
/// `SPAWNGROUP_FLAG_DESPAWN_ON_CONDITION_FAILURE`.
const DESPAWN_ON_CONDITION_FAILURE: u32 = 0x20;
const SOURCE_SPAWN_GROUP: u32 = 33;
/// `CONDITION_WORLD_STATE`: world state `ConditionValue1` equals `ConditionValue2`.
const CONDITION_WORLD_STATE: u32 = 11;
/// `CONDITION_INSTANCE_INFO` with `INSTANCE_INFO_DATA` (`ConditionValue3` 0):
/// `GetData(ConditionValue1)` equals `ConditionValue2`.
const CONDITION_INSTANCE_INFO: u32 = 13;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Check {
    WorldState { state: u32, value: i32 },
    InstanceData { data: u32, value: u32 },
}

#[derive(Debug, Clone, Copy)]
struct Condition {
    check: Check,
    negative: bool,
}

#[derive(Debug)]
pub struct SpawnGroup {
    pub despawn_on_failure: bool,
    /// Content creature spawn guids.
    pub creatures: Vec<u32>,
    /// `ElseGroup` alternatives, each a conjunction.
    alternatives: Vec<Vec<Condition>>,
}

/// A creature spawned by its condition-driven spawn group: the spawn guid it is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupedSpawn(pub u32);

/// Grouped spawns of each map copy whose group is inactive: they must not respawn.
#[derive(Resource, Debug, Default)]
pub(crate) struct InactiveGroupedSpawns(pub HashMap<crate::maps::InMap, BTreeSet<u32>>);

#[derive(Debug, Default)]
pub struct SpawnGroups {
    groups: Vec<SpawnGroup>,
    member_of: HashMap<u32, usize>,
}

impl SpawnGroups {
    /// Groups with spawn group conditions whose creatures are on a conditioned map.
    pub fn load(conn: &Connection) -> Result<Self, String> {
        let rows = load_condition_rows(conn).map_err(|e| e.to_string())?;
        let mut groups = Self::default();
        for (id, flags, creatures) in load_members(conn).map_err(|e| e.to_string())? {
            let Some(conditions) = rows.get(&id) else {
                continue;
            };
            let alternatives = parse_alternatives(conditions)
                .map_err(|reason| format!("spawn group {id}: {reason}"))?;
            let index = groups.groups.len();
            for &guid in &creatures {
                groups.member_of.insert(guid, index);
            }
            groups.groups.push(SpawnGroup {
                despawn_on_failure: flags & DESPAWN_ON_CONDITION_FAILURE != 0,
                creatures,
                alternatives,
            });
        }
        Ok(groups)
    }

    pub fn is_member(&self, guid: u32) -> bool {
        self.member_of.contains_key(&guid)
    }

    pub fn groups(&self) -> &[SpawnGroup] {
        &self.groups
    }

    /// Every instance data id a group condition reads.
    pub fn instance_data_ids(&self) -> BTreeSet<u32> {
        self.groups
            .iter()
            .flat_map(|group| group.alternatives.iter().flatten())
            .filter_map(|condition| match condition.check {
                Check::InstanceData { data, .. } => Some(data),
                Check::WorldState { .. } => None,
            })
            .collect()
    }
}

impl SpawnGroup {
    /// `ConditionMgr::IsObjectMeetToConditions`; an absent world state reads 0
    /// (`WorldStateMgr::GetValue`).
    pub fn holds(&self, states: &HashMap<u32, i32>, data: impl Fn(u32) -> u32) -> bool {
        self.alternatives.iter().any(|all| {
            all.iter().all(|condition| {
                let met = match condition.check {
                    Check::WorldState { state, value } => {
                        states.get(&state).copied().unwrap_or(0) == value
                    }
                    Check::InstanceData { data: id, value } => data(id) == value,
                };
                met != condition.negative
            })
        })
    }
}

/// `Map::UpdateSpawnGroupConditions` for `map`'s copy: condition-met groups spawn their
/// absent members, failed groups with `DESPAWN_ON_CONDITION_FAILURE` despawn theirs, and
/// every failed group's members stop respawning (`SetSpawnGroupInactive`).
pub(crate) fn synchronize(
    world: &mut World,
    map: crate::maps::InMap,
    states: &[(u32, i32)],
    data: impl Fn(u32) -> u32,
) {
    let states: HashMap<u32, i32> = states.iter().copied().collect();
    let mut query = world.query::<(Entity, &GroupedSpawn, &crate::maps::InMap)>();
    let present: HashMap<u32, Entity> = query
        .iter(world)
        .filter(|(_, _, copy)| **copy == map)
        .map(|(entity, spawn, _)| (spawn.0, entity))
        .collect();
    let mut spawn = BTreeSet::new();
    let mut despawn = Vec::new();
    let mut inactive = BTreeSet::new();
    for group in world
        .resource::<crate::world_data::WorldData>()
        .spawn_groups
        .groups()
    {
        if group.holds(&states, &data) {
            spawn.extend(
                group
                    .creatures
                    .iter()
                    .filter(|guid| !present.contains_key(guid)),
            );
            continue;
        }
        inactive.extend(&group.creatures);
        if group.despawn_on_failure {
            despawn.extend(
                group
                    .creatures
                    .iter()
                    .filter_map(|guid| present.get(guid).copied()),
            );
        }
    }
    for entity in despawn {
        world.entity_mut(entity).despawn();
    }
    world
        .get_resource_or_init::<InactiveGroupedSpawns>()
        .0
        .insert(map, inactive);
    let difficulty_id = world
        .resource::<crate::maps::InstanceManager>()
        .copy_difficulty(map)
        .map_or(0, |difficulty| difficulty.difficulty_id);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    {
        let mut commands = Commands::new(&mut queue, world);
        let world_data = world.resource::<crate::world_data::WorldData>();
        crate::zones::spawn_grouped_creatures(
            &mut commands,
            world_data,
            map,
            difficulty_id,
            &spawn,
        );
    }
    queue.apply(world);
}

/// One `conditions` row: `ElseGroup`, type, `ConditionValue1..3`, `NegativeCondition`.
type ConditionRow = (u32, u32, i64, i64, i64, bool);

fn load_condition_rows(conn: &Connection) -> rusqlite::Result<HashMap<u32, Vec<ConditionRow>>> {
    let mut statement = conn.prepare(
        "SELECT SourceEntry, ElseGroup, ConditionTypeOrReference, ConditionValue1,
                ConditionValue2, ConditionValue3, NegativeCondition
         FROM tdb_conditions WHERE SourceTypeOrReferenceId = ?1",
    )?;
    let mut rows: HashMap<u32, Vec<ConditionRow>> = HashMap::new();
    for row in statement.query_map([SOURCE_SPAWN_GROUP], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            (
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get::<_, u8>(6)? != 0,
            ),
        ))
    })? {
        let (group, condition) = row?;
        rows.entry(group).or_default().push(condition);
    }
    Ok(rows)
}

fn parse_alternatives(rows: &[ConditionRow]) -> Result<Vec<Vec<Condition>>, String> {
    let mut alternatives: BTreeMap<u32, Vec<Condition>> = BTreeMap::new();
    for &(else_group, kind, value1, value2, value3, negative) in rows {
        let check = parse_check(kind, value1, value2, value3)?;
        alternatives
            .entry(else_group)
            .or_default()
            .push(Condition { check, negative });
    }
    Ok(alternatives.into_values().collect())
}

fn parse_check(kind: u32, value1: i64, value2: i64, value3: i64) -> Result<Check, String> {
    match (kind, value3) {
        (CONDITION_WORLD_STATE, _) => Ok(Check::WorldState {
            state: value1 as u32,
            value: value2 as i32,
        }),
        (CONDITION_INSTANCE_INFO, 0) => Ok(Check::InstanceData {
            data: value1 as u32,
            value: value2 as u32,
        }),
        _ => Err(format!(
            "unsupported condition type {kind} (value3 {value3})"
        )),
    }
}

/// `(group, flags, content creature guids)` of conditioned groups on conditioned maps.
fn load_members(conn: &Connection) -> rusqlite::Result<Vec<(u32, u32, Vec<u32>)>> {
    let maps = CONDITIONED_MAPS.map(|map| map.to_string()).join(",");
    let mut statement = conn.prepare(&format!(
        "SELECT t.groupId, t.groupFlags, c.guid
         FROM tdb_spawn_group g
         JOIN tdb_spawn_group_template t ON t.groupId = g.groupId
         JOIN content_creature c ON c.guid = g.spawnId + ?1
         WHERE g.spawnType = 0 AND c.map IN ({maps})
         ORDER BY t.groupId, c.guid"
    ))?;
    let mut groups: Vec<(u32, u32, Vec<u32>)> = Vec::new();
    for row in statement.query_map([RETAIL_GUID_OFFSET], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u32>(2)?,
        ))
    })? {
        let (id, flags, guid) = row?;
        match groups.last_mut() {
            Some((last, _, members)) if *last == id => members.push(guid),
            _ => groups.push((id, flags, vec![guid])),
        }
    }
    Ok(groups)
}
