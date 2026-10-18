//! Spawn pools: TrinityCore `PoolMgr` (src/server/game/Pools/PoolMgr.cpp, a352b1fa).
//!
//! `content_pool_template` caps how many members of a pool are spawned (`max_limit`);
//! `content_pool_members` holds its creature spawns (type 0), gameobject spawns (type 1)
//! and child pools (type 2). Each map copy keeps its own `SpawnedPoolData`: at creation
//! it spawns every pool not in a game event and not a child (`InitPoolsForMap`), and when
//! a pooled creature's respawn time comes the pool re-picks (`UpdatePool`) instead of
//! respawning that spawn (`Map::ProcessRespawns`, Map.cpp:2245-2256).
//!
//! The server does not spawn the gameobject types pools hold (ore, herbs, chests), but
//! their members still take pool slots: a picked gameobject counts against `max_limit`
//! exactly as in PoolMgr, and is never despawned since nothing gathers it.

use std::collections::{BTreeMap, HashMap, HashSet};

use bevy::ecs::world::CommandQueue;
use bevy::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;
use rusqlite::Connection;

use crate::maps::InMap;
use crate::world_data::WorldData;

/// `SpawnObjectType` of `pool_members.type`, and the index of its `PoolGroup`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Creature = 0,
    GameObject = 1,
    Pool = 2,
}

/// `PoolMgr::SpawnPool(spawns, pool_id)` order: child pools, gameobjects, creatures.
const SPAWN_ORDER: [Kind; 3] = [Kind::Pool, Kind::GameObject, Kind::Creature];
/// `PoolMgr::DespawnPool` order.
const DESPAWN_ORDER: [Kind; 3] = [Kind::Creature, Kind::GameObject, Kind::Pool];

/// `PoolObject`: a spawn guid (or child pool id) and its absolute chance.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PoolObject {
    id: u32,
    chance: f32,
}

/// `PoolGroup<T>`: one pool's members of one kind.
#[derive(Debug, Clone, Default)]
struct PoolGroup {
    explicitly_chanced: Vec<PoolObject>,
    equal_chanced: Vec<PoolObject>,
}

impl PoolGroup {
    /// `PoolGroup::AddEntry`.
    fn add(&mut self, object: PoolObject, max_limit: u32) {
        if object.chance != 0.0 && max_limit == 1 {
            self.explicitly_chanced.push(object);
        } else {
            self.equal_chanced.push(object);
        }
    }

    fn is_empty(&self) -> bool {
        self.explicitly_chanced.is_empty() && self.equal_chanced.is_empty()
    }

    /// `PoolGroup::CheckPool`: explicit chances alone must sum to 0 or 100.
    fn check(&self) -> bool {
        if !self.equal_chanced.is_empty() {
            return true;
        }
        let sum: f32 = self.explicitly_chanced.iter().map(|o| o.chance).sum();
        sum == 0.0 || sum == 100.0
    }

    fn members(&self) -> impl Iterator<Item = &PoolObject> {
        self.equal_chanced.iter().chain(&self.explicitly_chanced)
    }
}

/// `PoolTemplateData` plus the pool's three `PoolGroup`s.
#[derive(Debug, Clone, Default)]
struct PoolTemplate {
    max_limit: u32,
    map: Option<u16>,
    groups: [PoolGroup; 3],
}

/// Every pool of world.db (`PoolMgr` after `LoadFromDB`).
#[derive(Debug, Clone, Default)]
pub struct SpawnPools {
    templates: BTreeMap<u32, PoolTemplate>,
    /// `mCreatureSearchMap`, `mGameobjectSearchMap`, `mPoolSearchMap`: member -> its pool.
    search: [HashMap<u32, u32>; 3],
    /// `mAutoSpawnPoolsPerMap`.
    auto_spawn: HashMap<u16, Vec<u32>>,
}

