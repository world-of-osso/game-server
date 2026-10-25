//! Spawn pools on the imported world.db (TDB1210 pool_template / pool_members as
//! content_pool_*): `cargo test -p server spawn_pools_world_db -- --ignored`.

use super::*;
use crate::combat::DeathState;
use crate::world_data::load_world_data;
use bevy::time::TimeUpdateStrategy;
use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::path::PathBuf;

/// Lost Soul (1531, rare), pool 351 "Lost Soul": max_limit 1, three Tirisfal spawns
/// (TrinityCore guids 325765, 325966, 325970) at chance 0.
const LOST_SOUL_POOL: u32 = 351;
/// Westfall "The Jansen Stead - Hobo Pool 11": max_limit 1, four spawns of four entries.
const HOBO_POOL: u32 = 3_000_013;
/// "Felwood Ore Mother pool" 36101 (max_limit 66) picks among 174 "Felwood Ore Spawnpoint"
/// child pools; 361001..=361034 each hold Deadwood creatures at chance 0 and three ore
/// nodes whose chances sum to 100, so the ore fills the child's max_limit 1.
const FELWOOD_ORE_CHILD_POOLS: std::ops::RangeInclusive<u32> = 361_001..=361_034;

fn world_db_path() -> PathBuf {
    crate::test_data::world_db_path()
}

/// `content_pool_members` creature spawn guids of `pools`.
fn pool_creature_guids(pools: impl IntoIterator<Item = u32>) -> Vec<u32> {
    let conn = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("world.db");
    let mut statement = conn
        .prepare("SELECT spawnId FROM content_pool_members WHERE type = 0 AND poolSpawnId = ?1")
        .unwrap();
    let mut guids = Vec::new();
    for pool in pools {
        let rows = statement
            .query_map([pool], |row| row.get::<_, u32>(0))
            .unwrap();
        guids.extend(rows.map(Result::unwrap));
    }
    guids
}

fn pooled_guids_on_map(map: u16) -> HashSet<u32> {
    let conn = Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY)
        .expect("world.db");
    let mut statement = conn
        .prepare(
            "SELECT m.spawnId FROM content_pool_members m
             JOIN content_creature c ON c.guid = m.spawnId WHERE m.type = 0 AND c.map = ?1",
        )
        .unwrap();
    statement
        .query_map([map], |row| row.get::<_, u32>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// App with the real world data that fills continents 0 and 1 and runs the respawn timer.
fn continents_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(Time::<Fixed>::from_duration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(load_world_data(&world_db_path()));
    app.init_resource::<crate::combat::CombatLog>();
    app.add_systems(FixedUpdate, crate::combat::respawn_system);
    app.add_systems(
        PostStartup,
        |mut commands: Commands, world_data: Res<WorldData>| {
            for map_id in crate::maps::HOSTED_CONTINENTS {
                spawn_map_creatures(
                    &mut commands,
                    &world_data,
                    MapContent::continent(map_id),
                    false,
                    Some(&mut crate::test_data::ground_map()),
                );
            }
        },
    );
    app.update();
    app
}

/// The living creature entities standing at one of `guids`' spawn points as that
/// spawn's entry, with the guid of the spawn each one is.
fn alive_spawns(app: &mut App, guids: &[u32]) -> Vec<(u32, Entity)> {
    let world_data = app.world().resource::<WorldData>();
    let spawns: Vec<(u32, u32, Position)> = world_data
        .spawns
        .iter()
        .filter(|spawn| guids.contains(&spawn.guid))
        .map(|spawn| {
            let (x, y, z) = crate::terrain_height::wow_to_bevy(
                spawn.position_x,
                spawn.position_y,
                spawn.position_z,
            );
            (spawn.guid, spawn.template_entry, Position { x, y, z })
        })
        .collect();
    let mut query = app
        .world_mut()
        .query_filtered::<(Entity, &Npc, &Position), Without<DeathState>>();
    let mut alive: Vec<(u32, Entity)> = query
        .iter(app.world())
        .filter_map(|(entity, npc, position)| {
            spawns
                .iter()
                .find(|(_, entry, at)| *entry == npc.template_id && at == position)
                .map(|(guid, _, _)| (*guid, entity))
        })
        .collect();
    alive.sort();
    alive
}

#[test]
#[ignore]
fn continents_start_with_max_limit_members_of_each_pool_alive() {
    let lost_souls = pool_creature_guids([LOST_SOUL_POOL]);
    assert_eq!(lost_souls.len(), 3);
    let hobos = pool_creature_guids([HOBO_POOL]);
    assert_eq!(hobos.len(), 4);
    let deadwood = pool_creature_guids(FELWOOD_ORE_CHILD_POOLS);
    assert_eq!(deadwood.len(), 102);

    let mut app = continents_app();
    assert_eq!(alive_spawns(&mut app, &lost_souls).len(), 1);
    assert_eq!(alive_spawns(&mut app, &hobos).len(), 1);
    // A picked child pool spawns its ore, which fills max_limit 1 before its creatures
    // (PoolMgr::SpawnPool: gameobjects, then creatures, one shared counter).
    assert_eq!(alive_spawns(&mut app, &deadwood), vec![]);

    for map in [0, 1] {
        let pooled: Vec<u32> = pooled_guids_on_map(map).into_iter().collect();
        eprintln!(
            "map {map}: {} pooled creature spawns, {} alive at startup",
            pooled.len(),
            alive_spawns(&mut app, &pooled).len()
        );
    }
}

#[test]
#[ignore]
fn a_respawning_lost_soul_rerolls_its_pools_spawn_point() {
    let lost_souls = pool_creature_guids([LOST_SOUL_POOL]);
    let mut app = continents_app();
    let mut picked = HashSet::new();
    for _ in 0..40 {
        let alive = alive_spawns(&mut app, &lost_souls);
        assert_eq!(alive.len(), 1, "max_limit 1 alive: {alive:?}");
        let (guid, entity) = alive[0];
        picked.insert(guid);
        app.world_mut()
            .entity_mut(entity)
            .insert(DeathState { respawn_timer: 0.0 });
        app.update();
        assert!(
            app.world().get_entity(entity).is_err(),
            "the dead member leaves the world; its pool spawns a member anew"
        );
    }
    // Each respawn picks among every member not alive, the dead one included
    // (PoolGroup::SpawnObject); 40 picks of 3 miss one with probability 3 * (2/3)^40.
    assert_eq!(picked.len(), 3, "picked {picked:?}");
}
