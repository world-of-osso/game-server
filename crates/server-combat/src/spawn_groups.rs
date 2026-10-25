//! Condition-driven creature and gameobject spawn groups: TrinityCore `ObjectMgr::LoadSpawnGroups`,
//! `Map::UpdateSpawnGroupConditions` and the `ConditionMgr` checks they use (a352b1fa).
//!
//! TDB1210 `spawn_group` / `spawn_group_template` rows with
//! `CONDITION_SOURCE_TYPE_SPAWN_GROUP` (33) conditions decide which member creatures a
//! battleground copy holds (Alterac Valley node owners, defender tiers, mine crews; Isle of
//! Conquest node banners, spirit guides, miners and vehicles). A
//! group's members never spawn with ordinary map content; its battleground runtime
//! spawns them while the conditions hold. A group meets its conditions when every
//! condition of one `ElseGroup` holds, each negated by `NegativeCondition`.

use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Maps whose battleground runtime evaluates spawn group conditions.
const CONDITIONED_MAPS: [u16; 2] = [30, 628];
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

#[derive(Debug, Clone)]
pub struct SpawnGroup {
    pub despawn_on_failure: bool,
    /// Content creature spawn guids.
    pub creatures: Vec<u32>,
    /// Content gameobject spawn guids.
    pub objects: Vec<u32>,
    /// `ElseGroup` alternatives, each a conjunction.
    alternatives: Vec<Vec<Condition>>,
}

/// A creature spawned by its condition-driven spawn group: the spawn guid it is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupedSpawn(pub u32);

/// A game object spawned by its condition-driven spawn group: the spawn guid it is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupedObject(pub u32);

/// Grouped spawns of each map copy whose group is inactive: they must not respawn.
#[derive(Resource, Debug, Default)]
pub(crate) struct InactiveGroupedSpawns(pub HashMap<crate::maps::InMap, BTreeSet<u32>>);

#[derive(Debug, Clone, Default)]
pub struct SpawnGroups {
    groups: Vec<SpawnGroup>,
    creature_members: HashSet<u32>,
    object_members: HashSet<u32>,
}

impl SpawnGroups {
    /// Groups with spawn group conditions whose spawns are on a conditioned map.
    pub fn load(conn: &Connection) -> Result<Self, String> {
        let rows = load_condition_rows(conn).map_err(|e| e.to_string())?;
        let mut groups = Self::default();
        for member in load_members(conn).map_err(|e| e.to_string())? {
            let Some(conditions) = rows.get(&member.id) else {
                continue;
            };
            let alternatives = parse_alternatives(conditions)
                .map_err(|reason| format!("spawn group {}: {reason}", member.id))?;
            groups.creature_members.extend(&member.creatures);
            groups.object_members.extend(&member.objects);
            groups.groups.push(SpawnGroup {
                despawn_on_failure: member.flags & DESPAWN_ON_CONDITION_FAILURE != 0,
                creatures: member.creatures,
                objects: member.objects,
                alternatives,
            });
        }
        Ok(groups)
    }

    pub fn is_member(&self, guid: u32) -> bool {
        self.creature_members.contains(&guid)
    }