/// `SpawnedPoolData`: the spawned members of one map copy.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpawnedPoolData {
    creatures: HashSet<u32>,
    gameobjects: HashSet<u32>,
    /// `mSpawnedPools`: spawned member count per pool; a child pool's key exists while
    /// it is spawned.
    pools: HashMap<u32, u32>,
}

/// Creature spawns a pool operation leaves the world and enters it, in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolChanges {
    pub despawned: Vec<u32>,
    pub spawned: Vec<u32>,
    pub gameobjects_despawned: Vec<u32>,
    pub gameobjects_spawned: Vec<u32>,
}

/// One `content_pool_members` row with the map of its spawn (none for a child pool).
struct MemberRow {
    kind: Kind,
    id: u32,
    pool: u32,
    chance: f32,
    map: Option<u16>,
}

impl SpawnPools {
    /// `PoolMgr::LoadFromDB` over `content_pool_*` and `content_game_event_pool`.
    pub fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut pools = Self::default();
        let mut templates = conn.prepare("SELECT entry, max_limit FROM content_pool_template")?;
        for row in templates.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (entry, max_limit) = row?;
            pools.templates.insert(
                entry,
                PoolTemplate {
                    max_limit,
                    ..Default::default()
                },
            );
        }
        let members = load_member_rows(conn)?;
        for member in members.iter().filter(|m| m.kind != Kind::Pool) {
            pools.add_spawn_member(member);
        }
        for member in members.iter().filter(|m| m.kind == Kind::Pool) {
            pools.add_child_pool(member);
        }
        pools.propagate_child_maps();
        let children: HashSet<u32> = members
            .iter()
            .filter(|m| m.kind == Kind::Pool)
            .map(|m| m.id)
            .collect();
        let event_pools: HashSet<u32> = conn
            .prepare("SELECT pool_entry FROM content_game_event_pool")?
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        pools.select_auto_spawn(&children, &event_pools);
        Ok(pools)
    }

    /// The creature pool loader of `LoadFromDB` (PoolMgr.cpp:446-510), and its
    /// gameobject twin.
    fn add_spawn_member(&mut self, member: &MemberRow) {
        let Some(template) = self.templates.get_mut(&member.pool) else {
            error!(
                "pool member {} names pool {} without a template",
                member.id, member.pool
            );
            return;
        };
        if !(0.0..=100.0).contains(&member.chance) {
            error!(
                "pool member {} has invalid chance {}",
                member.id, member.chance
            );
            return;
        }
        let map = member.map.expect("spawn member rows carry their map");
        if *template.map.get_or_insert(map) != map {
            error!(
                "pool {} has spawns on several maps, member {} skipped",
                member.pool, member.id
            );
            return;
        }
        let object = PoolObject {
            id: member.id,
            chance: member.chance.abs(),
        };
        template.groups[member.kind as usize].add(object, template.max_limit);
        self.search[member.kind as usize].insert(member.id, member.pool);
    }

    /// The mother pool loader of `LoadFromDB` (PoolMgr.cpp:572-630).
    fn add_child_pool(&mut self, member: &MemberRow) {
        let (child, mother) = (member.id, member.pool);
        if !self.templates.contains_key(&child) || child == mother {
            error!("pool {mother} lists child pool {child} that is missing or itself");
            return;
        }
        let Some(template) = self.templates.get_mut(&mother) else {
            error!("child pool {child} names mother pool {mother} without a template");
            return;
        };
        if !(0.0..=100.0).contains(&member.chance) {
            error!("child pool {child} has invalid chance {}", member.chance);
            return;
        }
        let object = PoolObject {
            id: child,
            chance: member.chance.abs(),
        };
        template.groups[Kind::Pool as usize].add(object, template.max_limit);
        self.search[Kind::Pool as usize].insert(child, mother);
    }

    /// A mother pool takes its children's map; children on another map or in a
    /// cycle lose their link (PoolMgr.cpp:632-662).
    fn propagate_child_maps(&mut self) {
        let ids: Vec<u32> = self.templates.keys().copied().collect();
        for id in ids {
            let mut checked = HashSet::new();
            let mut child = id;
            while let Some(&mother) = self.search[Kind::Pool as usize].get(&child) {
                if let Some(child_map) = self.templates[&child].map {
                    let mother_map = *self
                        .templates
                        .get_mut(&mother)
                        .expect("mother pool template")
                        .map
                        .get_or_insert(child_map);
                    if mother_map != child_map {
                        error!("pool {mother} has child pools on several maps, {child} skipped");
                        self.unlink_child(child, mother);
                        break;
                    }
                }
                checked.insert(child);
                if checked.contains(&mother) {
                    error!("pools {checked:?} form a cycle; unlinking {child} from {mother}");
                    self.unlink_child(child, mother);
                    break;
                }
                child = mother;
            }
        }
    }

    /// `PoolGroup<Pool>::RemoveOneRelation` and the search map erase.
    fn unlink_child(&mut self, child: u32, mother: u32) {
        let group =
            &mut self.templates.get_mut(&mother).expect("mother").groups[Kind::Pool as usize];
        group.explicitly_chanced.retain(|o| o.id != child);
        group.equal_chanced.retain(|o| o.id != child);
        self.search[Kind::Pool as usize].remove(&child);
    }

    /// The pools `InitPoolsForMap` spawns: not empty, chances valid, not in a game
    /// event and not a child pool (PoolMgr.cpp:676-718).
    fn select_auto_spawn(&mut self, children: &HashSet<u32>, event_pools: &HashSet<u32>) {
        for (&id, template) in &self.templates {
            if event_pools.contains(&id) || self.is_empty(id) {
                continue;
            }
            if !self.check_pool(id) {
                error!("pool {id}: explicit chances do not sum to 100, not spawned");
                continue;
            }
            if !children.contains(&id) {
                let map = template.map.expect("a non-empty pool has a map");
                self.auto_spawn.entry(map).or_default().push(id);
            }
        }
    }

    /// `PoolMgr::IsEmpty`: no spawn member in the pool or any child pool.
    fn is_empty(&self, pool: u32) -> bool {
        let Some(template) = self.templates.get(&pool) else {
            return true;
        };
        template.groups[Kind::Creature as usize].is_empty()
            && template.groups[Kind::GameObject as usize].is_empty()
            && template.groups[Kind::Pool as usize]
                .members()
                .all(|child| self.is_empty(child.id))
    }

    /// `PoolMgr::CheckPool`.
    fn check_pool(&self, pool: u32) -> bool {
        self.templates
            .get(&pool)
            .is_none_or(|template| template.groups.iter().all(PoolGroup::check))
    }

    /// `PoolMgr::IsPartOfAPool<Creature>`: the pool of a creature spawn.
    pub fn creature_pool(&self, guid: u32) -> Option<u32> {
        self.search[Kind::Creature as usize].get(&guid).copied()
    }

    pub fn gameobject_pool(&self, guid: u32) -> Option<u32> {
        self.search[Kind::GameObject as usize].get(&guid).copied()
    }

    pub fn respawn_gameobject(
        &self,
        spawns: &mut SpawnedPoolData,
        guid: u32,
        rng: &mut impl Rng,
    ) -> PoolChanges {
        let mut changes = PoolChanges::default();
        let pool = self.gameobject_pool(guid).expect("pooled gameobject");
        match self.search[Kind::Pool as usize].get(&pool) {
            Some(&mother) => {
                self.spawn_group(Kind::Pool, spawns, mother, Some(pool), &mut changes, rng)
            }
            None => self.spawn_group(
                Kind::GameObject,
                spawns,
                pool,
                Some(guid),
                &mut changes,
                rng,
            ),
        }
        changes
    }

    /// `PoolMgr::InitPoolsForMap`: a new copy of `map` with its pools spawned; returns
    /// the creature spawns picked.
    pub fn init_map(&self, map: u16, rng: &mut impl Rng) -> (SpawnedPoolData, Vec<u32>) {
        let mut spawns = SpawnedPoolData::default();
        let mut changes = PoolChanges::default();
        for &pool in self.auto_spawn.get(&map).into_iter().flatten() {
            self.spawn_pool(&mut spawns, pool, &mut changes, rng);
        }
        (spawns, changes.spawned)
    }

    /// `PoolMgr::UpdatePool<Creature>`: the respawn time of pooled creature `guid` came.
    pub fn respawn_creature(
        &self,
        spawns: &mut SpawnedPoolData,
        guid: u32,
        rng: &mut impl Rng,
    ) -> PoolChanges {
        let mut changes = PoolChanges::default();
        let pool = self
            .creature_pool(guid)
            .expect("respawn_creature takes a pooled creature");
        match self.search[Kind::Pool as usize].get(&pool) {
            Some(&mother) => {
                self.spawn_group(Kind::Pool, spawns, mother, Some(pool), &mut changes, rng)
            }
            None => self.spawn_group(Kind::Creature, spawns, pool, Some(guid), &mut changes, rng),
        }
        changes
    }

    /// `PoolMgr::SpawnPool(spawns, pool_id)`.
    fn spawn_pool(
        &self,
        spawns: &mut SpawnedPoolData,
        pool: u32,
        changes: &mut PoolChanges,
        rng: &mut impl Rng,
    ) {
        for kind in SPAWN_ORDER {
            self.spawn_group(kind, spawns, pool, None, changes, rng);
        }
    }

    /// `PoolMgr::SpawnPool<T>` → `PoolGroup<T>::SpawnObject` (PoolMgr.cpp:269-331).
    fn spawn_group(
        &self,
        kind: Kind,
        spawns: &mut SpawnedPoolData,
        pool: u32,
        trigger: Option<u32>,
        changes: &mut PoolChanges,
        rng: &mut impl Rng,
    ) {
        let Some(template) = self.templates.get(&pool) else {
            return;
        };
        let group = &template.groups[kind as usize];
        if group.is_empty() {
            return;
        }
        if let Some(trigger) = trigger {
            self.despawn_object(kind, spawns, pool, trigger, changes);
        }
        let count = i64::from(template.max_limit) - i64::from(spawns.spawned_in(pool));
        if count <= 0 {
            return;
        }
        let mut candidates: Vec<PoolObject> = group
            .members()
            .filter(|object| !spawns.is_spawned(kind, object.id))
            .copied()
            .collect();
        if candidates.is_empty() {
            return;
        }
        let mut rolled = Vec::new();
        if !group.explicitly_chanced.is_empty() {
            let mut roll = rng.gen_range(0.0..100.0_f32);
            let picked = candidates.iter().position(|candidate| {
                if candidate.chance <= 0.0 {
                    return false;
                }
                roll -= candidate.chance;
                roll < 0.0
            });
            if let Some(index) = picked {
                rolled.push(candidates.swap_remove(index));
            }
        }
        let remaining = count as usize - rolled.len();
        if remaining > 0 {
            if candidates.len() > remaining {
                candidates.shuffle(rng);
                candidates.truncate(remaining);
            }
            rolled.extend(candidates);
        }
        for object in rolled {
            spawns.add(kind, object.id, pool);
            match kind {
                Kind::Creature => changes.spawned.push(object.id),
                Kind::GameObject => changes.gameobjects_spawned.push(object.id),
                Kind::Pool => self.spawn_pool(spawns, object.id, changes, rng),
            }
        }
    }

    /// `PoolGroup<T>::DespawnObject`.
    fn despawn_object(
        &self,
        kind: Kind,
        spawns: &mut SpawnedPoolData,
        pool: u32,
        id: u32,
        changes: &mut PoolChanges,
    ) {
        if !spawns.is_spawned(kind, id) {
            return;
        }
        match kind {
            Kind::Creature => changes.despawned.push(id),
            Kind::GameObject => changes.gameobjects_despawned.push(id),
            Kind::Pool => self.despawn_pool(spawns, id, changes),
        }
        spawns.remove(kind, id, pool);
    }

    /// `PoolMgr::DespawnPool`.
    fn despawn_pool(&self, spawns: &mut SpawnedPoolData, pool: u32, changes: &mut PoolChanges) {
        let Some(template) = self.templates.get(&pool) else {
            return;
        };
        for kind in DESPAWN_ORDER {
            for object in template.groups[kind as usize].members() {
                self.despawn_object(kind, spawns, pool, object.id, changes);
            }
        }
    }
}

impl SpawnedPoolData {
    pub fn gameobject_spawned(&self, guid: u32) -> bool {
        self.gameobjects.contains(&guid)
    }

    /// `SpawnedPoolData::GetSpawnedObjects`.
    fn spawned_in(&self, pool: u32) -> u32 {
        self.pools.get(&pool).copied().unwrap_or(0)
    }

    /// `SpawnedPoolData::IsSpawnedObject<T>`.
    fn is_spawned(&self, kind: Kind, id: u32) -> bool {
        match kind {
            Kind::Creature => self.creatures.contains(&id),
            Kind::GameObject => self.gameobjects.contains(&id),
            Kind::Pool => self.pools.contains_key(&id),
        }
    }

    /// `SpawnedPoolData::AddSpawn<T>`.
    fn add(&mut self, kind: Kind, id: u32, pool: u32) {
        match kind {
            Kind::Creature => {
                self.creatures.insert(id);
            }
            Kind::GameObject => {
                self.gameobjects.insert(id);
            }
            Kind::Pool => {
                self.pools.insert(id, 0);
            }
        }
        *self.pools.entry(pool).or_insert(0) += 1;
    }

    /// `SpawnedPoolData::RemoveSpawn<T>`.
    fn remove(&mut self, kind: Kind, id: u32, pool: u32) {
        match kind {
            Kind::Creature => {
                self.creatures.remove(&id);
            }
            Kind::GameObject => {
                self.gameobjects.remove(&id);
            }
            Kind::Pool => {
                self.pools.remove(&id);
            }
        }
        let count = self.pools.entry(pool).or_insert(0);
        *count = count.saturating_sub(1);
    }
}