    pub fn is_object_member(&self, guid: u32) -> bool {
        self.object_members.contains(&guid)
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
/// every failed group's creatures stop respawning (`SetSpawnGroupInactive`).
pub(crate) fn synchronize(
    world: &mut World,
    map: crate::maps::InMap,
    states: &[(u32, i32)],
    data: impl Fn(u32) -> u32,
) {
    let states: HashMap<u32, i32> = states.iter().copied().collect();
    let mut creatures = world.query::<(Entity, &GroupedSpawn, &crate::maps::InMap)>();
    let creatures: HashMap<u32, Entity> = creatures
        .iter(world)
        .filter(|(_, _, copy)| **copy == map)
        .map(|(entity, spawn, _)| (spawn.0, entity))
        .collect();
    let mut objects = world.query::<(Entity, &GroupedObject, &crate::maps::InMap)>();
    let objects: HashMap<u32, Entity> = objects
        .iter(world)
        .filter(|(_, _, copy)| **copy == map)
        .map(|(entity, spawn, _)| (spawn.0, entity))
        .collect();
    let groups = &world
        .resource::<crate::world_data::WorldData>()
        .spawn_groups;
    let plan = plan(groups, &states, &data, &creatures, &objects);
    apply(world, map, plan);
}

#[derive(Default)]
struct Plan {
    creatures: BTreeSet<u32>,
    objects: BTreeSet<u32>,
    despawn: Vec<Entity>,
    inactive: BTreeSet<u32>,
}

fn plan(
    groups: &SpawnGroups,
    states: &HashMap<u32, i32>,
    data: &impl Fn(u32) -> u32,
    creatures: &HashMap<u32, Entity>,
    objects: &HashMap<u32, Entity>,
) -> Plan {
    let absent = |guids: &[u32], present: &HashMap<u32, Entity>| {
        guids
            .iter()
            .filter(|guid| !present.contains_key(guid))
            .copied()
            .collect::<Vec<_>>()
    };
    let spawned = |guids: &[u32], present: &HashMap<u32, Entity>| {
        guids
            .iter()
            .filter_map(|guid| present.get(guid).copied())
            .collect::<Vec<_>>()
    };
    let mut plan = Plan::default();
    for group in &groups.groups {
        if group.holds(states, data) {
            plan.creatures.extend(absent(&group.creatures, creatures));
            plan.objects.extend(absent(&group.objects, objects));
            continue;
        }
        plan.inactive.extend(&group.creatures);
        if group.despawn_on_failure {
            plan.despawn.extend(spawned(&group.creatures, creatures));
            plan.despawn.extend(spawned(&group.objects, objects));
        }
    }
    plan
}

fn apply(world: &mut World, map: crate::maps::InMap, plan: Plan) {
    for entity in plan.despawn {
        world.entity_mut(entity).despawn();
    }
    world
        .get_resource_or_init::<InactiveGroupedSpawns>()
        .0
        .insert(map, plan.inactive);
    let difficulty_id = world
        .resource::<crate::maps::InstanceManager>()
        .copy_difficulty(map)
        .map_or(0, |difficulty| difficulty.difficulty_id);
    let mut queue = bevy::ecs::world::CommandQueue::default();
    world.init_resource::<crate::ground::GroundMap>();
    world.resource_scope(|world, mut ground: Mut<crate::ground::GroundMap>| {
        let mut commands = Commands::new(&mut queue, world);
        let world_data = world.resource::<crate::world_data::WorldData>();
        crate::zones::spawn_grouped_creatures(
            &mut commands,
            world_data,
            map,
            difficulty_id,
            &plan.creatures,
            Some(&mut ground),
        );
        let catalog = world.resource::<crate::game_objects::GameObjectSpawns>();
        for spawn in catalog
            .0
            .iter()
            .filter(|spawn| plan.objects.contains(&spawn.guid))
        {
            let object = crate::game_objects::spawn_game_object(&mut commands, spawn, map);
            commands.entity(object).insert(GroupedObject(spawn.guid));
        }
    });
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

/// One conditioned map's spawn group with its content creature and gameobject guids.
struct Members {
    id: u32,
    flags: u32,
    creatures: Vec<u32>,
    objects: Vec<u32>,
}

fn load_members(conn: &Connection) -> rusqlite::Result<Vec<Members>> {
    let maps = CONDITIONED_MAPS.map(|map| map.to_string()).join(",");
    let mut statement = conn.prepare(&format!(
        "SELECT t.groupId, t.groupFlags, g.spawnType, COALESCE(c.guid, o.guid)
         FROM tdb_spawn_group g
         JOIN tdb_spawn_group_template t ON t.groupId = g.groupId
         LEFT JOIN content_creature c ON g.spawnType = 0 AND c.guid = g.spawnId + ?1
         LEFT JOIN content_gameobject o ON g.spawnType = 1 AND o.guid = g.spawnId + ?1
         WHERE COALESCE(c.map, o.map) IN ({maps})
         ORDER BY t.groupId, g.spawnType, g.spawnId"
    ))?;
    let mut groups: Vec<Members> = Vec::new();
    for row in statement.query_map([RETAIL_GUID_OFFSET], |row| {
        Ok((
            row.get::<_, u32>(0)?,
            row.get::<_, u32>(1)?,
            row.get::<_, u8>(2)?,
            row.get::<_, u32>(3)?,
        ))
    })? {
        let (id, flags, kind, guid) = row?;
        if groups.last().is_none_or(|last| last.id != id) {
            groups.push(Members {
                id,
                flags,
                creatures: Vec::new(),
                objects: Vec::new(),
            });
        }
        let members = groups.last_mut().expect("pushed group");
        match kind {
            0 => members.creatures.push(guid),
            _ => members.objects.push(guid),
        }
    }
    Ok(groups)
}