/// `content_pool_members` in load order, with each spawn member's map; members whose
/// spawn is not in content_creature / content_gameobject are left out
/// (`pool_creature has a non existing creature spawn`).
fn load_member_rows(conn: &Connection) -> rusqlite::Result<Vec<MemberRow>> {
    let mut statement = conn.prepare(
        "SELECT m.type, m.spawnId, m.poolSpawnId, m.chance, COALESCE(c.map, g.map)
         FROM content_pool_members m
         LEFT JOIN content_creature c ON m.type = 0 AND c.guid = m.spawnId
         LEFT JOIN content_gameobject g ON m.type = 1 AND g.guid = m.spawnId
         WHERE m.type = 2 OR c.guid IS NOT NULL OR g.guid IS NOT NULL
         ORDER BY m.type, m.spawnId",
    )?;
    let rows = statement.query_map([], |row| {
        let kind = match row.get::<_, u8>(0)? {
            0 => Kind::Creature,
            1 => Kind::GameObject,
            2 => Kind::Pool,
            other => panic!("content_pool_members type {other}"),
        };
        Ok(MemberRow {
            kind,
            id: row.get(1)?,
            pool: row.get(2)?,
            chance: row.get::<_, f64>(3)? as f32,
            map: row.get(4)?,
        })
    })?;
    rows.collect()
}

/// A creature spawned by its pool: the spawn guid it is.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PooledSpawn(pub u32);

/// One map copy's pool state and the entity of each pooled creature it spawned.
#[derive(Debug, Default)]
pub(crate) struct MapPools {
    pub spawns: SpawnedPoolData,
    pub entities: HashMap<u32, Entity>,
    pub gameobjects: HashMap<u32, Entity>,
    pub difficulty_id: u32,
    pub replicate: bool,
}

/// `Map::GetPoolData` of every map copy with creatures.
#[derive(Resource, Debug, Default)]
pub(crate) struct PoolSpawns(pub HashMap<InMap, MapPools>);

/// `Map::ProcessRespawns` for a pooled creature: the dead entity leaves the world and
/// its pool spawns what it picks (`PoolMgr::UpdatePool`); creatures of a child pool the
/// mother pool despawns leave too.
pub(crate) fn respawn_pooled(world: &mut World, entity: Entity) {
    let (Some(&PooledSpawn(guid)), Some(&map)) =
        (world.get::<PooledSpawn>(entity), world.get::<InMap>(entity))
    else {
        return;
    };
    let mut map_pools = world
        .resource_mut::<PoolSpawns>()
        .0
        .remove(&map)
        .expect("a pooled creature's map copy has pool data");
    world.resource_scope(|world, world_data: Mut<WorldData>| {
        let changes =
            world_data
                .pools
                .respawn_creature(&mut map_pools.spawns, guid, &mut rand::thread_rng());
        apply_pool_changes(world, &world_data, map, &mut map_pools, &changes);
    });
    world.resource_mut::<PoolSpawns>().0.insert(map, map_pools);
}

pub(crate) fn respawn_gameobject(
    world: &mut World,
    spawn: &crate::game_objects::GameObjectSpawn,
    map: InMap,
) {
    if world
        .resource::<WorldData>()
        .pools
        .gameobject_pool(spawn.guid)
        .is_none()
    {
        if let Err(gap) = crate::gathering::spawn_node(world, spawn, map) {
            warn!("{gap}");
        }
        return;
    }
    let mut map_pools = world
        .resource_mut::<PoolSpawns>()
        .0
        .remove(&map)
        .expect("pooled node map state");
    world.resource_scope(|world, world_data: Mut<WorldData>| {
        let changes = world_data.pools.respawn_gameobject(
            &mut map_pools.spawns,
            spawn.guid,
            &mut rand::thread_rng(),
        );
        apply_pool_changes(world, &world_data, map, &mut map_pools, &changes);
    });
    world.resource_mut::<PoolSpawns>().0.insert(map, map_pools);
}

fn apply_pool_changes(
    world: &mut World,
    world_data: &WorldData,
    map: InMap,
    map_pools: &mut MapPools,
    changes: &PoolChanges,
) {
    for guid in &changes.gameobjects_despawned {
        if let Some(entity) = map_pools.gameobjects.remove(guid)
            && world.get_entity(entity).is_ok()
        {
            world.entity_mut(entity).despawn();
        }
    }
    let nodes = world
        .resource::<crate::game_objects::GameObjectSpawns>()
        .0
        .iter()
        .filter(|spawn| changes.gameobjects_spawned.contains(&spawn.guid))
        .cloned()
        .collect::<Vec<_>>();
    for spawn in nodes {
        match crate::gathering::spawn_node(world, &spawn, map) {
            Ok(entity) => {
                map_pools.gameobjects.insert(spawn.guid, entity);
            }
            Err(gap) => warn!("{gap}"),
        }
    }
    for guid in &changes.despawned {
        if let Some(entity) = map_pools.entities.remove(guid) {
            world.entity_mut(entity).despawn();
        }
    }
    let mut queue = CommandQueue::default();
    world.init_resource::<crate::ground::GroundMap>();
    let spawned = world.resource_scope(|world, mut ground: Mut<crate::ground::GroundMap>| {
        let mut commands = Commands::new(&mut queue, world);
        crate::zones::spawn_pooled_creatures(
            &mut commands,
            world_data,
            map,
            map_pools,
            &changes.spawned,
            Some(&mut ground),
        )
    });
    queue.apply(world);
    map_pools.entities.extend(spawned);
}
